//! Low-level Windows keyboard hook for the modifier-only Right Alt shortcut.

use std::{
    sync::{
        mpsc::{self, Sender},
        OnceLock,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use windows::Win32::{
    Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Input::KeyboardAndMouse::{VK_ESCAPE, VK_RMENU},
        WindowsAndMessaging::{
            CallNextHookEx, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, KBDLLHOOKSTRUCT,
            MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
        },
    },
};

static EVENT_SENDER: OnceLock<Sender<RightAltEvent>> = OnceLock::new();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RightAltEvent {
    Pressed,
    Released,
    Cancel,
}

pub fn start(sender: Sender<RightAltEvent>) -> Result<JoinHandle<Result<(), String>>, String> {
    EVENT_SENDER
        .set(sender)
        .map_err(|_| "right Alt keyboard hook has already started".to_owned())?;

    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let handle = thread::Builder::new()
        .name("right-alt-hook".to_owned())
        .spawn(move || run_message_loop(ready_sender))
        .map_err(|error| format!("failed to start keyboard hook thread: {error}"))?;

    match ready_receiver.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(())) => Ok(handle),
        Ok(Err(error)) => Err(error),
        Err(error) => Err(format!("keyboard hook did not become ready: {error}")),
    }
}

fn run_message_loop(ready: mpsc::SyncSender<Result<(), String>>) -> Result<(), String> {
    // SAFETY: The hook callback is a process-static function, and this dedicated
    // thread owns the hook and pumps its message queue until process shutdown.
    unsafe {
        let hook = (|| {
            let module = GetModuleHandleW(None)
                .map_err(|error| format!("failed to get module handle: {error}"))?;
            SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_hook),
                Some(HINSTANCE(module.0)),
                0,
            )
            .map_err(|error| format!("failed to install keyboard hook: {error}"))
        })();

        let hook = match hook {
            Ok(hook) => {
                let _ = ready.send(Ok(()));
                hook
            }
            Err(error) => {
                let _ = ready.send(Err(error.clone()));
                return Err(error);
            }
        };

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {}

        UnhookWindowsHookEx(hook)
            .map_err(|error| format!("failed to remove keyboard hook: {error}"))?;
    }

    Ok(())
}

unsafe extern "system" fn keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        // SAFETY: Windows guarantees lparam points to KBDLLHOOKSTRUCT for a
        // WH_KEYBOARD_LL callback while this function is executing.
        let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let message = wparam.0 as u32;

        if event.vkCode == u32::from(VK_RMENU.0) {
            let shortcut_event = match message {
                WM_KEYDOWN | WM_SYSKEYDOWN => Some(RightAltEvent::Pressed),
                WM_KEYUP | WM_SYSKEYUP => Some(RightAltEvent::Released),
                _ => None,
            };

            if let Some(shortcut_event) = shortcut_event {
                if let Some(sender) = EVENT_SENDER.get() {
                    let _ = sender.send(shortcut_event);
                }

                // Right Alt is reserved by the app while it is running. This
                // prevents the foreground application's menu from activating.
                return LRESULT(1);
            }
        }

        if event.vkCode == u32::from(VK_ESCAPE.0) && matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN) {
            if let Some(sender) = EVENT_SENDER.get() {
                let _ = sender.send(RightAltEvent::Cancel);
            }
        }
    }

    // SAFETY: Passing unhandled events to the next hook is required by the
    // Windows hook contract.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}
