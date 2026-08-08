# Windows P0 manual test

## Build

```powershell
pnpm tauri build --debug --no-bundle
```

The executable is created at:

```text
src-tauri/target/debug/listen-to-me.exe
```

## Right Alt hold test

1. Start Listen to Me.
2. Open **设置 → 模型与网络** and enter a newly rotated Qwen API Key. Confirm
   the field clears and the status changes to **已配置**.
3. Open Notepad and place the caret in an empty document.
4. Tap Right Alt for less than 220 ms.
5. Confirm that no overlay appears and no text is inserted.
6. Hold Right Alt for at least 220 ms and speak a short Mandarin sentence.
7. Confirm that the voice overlay appears without taking focus from Notepad.
8. Release Right Alt and observe recording → transcribing → rewriting → inserting.
   Confirm the overlay shows input level while recording and recognition/rewrite
   timings during processing.
9. Confirm that the cleaned-up sentence is inserted at the original caret and
   the overlay disappears.

## Cancellation test

1. Hold Right Alt until the overlay appears.
2. Press Escape before releasing Right Alt.
3. Confirm that the overlay disappears and no text is inserted.
4. Repeat by pressing Escape during transcribing and rewriting; confirm no history
   row is created and no delayed text appears.

## Compatibility checks

Repeat the hold test in:

- Windows Notepad.
- A Chromium browser text field.
- A multiline textarea.
- Microsoft Word, if installed.
- A commonly used IM client, if available.

Record failures separately for:

- Elevated applications. Windows blocks lower-integrity input injection into
  higher-integrity processes.
- Keyboard layouts where Right Alt acts as AltGr.
- Applications that ignore Unicode `SendInput`; these will use the future
  transactional clipboard fallback.

## Management client test

1. In **语音与语言**, change language and rewrite mode, leave the page and
   return, then confirm both values persisted.
2. In **词典**, add a categorized correction, edit it, and perform a voice
   input containing the original term. Confirm the expected spelling is used.
3. Open **历史记录** and confirm the session contains both the original
   transcript and final output. Test copy, single delete and clear-all.
4. Disable **保存输入历史**, complete another voice input, and confirm no new
   history row is created.
5. In **模型与网络**, run **测试连接** and confirm a successful or sanitized
   failure message is shown without revealing the API key.

## Lifecycle test

1. Fully exit every Listen to Me development instance from its tray menu.
2. Start the latest executable twice. Confirm the second launch focuses the
   existing main window instead of creating another tray icon or keyboard hook.
3. Close the main window. Confirm voice input remains available and the tray
   icon remains visible.
4. Left-click the tray icon to restore the window; use the tray menu **退出** to
   stop the process completely.
5. Toggle **开机自动启动**, restart the app and confirm the switch reflects the
   Windows registration state. A real login-cycle test is required before release.

## Current spike limitations

- Right Alt is reserved and swallowed while the application is running.
- The overlay is centered for the spike; final placement is not implemented.
- Text is injected directly with Unicode `SendInput`. Clipboard fallback and
  full clipboard restoration are deferred until an incompatible target is
  reproduced.
- Audio capture is capped at two minutes and transcription begins after Right Alt
  is released. Provider responses are consumed as a stream, but microphone audio
  is not uploaded while recording.
- A failed session shows a sanitized reason; detailed diagnostic logs are not yet
  exposed in the client.
- The current direct Unicode injection cannot cross Windows integrity levels.
  Clipboard fallback remains deferred until a reproducible incompatible target
  is identified, because restoring every clipboard format safely is non-trivial.
