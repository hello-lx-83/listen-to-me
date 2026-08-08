//! Inserts Unicode text into the control that currently owns keyboard focus.

use std::mem::size_of;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    VIRTUAL_KEY,
};

const CODE_UNITS_PER_BATCH: usize = 256;

pub fn send_unicode_text(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }

    let code_units = text.encode_utf16().collect::<Vec<_>>();
    for batch in code_units.chunks(CODE_UNITS_PER_BATCH) {
        let mut inputs = Vec::with_capacity(batch.len() * 2);
        for code_unit in batch {
            inputs.push(keyboard_input(*code_unit, false));
            inputs.push(keyboard_input(*code_unit, true));
        }

        // SAFETY: The slice owns fully initialized INPUT values for the duration
        // of the call, and cbSize matches the Windows INPUT structure.
        let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
        if sent != inputs.len() as u32 {
            return Err(format!(
                "SendInput inserted {sent} of {} keyboard events",
                inputs.len()
            ));
        }
    }

    Ok(())
}

fn keyboard_input(code_unit: u16, key_up: bool) -> INPUT {
    let flags = if key_up {
        KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
    } else {
        KEYEVENTF_UNICODE
    };

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: code_unit,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}
