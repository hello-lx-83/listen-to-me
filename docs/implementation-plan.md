# Listen to Me implementation plan

## Current status — 2026-08-08

- P0 implementation is complete and passes unit, build and startup checks.
- Physical Right Alt and target-application behavior awaits manual desktop testing.
- Default-device capture, mono PCM buffering and in-memory WAV encoding are
  connected to the Right Alt runtime and covered by an encoding test.
- Qwen ASR and cleanup rewriting are connected to the runtime pipeline. No live
  request has been made by the implementation agent.
- The settings client can save, replace and remove the Qwen API key through the
  current Windows user's Credential Manager. The frontend only receives a
  configured/not-configured flag and never reads the stored value.
- SQLite migrations and functional clients for history, dictionary and settings
  are implemented. No source-application or audio data is stored.
- Dictionary entries are sent as ASR vocabulary context and applied as
  deterministic corrections before rewriting.
- Tray lifecycle, autostart controls and the official single-instance plugin are
  implemented. A clean-process manual single-instance check remains because an
  older development instance was already running during automated validation.
- The first optimization pass is implemented: 16 kHz mono resampling, a
  two-minute capture cap, streaming Qwen response parsing, retry for transient
  gateway failures, request cancellation, live input level, stage timings and
  frontend route/window code splitting.
- Next: run the updated manual compatibility matrix against real target apps,
  use its timings as the baseline, then tune model prompts and injection fallbacks.

## Delivery order

### P0 — Windows interaction spike

- Detect physical Right Alt press and release globally.
- Ignore key-repeat events and distinguish Right Alt from left Alt.
- Enter recording state only after a configurable 220 ms hold threshold.
- Keep the current foreground application focused.
- Show the non-focusable `voice-overlay` window while a session is active.
- On release, inject the processed Unicode text into the focused control.
- Support cancellation and ensure stale session work cannot complete later.

Acceptance: holding Right Alt in Notepad shows the overlay; releasing it inserts
the processed speech at the caret; a short tap does not start a session.

### P1 — Audio capture

- Select and open the default Windows input device.
- Capture mono PCM without passing audio through the webview.
- Display throttled input level in the overlay.
- Stop, cancel and release the device deterministically.
- Encode an in-memory WAV payload for cloud ASR.

Acceptance: a hold session produces a valid local WAV in memory and reports an
audio level without saving raw audio to disk.

### P2 — Cloud vertical slice

- Implement Qwen ASR behind `SpeechRecognizer`.
- Implement Qwen text cleanup behind `TextRewriter`.
- Add timeout, cancellation, retry and sanitized error mapping.
- Complete hold → record → ASR → rewrite → insert.
- Do not log authorization headers, raw audio or complete user text.

Initial candidates from the provider documentation:

- ASR: `qwen3-asr-flash`, Base64 16 kHz mono WAV input with streamed response parsing.
- Rewrite: a low-latency Qwen Flash model through the OpenAI-compatible chat
  endpoint; the exact account-available model is confirmed with a model-list or
  minimal request before becoming a default.

### P3 — Local data and management client

- SQLite migrations for history, dictionary and typed settings. (Implemented.)
- Windows Credential Manager for provider secrets. (Implemented for Qwen.)
- History list/detail/copy/delete/clear. (Implemented.)
- Dictionary CRUD with category retained. (Implemented.)
- Model connection test and route settings. (Connection test implemented;
  local route remains intentionally disabled until P5.)

### P4 — Reliability and packaging

- Single-instance and tray lifecycle. (Implemented; manual clean-process check pending.)
- Autostart setting. (Implemented.)
- Recovery after provider/network failure.
- Installer, update strategy and privacy defaults.
- Test matrix for Notepad, browsers, Office and common IM clients.

### P5 — Local inference

- Measure target machine CPU, RAM and optional GPU capabilities.
- Select local ASR and rewrite runtimes from benchmark results.
- Implement local adapters without changing the input pipeline.
- Add model download, checksum, storage and removal flows.

## Resources needed from the product owner

Provided:

- Qwen API access. The secret must be entered through the app or operating-system
  credential store and must never be committed to the repository.

Needed before P2 acceptance:

- Confirmation that the account can call the intended ASR and text models.
- A small billing/spend limit suitable for development tests.
- 5–10 short Mandarin recordings covering normal speech, English terms, names
  and punctuation, together with expected transcripts.
- 10–20 examples of raw spoken text and the preferred output for each rewrite
  mode.

Needed before packaging:

- Final product name, publisher name and Windows signing certificate decision.
- App icon assets in SVG or high-resolution PNG.
- Privacy policy and retention defaults.

Needed before local inference:

- Minimum supported Windows version.
- Minimum target hardware and acceptable model download size.
- Whether NVIDIA GPU acceleration is required or CPU-only must be supported.

## Safety notes

- Any API key posted in chat should be rotated after development setup.
- `.env` files are ignored, but production secrets will use Windows Credential
  Manager rather than repository files.
- The Right Alt hook is Windows-only and will need explicit AltGr-layout testing.
- Escape cancels the active request/response stream and suppresses history and
  text injection. Provider-side billing behavior after network cancellation is
  controlled by the provider and cannot be guaranteed by the client.
