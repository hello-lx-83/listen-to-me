use crate::{
    adapters::{
        ai::{cloud_asr::CloudSpeechRecognizer, cloud_rewriter::CloudTextRewriter},
        audio::cpal_capture::CpalAudioCapture,
        secrets::credential_store::{wipe_string, CredentialStore},
    },
    core::{
        models::{
            DictionaryEntry, QwenModelSettings, RecordedAudio, RewriteMode, RewrittenText,
            Transcript,
        },
        ports::{AudioCapture, SpeechRecognizer, TextRewriter},
    },
};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub struct VoiceService {
    capture: CpalAudioCapture,
}

impl VoiceService {
    pub fn input_level(&self) -> f32 {
        self.capture.input_level()
    }

    pub async fn start_recording(&self) -> Result<(), String> {
        self.capture.start().await
    }

    pub async fn stop_recording(&self) -> Result<RecordedAudio, String> {
        self.capture.stop().await
    }

    pub async fn cancel_recording(&self) -> Result<(), String> {
        self.capture.cancel().await
    }

    pub async fn test_cloud_asr(api_key: String) -> Result<String, String> {
        let capture = CpalAudioCapture::default();
        capture.start().await?;
        tokio::time::sleep(std::time::Duration::from_secs(4)).await;
        let audio = capture.stop().await?;
        if audio.samples.len() < audio.sample_rate as usize / 2 {
            return Err("ASR microphone test did not capture enough audio".to_owned());
        }
        let recognizer = CloudSpeechRecognizer::new(api_key)?;
        recognizer
            .transcribe(audio)
            .await
            .map(|transcript| transcript.0)
    }

    pub fn cloud_models(
        &self,
        language: &str,
        dictionary: &[DictionaryEntry],
        model_settings: &QwenModelSettings,
        cancellation: CancellationToken,
    ) -> Result<CloudVoiceModels, String> {
        let mut api_key = CredentialStore::qwen_api_key()?;
        let recognizer =
            CloudSpeechRecognizer::new_with_cancellation(api_key.clone(), cancellation.clone())
                .map(|mut recognizer| {
                    recognizer.configure(language, dictionary_vocabulary(dictionary));
                    recognizer
                });
        let rewriter = CloudTextRewriter::new_with_cancellation(
            api_key.clone(),
            model_settings.rewrite_model.clone(),
            cancellation,
        );
        wipe_string(&mut api_key);

        Ok(CloudVoiceModels {
            recognizer: recognizer?,
            rewriter: rewriter?,
        })
    }
}

pub struct CloudVoiceModels {
    recognizer: CloudSpeechRecognizer,
    rewriter: CloudTextRewriter,
}

impl CloudVoiceModels {
    pub async fn transcribe(&self, audio: RecordedAudio) -> Result<Transcript, String> {
        self.recognizer.transcribe(audio).await
    }

    pub async fn rewrite(
        &self,
        transcript: Transcript,
        mode: RewriteMode,
        protected_terms: Vec<String>,
    ) -> Result<RewrittenText, String> {
        self.rewriter
            .rewrite(transcript, mode, protected_terms)
            .await
    }
}

