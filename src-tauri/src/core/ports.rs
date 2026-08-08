use std::{future::Future, pin::Pin};

use crate::core::models::{RecordedAudio, RewriteMode, RewrittenText, Transcript};

pub type PortResult<T> = Result<T, String>;
pub type PortFuture<'a, T> = Pin<Box<dyn Future<Output = PortResult<T>> + Send + 'a>>;

pub trait AudioCapture: Send + Sync {
    fn start(&self) -> PortFuture<'_, ()>;
    fn stop(&self) -> PortFuture<'_, RecordedAudio>;
    fn cancel(&self) -> PortFuture<'_, ()>;
}

pub trait SpeechRecognizer: Send + Sync {
    fn transcribe(&self, audio: RecordedAudio) -> PortFuture<'_, Transcript>;
}

pub trait TextRewriter: Send + Sync {
    fn rewrite(&self, transcript: Transcript, mode: RewriteMode) -> PortFuture<'_, RewrittenText>;
}

pub trait TextInjector: Send + Sync {
    fn insert(&self, text: RewrittenText) -> PortFuture<'_, ()>;
}

pub trait HotkeyListener: Send + Sync {
    fn start(&self) -> PortFuture<'_, ()>;
}
