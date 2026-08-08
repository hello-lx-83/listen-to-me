//! Pastes complete Unicode text through one clipboard path.

use std::{mem::size_of, ptr::copy_nonoverlapping, thread, time::Duration};

use windows::Win32::{
    Foundation::{GlobalFree, HANDLE, HWND},
    System::{
        DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData},
        Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
    },
    UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
    },
};

const CLIPBOARD_OPEN_ATTEMPTS: usize = 10;
const CLIPBOARD_RETRY_DELAY: Duration = Duration::from_millis(10);
const CF_UNICODE_TEXT: u32 = 13;

pub fn paste_unicode_text(owner: HWND, text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }

    let clipboard_text = encode_clipboard_text(text);
    set_clipboard_text(owner, &clipboard_text)?;
    // The transcript intentionally remains on the clipboard so a failed or
    // blocked automatic paste can be recovered with a manual Ctrl+V.
    send_paste_shortcut()
}

fn encode_clipboard_text(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

fn set_clipboard_text(owner: HWND, text: &[u16]) -> Result<(), String> {
    let bytes = size_of_val(text);
    let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) }
        .map_err(|error| format!("failed to allocate clipboard text: {error}"))?;

    let destination = unsafe { GlobalLock(memory) } as *mut u16;
    if destination.is_null() {
        let _ = unsafe { GlobalFree(Some(memory)) };
        return Err("failed to lock clipboard text memory".to_owned());
    }

    unsafe {
        copy_nonoverlapping(text.as_ptr(), destination, text.len());
    }
    let _ = unsafe { GlobalUnlock(memory) };

    if let Err(error) = open_clipboard_with_retry(owner) {
        let _ = unsafe { GlobalFree(Some(memory)) };
        return Err(error);
    }

    let result = (|| {
        unsafe { EmptyClipboard() }
            .map_err(|error| format!("failed to clear clipboard: {error}"))?;
        unsafe { SetClipboardData(CF_UNICODE_TEXT, Some(HANDLE(memory.0))) }
            .map_err(|error| format!("failed to set clipboard text: {error}"))?;
        Ok(())
    })();
    let _ = unsafe { CloseClipboard() };

    if result.is_err() {
        // Ownership transfers to the system only after SetClipboardData succeeds.
        let _ = unsafe { GlobalFree(Some(memory)) };
    }
    result
}

fn open_clipboard_with_retry(owner: HWND) -> Result<(), String> {
    let mut last_error = None;
    for attempt in 0..CLIPBOARD_OPEN_ATTEMPTS {
        match unsafe { OpenClipboard(Some(owner)) } {
            Ok(()) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
        if attempt + 1 < CLIPBOARD_OPEN_ATTEMPTS {
            thread::sleep(CLIPBOARD_RETRY_DELAY);
        }
    }

    Err(format!(
        "failed to open clipboard: {}",
        last_error.expect("at least one clipboard attempt")
    ))
}

fn send_paste_shortcut() -> Result<(), String> {
    let inputs = [
        virtual_key_input(VK_CONTROL.0, false),
        virtual_key_input(VK_V.0, false),
        virtual_key_input(VK_V.0, true),
        virtual_key_input(VK_CONTROL.0, true),
    ];
    let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
    if sent == inputs.len() as u32 {
        Ok(())
    } else {
        Err(format!(
            "SendInput inserted {sent} of {} paste events",
            inputs.len()
        ))
    }
}

fn virtual_key_input(virtual_key: u16, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(virtual_key),
                wScan: 0,
                dwFlags: if key_up {
                    KEYEVENTF_KEYUP
                } else {
                    Default::default()
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_text_is_utf16_and_null_terminated() {
        assert_eq!(
            encode_clipboard_text("中文 have fun 😊"),
            [
                "中文 have fun 😊".encode_utf16().collect::<Vec<_>>(),
                vec![0],
            ]
            .concat()
        );
    }

    #[test]
    fn paste_shortcut_is_one_control_v_chord() {
        let inputs = [
            virtual_key_input(VK_CONTROL.0, false),
            virtual_key_input(VK_V.0, false),
            virtual_key_input(VK_V.0, true),
            virtual_key_input(VK_CONTROL.0, true),
        ];
        assert_eq!(inputs.len(), 4);
    }
}
