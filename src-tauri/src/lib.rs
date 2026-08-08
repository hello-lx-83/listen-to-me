pub mod adapters;
mod app;
mod app_state;
mod commands;
pub mod core;
pub mod platform;
pub mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app::run();
}
