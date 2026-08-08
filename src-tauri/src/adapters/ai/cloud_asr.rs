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
    model: String,
    language: Option<String>,
    vocabulary: Vec<String>,
}

impl CloudSpeechRecognizer {
    pub fn new(api_key: String) -> Result<Self, String> {
        Self::new_with_model_and_cancellation(
            api_key,
            crate::core::models::DEFAULT_QWEN_ASR_MODEL,
            CancellationToken::new(),
        )
    }

    pub fn new_with_cancellation(
        api_key: String,
        cancellation: CancellationToken,
    ) -> Result<Self, String> {
        Self::new_with_model_and_cancellation(
            api_key,
            crate::core::models::DEFAULT_QWEN_ASR_MODEL,
            cancellation,
        )
    }

    pub fn new_with_model_and_cancellation(
        api_key: String,
        model: impl Into<String>,
        cancellation: CancellationToken,
    ) -> Result<Self, String> {
        Ok(Self {
            client: QwenClient::with_cancellation(api_key, cancellation)?,
            model: model.into(),
            language: None,
            vocabulary: Vec::new(),
        })
    }

    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            client: QwenClient::from_env()?,
            model: crate::core::models::DEFAULT_QWEN_ASR_MODEL.to_owned(),
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
            let payload = asr_payload(
                &self.model,
                data_url,
                self.language.as_deref(),
                &self.vocabulary,
            );
            let result = if is_fun_asr_model(&self.model) {
                self.client.fun_asr_completion(payload).await
            } else {
                self.client.completion(payload).await
            };
            result.map(|text| Transcript(text.trim().to_owned()))
        })
    }
}

fn is_fun_asr_model(model: &str) -> bool {
    model == "fun-asr-flash-2026-06-15"
}

fn asr_payload(
    model: &str,
    data_url: String,
    language: Option<&str>,
    vocabulary: &[String],
) -> serde_json::Value {
    if is_fun_asr_model(model) {
        return json!({
            "model": model,
            "input": {
                "messages": [{
                    "role": "user",
                    "content": [{
                        "type": "input_audio",
                        "input_audio": { "data": data_url }
                    }]
                }]
            },
            "parameters": { "format": "wav", "sample_rate": "16000" }
        });
    }

    let mut messages = Vec::new();
    if !vocabulary.is_empty() {
        messages.push(json!({
            "role": "system",
            "content": [{
                "type": "text",
                "text": vocabulary.join("、")
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
    if let Some(language) = language {
        asr_options.insert("language".to_owned(), json!(language));
    }

    json!({
        "model": model,
        "messages": messages,
        "asr_options": asr_options,
        "stream": false
    })
}

pub async fn test_qwen_asr_connection(api_key: String, model: String) -> Result<(), String> {
    let wav = encode_mono_pcm16(&RecordedAudio {
        samples: vec![0; 3_200],
        sample_rate: 16_000,
    })?;
    let data_url = format!("data:audio/wav;base64,{}", STANDARD.encode(wav));
    let payload = asr_payload(&model, data_url, None, &[]);
    let client = QwenClient::new(api_key)?;
    if is_fun_asr_model(&model) {
        client.validate_fun_asr_request(payload).await
    } else {
        client.validate_request(payload).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fun_asr_uses_dashscope_audio_payload() {
        let payload = asr_payload(
            "fun-asr-flash-2026-06-15",
            "data:audio/wav;base64,AA==".to_owned(),
            Some("zh"),
            &["Codex".to_owned()],
        );
        assert_eq!(payload["parameters"]["format"], "wav");
        assert_eq!(payload["parameters"]["sample_rate"], "16000");
        assert_eq!(
            payload["input"]["messages"][0]["content"][0]["type"],
            "input_audio"
        );
    }

    #[test]
    fn dedicated_asr_uses_non_streaming_asr_options() {
        let payload = asr_payload(
            "qwen3-asr-flash",
            "data:audio/wav;base64,AA==".to_owned(),
            Some("en"),
            &[],
        );
        assert_eq!(payload["stream"], false);
        assert_eq!(payload["asr_options"]["language"], "en");
    }
}
