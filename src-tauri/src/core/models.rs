use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VoiceSessionState {
    #[default]
    Idle,
    Arming,
    Recording,
    Transcribing,
    Rewriting,
    Injecting,
    Failed,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelRoute {
    #[default]
    Auto,
    Cloud,
    Local,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RewriteMode {
    Raw,
    #[default]
    Clean,
    Article,
    Structured,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub voice_session: VoiceSessionState,
    pub default_shortcut: String,
    pub model_route: ModelRoute,
}

#[derive(Clone, Debug)]
pub struct RecordedAudio {
    pub samples: Vec<i16>,
    pub sample_rate: u32,
}

#[derive(Clone, Debug)]
pub struct Transcript(pub String);

#[derive(Clone, Debug)]
pub struct RewrittenText(pub String);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: String,
    pub language: String,
    pub rewrite_mode: RewriteMode,
    pub save_history: bool,
    pub history_retention_days: u32,
}

pub const DEFAULT_QWEN_ASR_MODEL: &str = "qwen3-asr-flash";
pub const DEFAULT_QWEN_REWRITE_MODEL: &str = "qwen3.7-flash";

pub fn is_supported_qwen_asr_model(model: &str) -> bool {
    matches!(model, "qwen3-asr-flash" | "fun-asr-flash-2026-06-15")
}

pub fn is_supported_qwen_rewrite_model(model: &str) -> bool {
    matches!(
        model,
        "qwen3.7-flash"
            | "qwen3.7-plus"
            | "qwen3.7-max"
            | "qwen3.6-flash"
            | "qwen3.6-plus"
            | "qwen3.5-flash"
            | "qwen3.5-plus"
    )
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QwenModelSettings {
    pub asr_model: String,
    pub rewrite_model: String,
}

impl Default for QwenModelSettings {
    fn default() -> Self {
        Self {
            asr_model: DEFAULT_QWEN_ASR_MODEL.to_owned(),
            rewrite_model: DEFAULT_QWEN_REWRITE_MODEL.to_owned(),
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "system".to_owned(),
            language: "auto".to_owned(),
            rewrite_mode: RewriteMode::Clean,
            save_history: true,
            history_retention_days: 30,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    pub id: i64,
    pub created_at: i64,
    pub mode: RewriteMode,
    pub transcript: String,
    pub output: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryEntry {
    pub id: i64,
    pub source: String,
    pub replacement: String,
    pub category: String,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryEntryInput {
    pub id: Option<i64>,
    pub source: String,
    pub replacement: String,
    pub category: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryCategory {
    pub name: String,
    pub entry_count: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardOverview {
    pub qwen_configured: bool,
    pub history_count: i64,
    pub dictionary_count: i64,
}
