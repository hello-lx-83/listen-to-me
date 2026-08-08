# Listen to Me architecture

## Decision

The application starts as a modular monolith: one React application and one
Cargo package. The Rust core follows lightweight hexagonal boundaries so cloud
and local implementations can be replaced without moving orchestration into the
webview.

## Runtime ownership

- Rust owns the voice-session state machine, hotkey, audio, model calls, text
  injection and persistence.
- React owns navigation, configuration screens and projections of Rust state.
- The `main` window is the management client.
- The hidden `voice-overlay` window is a small status surface controlled by Rust.

## Frontend boundaries

- `app`: entry points, providers and routing.
- `layouts`: primary and settings navigation shells.
- `pages`: route-level components.
- `features`: feature behavior and feature-local UI as it is implemented.
- `components/ui`: unmodified shadcn primitives.
- `components/app`: product-specific compositions.
- `services`: typed Tauri IPC and event access.
- `shared`: frontend contracts and utilities.

## Rust boundaries

- `core`: state machine, models, policies and ports. It does not depend on
  Tauri, providers or Windows APIs.
- `services`: use-case orchestration.
- `commands`: thin Tauri request/response adapters.
- `adapters`: cloud, local, audio, storage and secret implementations.
- `platform/windows`: global input, focus, overlay and text-injection details.

## Dependency rule

Outer modules depend inward. `core` must not import from `commands`, `adapters`,
`platform` or `services`. Concrete implementations are assembled in `app.rs`.

## Deferred decisions

- The cloud ASR and rewrite providers.
- SQLite and credential-store crates.
- Tauri global shortcut versus a low-level Windows hook for modifier-only Right
  Alt.
- The local inference runtime and whether it needs a sidecar or Cargo workspace.
