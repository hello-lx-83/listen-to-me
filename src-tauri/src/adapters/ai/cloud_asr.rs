use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Map, Value};
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

const MAX_VOCABULARY_TERMS: usize = 50;
const MAX_VOCABULARY_TERM_CHARS: usize = 60;
const MAX_CONTEXT_CHARS: usize = 400;

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
            let payload = asr_payload(data_url, self.language.as_deref(), &self.vocabulary);
            self.client
                .asr_completion(payload)
                .await
                .map(|text| Transcript(text.trim().to_owned()))
        })
    }
}

fn asr_payload(data_url: String, language: Option<&str>, vocabulary: &[String]) -> Value {
    let vocabulary = normalized_vocabulary(vocabulary);
    let mut messages = vec![json!({
        "role": "user",
        "content": [{
            "type": "input_text",
            "text": recognition_context(&vocabulary)
        }]
    })];
    messages.push(json!({
        "role": "user",
        "content": [{
            "type": "input_audio",
            "input_audio": { "data": data_url }
        }]
    }));

    let mut parameters = Map::new();
    parameters.insert("format".to_owned(), json!("wav"));
    parameters.insert("sample_rate".to_owned(), json!("16000"));
    if let Some(language) = language {
        parameters.insert("language_hints".to_owned(), json!([language]));
    }
    if !vocabulary.is_empty() {
        parameters.insert(
            "vocabulary".to_owned(),
            Value::Object(
                vocabulary
                    .iter()
                    .map(|term| (term.clone(), json!(5)))
                    .collect(),
            ),
        );
    }

    json!({
        "model": crate::core::models::DEFAULT_QWEN_ASR_MODEL,
        "input": { "messages": messages },
        "parameters": parameters
    })
}

fn normalized_vocabulary(vocabulary: &[String]) -> Vec<String> {
    vocabulary
        .iter()
        .map(|term| term.trim())
        .filter(|term| !term.is_empty() && term.chars().count() <= MAX_VOCABULARY_TERM_CHARS)
        .take(MAX_VOCABULARY_TERMS)
        .map(str::to_owned)
        .collect()
}

fn recognition_context(vocabulary: &[String]) -> String {
    let base = "只转写实际语音，准确保留数字、时间、版本号、百分比和中英文专业术语。";
    let prefix = "领域词表：";
    let mut context = base.to_owned();
    if vocabulary.is_empty() {
        return context;
    }
    context.push_str(prefix);
    for term in vocabulary {
        let separator = if context.ends_with(prefix) { "" } else { "、" };
        if context.chars().count() + separator.chars().count() + term.chars().count()
            > MAX_CONTEXT_CHARS
        {
            break;
        }
        context.push_str(separator);
        context.push_str(term);
    }
    context
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qwen_audio_uses_dashscope_audio_payload() {
        let payload = asr_payload(
            "data:audio/wav;base64,AA==".to_owned(),
            Some("zh"),
            &["Codex".to_owned()],
        );
        assert_eq!(payload["model"], "qwen-audio-3.0-asr-flash");
        assert_eq!(payload["parameters"]["format"], "wav");
        assert_eq!(payload["parameters"]["sample_rate"], "16000");
        assert_eq!(payload["parameters"]["language_hints"], json!(["zh"]));
        assert_eq!(payload["parameters"]["vocabulary"]["Codex"], 5);
        assert_eq!(
            payload["input"]["messages"][1]["content"][0]["type"],
            "input_audio"
        );
    }

    #[test]
    fn vocabulary_is_sent_as_bounded_context_before_audio() {
        let payload = asr_payload(
            "data:audio/wav;base64,AA==".to_owned(),
            None,
            &(0..60)
                .map(|index| format!("term-{index}"))
                .collect::<Vec<_>>(),
        );
        assert_eq!(
            payload["input"]["messages"][0]["content"][0]["type"],
            "input_text"
        );
        assert!(
            payload["input"]["messages"][0]["content"][0]["text"]
                .as_str()
                .expect("context text")
                .chars()
                .count()
                <= MAX_CONTEXT_CHARS
        );
        assert_eq!(
            payload["parameters"]["vocabulary"]
                .as_object()
                .expect("vocabulary object")
                .len(),
            MAX_VOCABULARY_TERMS
        );
    }

    #[test]
    fn recognition_context_always_protects_numeric_facts() {
        let payload = asr_payload("data:audio/wav;base64,AA==".to_owned(), None, &[]);
        assert!(payload["input"]["messages"][0]["content"][0]["text"]
            .as_str()
            .expect("context text")
            .contains("百分比"));
        assert_eq!(
            payload["input"]["messages"][1]["content"][0]["type"],
            "input_audio"
        );
    }
}
