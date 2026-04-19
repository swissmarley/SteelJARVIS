use std::sync::Mutex;
use tauri::State;

use crate::permissions::{PermissionManager, PermissionCategory, PermissionLevel};

#[tauri::command]
pub fn check_permission(
    category: String,
    action: String,
    manager: State<'_, Mutex<PermissionManager>>,
) -> Result<bool, String> {
    let manager = manager.lock().map_err(|e| e.to_string())?;
    let cat = PermissionCategory::from_str(&category);
    Ok(manager.check(cat, &action))
}

#[tauri::command]
pub fn grant_permission(
    category: String,
    level: String,
    manager: State<'_, Mutex<PermissionManager>>,
) -> Result<(), String> {
    let mut manager = manager.lock().map_err(|e| e.to_string())?;
    let cat = PermissionCategory::from_str(&category);
    let lvl = PermissionLevel::from_str(&level);
    manager.set(cat, lvl);
    Ok(())
}

#[tauri::command]
pub fn deny_permission(
    category: String,
    manager: State<'_, Mutex<PermissionManager>>,
) -> Result<(), String> {
    let mut manager = manager.lock().map_err(|e| e.to_string())?;
    let cat = PermissionCategory::from_str(&category);
    manager.set(cat, PermissionLevel::Denied);
    Ok(())
}

#[tauri::command]
pub fn list_permissions(
    manager: State<'_, Mutex<PermissionManager>>,
) -> Result<serde_json::Value, String> {
    let manager = manager.lock().map_err(|e| e.to_string())?;
    Ok(manager.list_all())
}