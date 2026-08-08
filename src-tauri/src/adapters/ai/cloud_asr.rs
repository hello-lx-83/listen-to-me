use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::{
    adapters::{ai::qwen_client::QwenClient, audio::wav::encode_mono_pcm16},
    core::{
        models::{RecordedAudio, Transcript},
        ports::{PortFuture, SpeechRecognizer},
    },
};

pub struct CloudSpeechRecognizer {
    client: QwenClient,
    language: Option<String>,
    vocabulary: Vec<String>,
}

impl CloudSpeechRecognizer {
    pub fn new(api_key: String) -> Result<Self, String> {
        Self::new_with_cancellation(api_key, CancellationToken::new())
    }

    pub fn new_with_cancellation(
        api_key: String,
        cancellation: CancellationToken,
    ) -> Result<Self, String> {
        Ok(Self {
            client: QwenClient::with_cancellation(api_key, cancellation)?,
            language: None,
            vocabulary: Vec::new(),
        })
    }

    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            client: QwenClient::from_env()?,
            language: None,
            vocabulary: Vec::new(),
        })
    }

    pub fn configure(&mut self, language: &str, vocabulary: Vec<String>) {
        self.language = match language {
            "zh" | "en" => Some(language.to_owned()),
            _ => None,
        };
        self.vocabulary = vocabulary;
    }
}

impl SpeechRecognizer for CloudSpeechRecognizer {
    fn transcribe(&self, audio: RecordedAudio) -> PortFuture<'_, Transcript> {
        Box::pin(async move {
            let wav = encode_mono_pcm16(&audio)?;
            let data_url = format!("data:audio/wav;base64,{}", STANDARD.encode(wav));
            let mut messages = Vec::new();
            if !self.vocabulary.is_empty() {
                messages.push(json!({
                    "role": "system",
                    "content": [{
                        "type": "text",
                        "text": self.vocabulary.join("、")
                    }]
                }));
            }
            messages.push(json!({
                "role": "user",
                "content": [{
                    "type": "input_audio",
                    "input_audio": { "data": data_url }
                }]
            }));

            let mut asr_options = serde_json::Map::new();
            asr_options.insert("enable_itn".to_owned(), json!(true));
            if let Some(language) = &self.language {
                asr_options.insert("language".to_owned(), json!(language));
            }

            let payload = json!({
                "model": "qwen3-asr-flash",
                "messages": messages,
                "asr_options": asr_options,
                "stream": false
            });

            self.client
                .completion(payload)
                .await
                .map(|text| Transcript(text.trim().to_owned()))
        })
    }
}
