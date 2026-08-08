use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};
use tokio_util::sync::CancellationToken;
use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, DeleteObject, SetWindowRgn};

use crate::{
    app_state::AppState,
    core::{
        models::{RewriteMode, RewrittenText, VoiceSessionState},
        voice_session::{HoldAction, HoldController},
    },
    platform::windows::{
        keyboard_hook::{self, RightAltEvent},
        text_injector::paste_unicode_text,
    },
    services::voice::{apply_dictionary, VoiceService},
};

const HOLD_THRESHOLD: Duration = Duration::from_millis(220);
const IDLE_WAIT: Duration = Duration::from_secs(60);
const OVERLAY_LABEL: &str = "voice-overlay";
const STATE_EVENT: &str = "voice://state-changed";
const ERROR_EVENT: &str = "voice://error";
const LEVEL_EVENT: &str = "voice://input-level";
const METRIC_EVENT: &str = "voice://stage-metric";
const MODE_EVENT: &str = "voice://mode-changed";
const LEVEL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StageMetric {
    stage: &'static str,
    elapsed_ms: u64,
}

pub fn start(app: AppHandle) -> Result<(), String> {
    let (sender, receiver) = mpsc::channel();
    let _hook_thread = keyboard_hook::start(sender)?;

    thread::Builder::new()
        .name("voice-runtime".to_owned())
        .spawn(move || run(app, receiver))
        .map_err(|error| format!("failed to start voice runtime: {error}"))?;

    Ok(())
}

