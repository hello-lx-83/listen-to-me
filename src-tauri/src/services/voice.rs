use crate::{
    adapters::{
        ai::{cloud_asr::CloudSpeechRecognizer, cloud_rewriter::CloudTextRewriter},
        audio::cpal_capture::CpalAudioCapture,
        secrets::credential_store::{wipe_string, CredentialStore},
    },
    core::{
        models::{DictionaryEntry, RecordedAudio, RewriteMode, RewrittenText, Transcript},
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

    pub fn cloud_models(
        &self,
        language: &str,
        dictionary: &[DictionaryEntry],
        cancellation: CancellationToken,
    ) -> Result<CloudVoiceModels, String> {
        let mut api_key = CredentialStore::qwen_api_key()?;
        let recognizer =
            CloudSpeechRecognizer::new_with_cancellation(api_key.clone(), cancellation.clone())
                .map(|mut recognizer| {
                    recognizer.configure(
                        language,
                        dictionary
                            .iter()
                            .map(|entry| entry.replacement.clone())
                            .collect::<std::collections::BTreeSet<_>>()
                            .into_iter()
                            .collect(),
                    );
                    recognizer
                });
        let rewriter = CloudTextRewriter::new_with_cancellation(
            api_key.clone(),
            "qwen3.7-flash",
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
    ) -> Result<RewrittenText, String> {
        self.rewriter.rewrite(transcript, mode).await
    }
}

pub fn apply_dictionary(transcript: &Transcript, dictionary: &[DictionaryEntry]) -> Transcript {
    let mut replacements = dictionary
        .iter()
        .flat_map(|entry| {
            dictionary_aliases(&entry.source)
                .into_iter()
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
}
