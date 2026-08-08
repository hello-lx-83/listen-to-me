use std::sync::Mutex;

use crate::{
    adapters::persistence::sqlite::SqliteStore,
    core::{
        models::{AppSnapshot, VoiceSessionState},
        voice_session::VoiceSession,
    },
};

pub struct AppState {
    voice_session: Mutex<VoiceSession>,
    store: SqliteStore,
}

impl AppState {
    pub fn new(store: SqliteStore) -> Self {
        Self {
            voice_session: Mutex::new(VoiceSession::default()),
            store,
        }
    }

    pub fn store(&self) -> &SqliteStore {
        &self.store
    }

    pub fn set_voice_state(&self, next: VoiceSessionState) {
        let mut session = self
            .voice_session
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        session.transition_to(next);
    }

    pub fn snapshot(&self) -> AppSnapshot {
        let session = self
            .voice_session
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        AppSnapshot {
            voice_session: session.state(),
            default_shortcut: "Right Alt".to_owned(),
            model_route: crate::core::models::ModelRoute::Cloud,
        }
    }
}
