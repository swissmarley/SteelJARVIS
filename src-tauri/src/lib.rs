mod commands;
mod agent;
mod memory;
mod desktop;
mod voice;
mod search;
mod session;
mod permissions;
mod observability;
mod config;
mod tray;

use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

use voice::{SpeechManager, ClapDetector, ActivationManager, AudioCapture, SpeechRecognizer};
use voice::clap_detector::ClapConfig;
use search::SearchProvider;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let api_key = std::env::var("ANTHROPIC_API_KEY")
                .or_else(|_| dotenvy::var("ANTHROPIC_API_KEY"))
                .unwrap_or_default();

            let agent = agent::AgentEngine::new(api_key);
            app.manage(Mutex::new(agent));

            let mem_path = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir")
                .join("jarvis.db");

            std::fs::create_dir_all(mem_path.parent().unwrap())?;
            let mem_store = memory::MemoryStore::new(&mem_path)?;
            app.manage(Mutex::new(mem_store));

            let mut event_bus = observability::EventBus::new(1000);
            event_bus.set_app_handle(app.handle().clone());
            let event_bus = Arc::new(Mutex::new(event_bus));
            app.manage(event_bus.clone());

            let perm_manager = permissions::PermissionManager::new();
            app.manage(Mutex::new(perm_manager));

            let desktop_provider = desktop::MacOSDesktopProvider::new();
            app.manage(Mutex::new(desktop_provider));

            let speech_manager = SpeechManager::new();
            app.manage(Mutex::new(speech_manager));

            let clap_detector = Arc::new(Mutex::new(ClapDetector::new(ClapConfig::default())));
            app.manage(clap_detector.clone());

            let activation_manager = ActivationManager::new();
            app.manage(Mutex::new(activation_manager));

            let audio_capture = AudioCapture::new(
                clap_detector,
                event_bus.clone(),
                app.handle().clone(),
            );
            app.manage(Mutex::new(audio_capture));

            let speech_recognizer = SpeechRecognizer::new(event_bus, app.handle().clone())
                .expect("Failed to initialize speech recognizer");
            app.manage(Mutex::new(speech_recognizer));

            let search_api_key = std::env::var("SEARCH_API_KEY")
                .or_else(|_| dotenvy::var("SEARCH_API_KEY"))
                .ok();
            let search_engine_id = std::env::var("SEARCH_ENGINE_ID")
                .or_else(|_| dotenvy::var("SEARCH_ENGINE_ID"))
                .ok();
            let search_provider = SearchProvider::new(search_api_key, search_engine_id);
            app.manage(Mutex::new(search_provider));

            // Setup system tray
            tray::setup_tray(app.handle())?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                let _ = window.app_handle().emit("window-hidden", ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::chat::send_message,
            commands::chat::check_health,
            commands::memory::save_memory,
            commands::memory::search_memories,
            commands::memory::list_memories,
            commands::memory::delete_memory,
            commands::memory::pin_memory,
            commands::desktop::launch_app,
            commands::desktop::open_url,
            commands::desktop::open_file,
            commands::desktop::list_running_apps,
            commands::desktop::quit_app,
            commands::permissions::check_permission,
            commands::permissions::grant_permission,
            commands::permissions::deny_permission,
            commands::permissions::list_permissions,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::voice::speak,
            commands::voice::stop_speaking,
            commands::voice::set_voice,
            commands::voice::set_speech_rate,
            commands::voice::list_voices,
            commands::voice::get_voice_config,
            commands::voice::set_clap_enabled,
            commands::voice::get_activation_config,
            commands::voice::set_clap_sensitivity,
            commands::voice::start_clap_detection,
            commands::voice::stop_clap_detection,
            commands::voice::start_listening,
            commands::voice::stop_listening,
            commands::voice::log_debug,
            commands::search::web_search,
        ])
        .run(tauri::generate_context!())
        .expect("error while running JARVIS");
}