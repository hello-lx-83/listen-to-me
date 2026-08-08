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
    let mut entries = dictionary.iter().collect::<Vec<_>>();
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.source.chars().count()));
    let corrected = entries.iter().fold(transcript.0.clone(), |text, entry| {
        text.replace(&entry.source, &entry.replacement)
    });
    Transcript(corrected)
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
}
