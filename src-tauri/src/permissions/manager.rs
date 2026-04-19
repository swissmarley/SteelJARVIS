use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PermissionCategory {
    AppLaunch,
    AppControl,
    FileAccess,
    WindowControl,
    WebSearch,
    MemoryWrite,
    Clipboard,
    NetworkAccess,
}

impl PermissionCategory {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "app_launch" => Self::AppLaunch,
            "app_control" => Self::AppControl,
            "file_access" => Self::FileAccess,
            "window_control" => Self::WindowControl,
            "web_search" => Self::WebSearch,
            "memory_write" => Self::MemoryWrite,
            "clipboard" => Self::Clipboard,
            "network" => Self::NetworkAccess,
            _ => Self::NetworkAccess,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::AppLaunch => "app_launch",
            Self::AppControl => "app_control",
            Self::FileAccess => "file_access",
            Self::WindowControl => "window_control",
            Self::WebSearch => "web_search",
            Self::MemoryWrite => "memory_write",
            Self::Clipboard => "clipboard",
            Self::NetworkAccess => "network",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionLevel {
    Allowed,
    AskOnce,
    AskAlways,
    Denied,
}

impl PermissionLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "allowed" => Self::Allowed,
            "ask_once" => Self::AskOnce,
            "ask_always" => Self::AskAlways,
            "denied" => Self::Denied,
            _ => Self::AskOnce,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Allowed => "allowed",
            Self::AskOnce => "ask_once",
            Self::AskAlways => "ask_always",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Serialize)]
struct PermissionEntry {
    category: String,
    level: String,
}

pub struct PermissionManager {
    rules: HashMap<PermissionCategory, PermissionLevel>,
}

impl PermissionManager {
    pub fn new() -> Self {
        let mut rules = HashMap::new();
        rules.insert(PermissionCategory::AppLaunch, PermissionLevel::AskOnce);
        rules.insert(PermissionCategory::AppControl, PermissionLevel::AskAlways);
        rules.insert(PermissionCategory::FileAccess, PermissionLevel::AskOnce);
        rules.insert(PermissionCategory::WindowControl, PermissionLevel::AskOnce);
        rules.insert(PermissionCategory::WebSearch, PermissionLevel::Allowed);
        rules.insert(PermissionCategory::MemoryWrite, PermissionLevel::Allowed);
        rules.insert(PermissionCategory::Clipboard, PermissionLevel::AskOnce);
        rules.insert(PermissionCategory::NetworkAccess, PermissionLevel::Allowed);
        Self { rules }
    }

    pub fn check(&self, category: PermissionCategory, _action: &str) -> bool {
        match self.rules.get(&category) {
            Some(PermissionLevel::Allowed) => true,
            Some(PermissionLevel::Denied) => false,
            Some(PermissionLevel::AskOnce) => true, // would need persistent state for real ask-once
            Some(PermissionLevel::AskAlways) => true, // frontend handles prompt
            None => false,
        }
    }

    pub fn set(&mut self, category: PermissionCategory, level: PermissionLevel) {
        self.rules.insert(category, level);
    }

    pub fn list_all(&self) -> serde_json::Value {
        let entries: Vec<PermissionEntry> = self
            .rules
            .iter()
            .map(|(cat, lvl)| PermissionEntry {
                category: cat.as_str().to_string(),
                level: lvl.as_str().to_string(),
            })
            .collect();
        serde_json::json!({ "permissions": entries })
    }
}