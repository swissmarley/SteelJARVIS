use std::sync::Mutex;
use tauri::State;

use crate::desktop::MacOSDesktopProvider;

#[tauri::command]
pub fn launch_app(
    name: String,
    provider: State<'_, Mutex<MacOSDesktopProvider>>,
) -> Result<String, String> {
    let p = provider.lock().map_err(|e| e.to_string())?;
    // DesktopProvider methods are synchronous internally
    p.launch_app_sync(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_url(
    url: String,
    provider: State<'_, Mutex<MacOSDesktopProvider>>,
) -> Result<String, String> {
    let p = provider.lock().map_err(|e| e.to_string())?;
    p.open_url_sync(&url).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_file(
    path: String,
    provider: State<'_, Mutex<MacOSDesktopProvider>>,
) -> Result<String, String> {
    let p = provider.lock().map_err(|e| e.to_string())?;
    p.open_file_sync(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_running_apps(
    provider: State<'_, Mutex<MacOSDesktopProvider>>,
) -> Result<Vec<String>, String> {
    let p = provider.lock().map_err(|e| e.to_string())?;
    p.list_running_apps_sync().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn quit_app(
    name: String,
    provider: State<'_, Mutex<MacOSDesktopProvider>>,
) -> Result<String, String> {
    let p = provider.lock().map_err(|e| e.to_string())?;
    p.quit_app_sync(&name).map_err(|e| e.to_string())
}