fn run(app: AppHandle, receiver: Receiver<RightAltEvent>) {
    let clock = Instant::now();
    let mut hold = HoldController::new(HOLD_THRESHOLD);
    let voice = Arc::new(VoiceService::default());
    let processing = Arc::new(AtomicBool::new(false));
    let cancellation = Arc::new(Mutex::new(None::<CancellationToken>));
    let mut capturing = false;

    loop {
        let now = clock.elapsed();
        let wait = hold.time_until_activation(now).unwrap_or(IDLE_WAIT);
        let wait = if capturing {
            wait.min(LEVEL_INTERVAL)
        } else {
            wait
        };

        match receiver.recv_timeout(wait) {
            Ok(RightAltEvent::Pressed) => {
                if !processing.load(Ordering::Acquire) && !hold.is_pending() {
                    hold.press(clock.elapsed());
                    set_state(&app, VoiceSessionState::Arming);
                }
            }
            Ok(RightAltEvent::Released) => match hold.release() {
                Some(HoldAction::Finish) if capturing => {
                    capturing = false;
                    finish_session(
                        &app,
                        voice.clone(),
                        processing.clone(),
                        cancellation.clone(),
                    );
                }
                Some(HoldAction::Tap) if !processing.load(Ordering::Acquire) => {
                    cycle_rewrite_mode(&app);
                }
                _ if !processing.load(Ordering::Acquire) => reset(&app),
                _ => {}
            },
            Ok(RightAltEvent::Cancel) => {
                if processing.load(Ordering::Acquire) {
                    if let Ok(active) = cancellation.lock() {
                        if let Some(token) = active.as_ref() {
                            token.cancel();
                        }
                    }
                    reset(&app);
                } else if hold.cancel().is_some() {
                    if capturing {
                        let _ = tauri::async_runtime::block_on(voice.cancel_recording());
                        capturing = false;
                    }
                    if !processing.load(Ordering::Acquire) {
                        reset(&app);
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if capturing {
                    let _ = app.emit_to(OVERLAY_LABEL, LEVEL_EVENT, voice.input_level());
                }
                if matches!(hold.poll(clock.elapsed()), Some(HoldAction::Begin)) {
                    capturing = begin(&app, &voice);
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn cycle_rewrite_mode(app: &AppHandle) {
    let store = app.state::<AppState>();
    let mut settings = match store.store().settings() {
        Ok(settings) => settings,
        Err(error) => {
            eprintln!("[voice-runtime] failed to read mode: {error}");
            reset(app);
            return;
        }
    };
    settings.rewrite_mode = match settings.rewrite_mode {
        RewriteMode::Clean => RewriteMode::Raw,
        RewriteMode::Raw => RewriteMode::Clean,
        RewriteMode::Structured | RewriteMode::Article => RewriteMode::Clean,
    };
    if let Err(error) = store.store().update_settings(&settings) {
        eprintln!("[voice-runtime] failed to change mode: {error}");
        reset(app);
        return;
    }

    set_state(app, VoiceSessionState::Idle);
    if let Some(overlay) = app.get_webview_window(OVERLAY_LABEL) {
        let _ = position_overlay(app, &overlay);
        let _ = apply_rounded_window_region(&overlay);
        let _ = app.emit_to(OVERLAY_LABEL, MODE_EVENT, settings.rewrite_mode);
        let _ = overlay.show();
    }
}

fn begin(app: &AppHandle, voice: &VoiceService) -> bool {
    set_state(app, VoiceSessionState::Recording);
    if let Some(overlay) = app.get_webview_window(OVERLAY_LABEL) {
        let _ = overlay.set_focusable(false);
        let _ = position_overlay(app, &overlay);
        let _ = apply_rounded_window_region(&overlay);
        let _ = overlay.show();
    }

    match tauri::async_runtime::block_on(voice.start_recording()) {
        Ok(()) => true,
        Err(_) => {
            fail(app, "无法启动麦克风，请检查默认输入设备和系统权限。");
            false
        }
    }
}

fn apply_rounded_window_region(overlay: &WebviewWindow) -> Result<(), String> {
    let hwnd = overlay
        .hwnd()
        .map_err(|error| format!("failed to get overlay window handle: {error}"))?;
    // The undecorated Win32 window can still have a small non-client frame.
    // Clip to the WebView's client size so that frame cannot show around the
    // rounded overlay on the right and bottom edges.
    let size = overlay
        .inner_size()
        .map_err(|error| format!("failed to read overlay content size: {error}"))?;
    let scale = overlay
        .scale_factor()
        .map_err(|error| format!("failed to read overlay scale factor: {error}"))?;
    let corner_diameter = (36.0 * scale).round() as i32;

    unsafe {
        let region = CreateRoundRectRgn(
            0,
            0,
            size.width as i32 + 1,
            size.height as i32 + 1,
            corner_diameter,
            corner_diameter,
        );
        if region.is_invalid() {
            return Err("failed to create rounded overlay window region".to_owned());
        }
        if SetWindowRgn(hwnd, Some(region), true) == 0 {
            let _ = DeleteObject(region.into());
            return Err("failed to apply rounded overlay window region".to_owned());
        }
    }

    Ok(())
}

fn position_overlay(app: &AppHandle, overlay: &WebviewWindow) -> tauri::Result<()> {
    let cursor = app.cursor_position()?;
    let monitor = app
        .monitor_from_point(cursor.x, cursor.y)?
        .or(app.primary_monitor()?);
    let Some(monitor) = monitor else {
        return overlay.center();
    };

    let work_area = monitor.work_area();
    let window_size = overlay.outer_size()?;
    let bottom_margin = (40.0 * monitor.scale_factor()).round() as u32;
    let x_offset = work_area.size.width.saturating_sub(window_size.width) / 2;
    let y_offset = work_area
        .size
        .height
        .saturating_sub(window_size.height)
        .saturating_sub(bottom_margin);

    overlay.set_position(PhysicalPosition::new(
        work_area.position.x + x_offset as i32,
        work_area.position.y + y_offset as i32,
    ))
}

fn finish_session(
    app: &AppHandle,
    voice: Arc<VoiceService>,
    processing: Arc<AtomicBool>,
    cancellation: Arc<Mutex<Option<CancellationToken>>>,
) {
    let audio = match tauri::async_runtime::block_on(voice.stop_recording()) {
        Ok(audio) if audio.samples.len() >= audio.sample_rate as usize / 10 => audio,
        Ok(_) | Err(_) => {
            fail(app, "没有录到声音，请检查麦克风后重试。");
            return;
        }
    };

    emit_metric(
        app,
        "recording",
        ((audio.samples.len() as u64) * 1_000) / audio.sample_rate as u64,
    );

    processing.store(true, Ordering::Release);
    let token = CancellationToken::new();
    if let Ok(mut active) = cancellation.lock() {
        *active = Some(token.clone());
    }
    set_state(app, VoiceSessionState::Transcribing);
    let worker_app = app.clone();
    let worker_processing = processing.clone();
    let worker_cancellation = cancellation.clone();
    let worker_token = token.clone();

    let spawn_result = thread::Builder::new()
        .name("voice-cloud-pipeline".to_owned())
        .spawn(move || {
            let result = tauri::async_runtime::block_on(async {
                let settings = worker_app.state::<AppState>().store().settings()?;
                let dictionary = worker_app.state::<AppState>().store().list_dictionary()?;
                let models =
                    voice.cloud_models(&settings.language, &dictionary, worker_token.clone())?;
                let transcribing_started = Instant::now();
                let transcript = models.transcribe(audio).await?;
                emit_metric(
                    &worker_app,
                    "transcribing",
                    elapsed_millis(transcribing_started),
                );
                ensure_not_cancelled(&worker_token)?;
                let original_transcript = transcript.0.clone();
                set_state(&worker_app, VoiceSessionState::Rewriting);
                let corrected = apply_dictionary(&transcript, &dictionary);
                let rewriting_started = Instant::now();
                let rewritten = match models.rewrite(corrected.clone(), settings.rewrite_mode).await {
                    Ok(rewritten) => rewritten,
                    Err(error) => {
                        eprintln!("[voice-runtime] text rewrite failed, using corrected transcript: {error}");
                        RewrittenText(corrected.0)
                    }
                };
                let rewritten = apply_dictionary(&crate::core::models::Transcript(rewritten.0), &dictionary);
                let rewritten = RewrittenText(rewritten.0);
                emit_metric(&worker_app, "rewriting", elapsed_millis(rewriting_started));
                ensure_not_cancelled(&worker_token)?;
                if settings.save_history {
                    worker_app.state::<AppState>().store().add_history(
                        settings.rewrite_mode,
                        &original_transcript,
                        &rewritten.0,
                    )?;
                }
                ensure_not_cancelled(&worker_token)?;
                set_state(&worker_app, VoiceSessionState::Injecting);
                let owner = worker_app
                    .get_webview_window(OVERLAY_LABEL)
                    .ok_or_else(|| "voice overlay window is unavailable".to_owned())?
                    .hwnd()
                    .map_err(|error| format!("failed to get clipboard owner window: {error}"))?;
                paste_unicode_text(owner, &rewritten.0)
            });

            worker_processing.store(false, Ordering::Release);
            if let Ok(mut active) = worker_cancellation.lock() {
                *active = None;
            }
            match result {
                Ok(()) => reset(&worker_app),
                Err(_) if worker_token.is_cancelled() => reset(&worker_app),
                Err(error) => {
                    eprintln!("[voice-runtime] voice pipeline failed: {error}");
                    fail(&worker_app, friendly_error(&error));
                }
            }
        });

    if spawn_result.is_err() {
        processing.store(false, Ordering::Release);
        if let Ok(mut active) = cancellation.lock() {
            *active = None;
        }
        fail(app, "无法启动语音处理线程，请重试。");
    }
}

fn ensure_not_cancelled(cancellation: &CancellationToken) -> Result<(), String> {
    if cancellation.is_cancelled() {
        Err("voice session was cancelled".to_owned())
    } else {
        Ok(())
    }
}

fn elapsed_millis(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u64::MAX as u128) as u64
}

fn emit_metric(app: &AppHandle, stage: &'static str, elapsed_ms: u64) {
    let _ = app.emit_to(
        OVERLAY_LABEL,
        METRIC_EVENT,
        StageMetric { stage, elapsed_ms },
    );
}

fn fail(app: &AppHandle, message: &str) {
    set_state(app, VoiceSessionState::Failed);
    let _ = app.emit_to(OVERLAY_LABEL, ERROR_EVENT, message);

    let failed_app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(8)).await;
        if matches!(
            failed_app.state::<AppState>().snapshot().voice_session,
            VoiceSessionState::Failed
        ) {
            reset(&failed_app);
        }
    });
}

fn friendly_error(error: &str) -> &'static str {
    if error.contains("not configured") || error.contains("authentication failed") {
        "模型凭据无效，请在客户端设置中重新配置。"
    } else if error.contains("quota") || error.contains("rate limit") {
        "模型请求额度或频率受限，请稍后重试。"
    } else if error.contains("network request failed") {
        "无法连接千问服务，请检查网络后重试。"
    } else if error.contains("HTTP status 400") {
        "语音识别请求格式无效，请更新客户端后重试。"
    } else if error.contains("HTTP status 404") {
        "当前语音识别模型不可用，请检查模型配置。"
    } else if error.contains("HTTP status 5") || error.contains("failed after retry") {
        "千问服务暂时不可用，请稍后重试。"
    } else if error.contains("empty response") || error.contains("could not be decoded") {
        "千问服务返回异常，请稍后重试。"
    } else if error.contains("WAV") || error.contains("audio") {
        "录音数据处理失败，请检查麦克风后重试。"
    } else if error.contains("history") || error.contains("database") {
        "本地数据保存失败，请打开客户端检查设置。"
    } else {
        "语音处理失败，请稍后重试。"
    }
}

fn reset(app: &AppHandle) {
    if let Some(overlay) = app.get_webview_window(OVERLAY_LABEL) {
        let _ = overlay.hide();
    }
    set_state(app, VoiceSessionState::Idle);
}

fn set_state(app: &AppHandle, next: VoiceSessionState) {
    app.state::<AppState>().set_voice_state(next);
    let _ = app.emit_to(OVERLAY_LABEL, STATE_EVENT, next);
}