pub fn dictionary_terms_in_transcript(
    transcript: &Transcript,
    dictionary: &[DictionaryEntry],
) -> Vec<String> {
    dictionary
        .iter()
        .map(|entry| entry.replacement.trim())
        .filter(|term| !term.is_empty() && contains_term(&transcript.0, term))
        .map(str::to_owned)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn dictionary_vocabulary(dictionary: &[DictionaryEntry]) -> Vec<String> {
    let replacements = dictionary
        .iter()
        .map(|entry| entry.replacement.trim())
        .filter(|term| !term.is_empty())
        .map(str::to_owned)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let replacement_set = replacements
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let aliases = dictionary
        .iter()
        .flat_map(|entry| dictionary_aliases(&entry.source))
        .map(str::to_owned)
        .filter(|alias| !replacement_set.contains(alias))
        .collect::<std::collections::BTreeSet<_>>();
    replacements.into_iter().chain(aliases).collect()
}

pub fn apply_dictionary(transcript: &Transcript, dictionary: &[DictionaryEntry]) -> Transcript {
    let mut replacements = dictionary
        .iter()
        .flat_map(|entry| {
            std::iter::once(entry.replacement.as_str())
                .chain(dictionary_aliases(&entry.source))
                .map(move |alias| (alias, entry.replacement.as_str()))
        })
        .collect::<Vec<_>>();
    replacements.sort_by_key(|(alias, _)| std::cmp::Reverse(alias.chars().count()));
    let corrected =
        replacements
            .into_iter()
            .fold(transcript.0.clone(), |text, (alias, replacement)| {
                if alias.is_ascii() {
                    replace_ascii_case_insensitive(&text, alias, replacement)
                } else {
                    text.replace(alias, replacement)
                }
            });
    Transcript(corrected)
}

fn dictionary_aliases(source: &str) -> Vec<&str> {
    source
        .split([',', '，', '、', '\n', ';', '；'])
        .map(str::trim)
        .filter(|alias| !alias.is_empty())
        .collect()
}

fn replace_ascii_case_insensitive(text: &str, source: &str, replacement: &str) -> String {
    let needle = source.to_ascii_lowercase();
    let mut output = text.to_owned();
    let mut cursor = 0;

    while let Some(offset) = output[cursor..].to_ascii_lowercase().find(&needle) {
        let start = cursor + offset;
        let end = start + source.len();
        let starts_inside_word = output[..start]
            .chars()
            .next_back()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_');
        let ends_inside_word = output[end..]
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_');
        if starts_inside_word || ends_inside_word {
            cursor = end;
            continue;
        }
        output.replace_range(start..end, replacement);
        cursor = start + replacement.len();
    }

    output
}

fn contains_term(text: &str, term: &str) -> bool {
    if term.is_ascii() {
        text.to_ascii_lowercase()
            .contains(&term.to_ascii_lowercase())
    } else {
        text.contains(term)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictionary_replaces_longer_terms_first() {
        let entries = vec![
            DictionaryEntry {
                id: 1,
                source: "千问".to_owned(),
                replacement: "Qwen".to_owned(),
                category: "产品".to_owned(),
                updated_at: 0,
            },
            DictionaryEntry {
                id: 2,
                source: "千问语音".to_owned(),
                replacement: "Qwen-ASR".to_owned(),
                category: "产品".to_owned(),
                updated_at: 0,
            },
        ];
        assert_eq!(
            apply_dictionary(&Transcript("使用千问语音".to_owned()), &entries).0,
            "使用Qwen-ASR"
        );
    }

    #[test]
    fn dictionary_accepts_multiple_spoken_forms_and_normalizes_ascii_case() {
        let entries = vec![DictionaryEntry {
            id: 1,
            source: "智能体、诶真特, agent".to_owned(),
            replacement: "Agent".to_owned(),
            category: "AI 术语".to_owned(),
            updated_at: 0,
        }];

        assert_eq!(
            apply_dictionary(
                &Transcript("让智能体和 AGENT 一起工作".to_owned()),
                &entries
            )
            .0,
            "让Agent和 Agent 一起工作"
        );
    }

    #[test]
    fn ascii_dictionary_terms_do_not_replace_inside_other_words() {
        let entries = vec![DictionaryEntry {
            id: 1,
            source: "agent".to_owned(),
            replacement: "Agent".to_owned(),
            category: "AI 术语".to_owned(),
            updated_at: 0,
        }];

        assert_eq!(
            apply_dictionary(&Transcript("magenta uses agent".to_owned()), &entries).0,
            "magenta uses Agent"
        );
    }

    #[test]
    fn dictionary_standardizes_ascii_replacement_casing() {
        let entries = vec![DictionaryEntry {
            id: 1,
            source: "扣得克斯".to_owned(),
            replacement: "Codex".to_owned(),
            category: "产品".to_owned(),
            updated_at: 0,
        }];
        assert_eq!(
            apply_dictionary(&Transcript("使用 codex".to_owned()), &entries).0,
            "使用 Codex"
        );
    }

    #[test]
    fn dictionary_terms_present_after_correction_are_protected_for_rewrite() {
        let entries = vec![
            DictionaryEntry {
                id: 1,
                source: "扣得克斯".to_owned(),
                replacement: "Codex".to_owned(),
                category: "产品".to_owned(),
                updated_at: 0,
            },
            DictionaryEntry {
                id: 2,
                source: "大模型".to_owned(),
                replacement: "大语言模型".to_owned(),
                category: "AI 术语".to_owned(),
                updated_at: 0,
            },
        ];
        let transcript =
            apply_dictionary(&Transcript("用扣得克斯检查这个项目".to_owned()), &entries);

        assert_eq!(
            dictionary_terms_in_transcript(&transcript, &entries),
            vec!["Codex"]
        );
    }

    #[test]
    fn vocabulary_can_include_standard_terms_and_spoken_aliases() {
        let entry = DictionaryEntry {
            id: 1,
            source: "千问 ASR、Qwen ASR".to_owned(),
            replacement: "Qwen-ASR".to_owned(),
            category: "产品".to_owned(),
            updated_at: 0,
        };
        let vocabulary = dictionary_vocabulary(&[entry]);
        assert_eq!(vocabulary, vec!["Qwen-ASR", "Qwen ASR", "千问 ASR"]);
    }
}
