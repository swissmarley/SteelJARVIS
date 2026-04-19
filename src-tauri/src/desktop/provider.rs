use std::process::Command;

use super::DesktopError;

pub struct MacOSDesktopProvider;

impl MacOSDesktopProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn launch_app_sync(&self, name: &str) -> Result<String, DesktopError> {
        run_applescript(&format!("tell application \"{}\" to activate", name))
    }

    pub fn open_url_sync(&self, url: &str) -> Result<String, DesktopError> {
        let output = Command::new("open")
            .arg(url)
            .output()
            .map_err(DesktopError::Io)?;

        if output.status.success() {
            Ok(format!("Opened {}", url))
        } else {
            Err(DesktopError::AppleScript(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    pub fn open_file_sync(&self, path: &str) -> Result<String, DesktopError> {
        let output = Command::new("open")
            .arg(path)
            .output()
            .map_err(DesktopError::Io)?;

        if output.status.success() {
            Ok(format!("Opened {}", path))
        } else {
            Err(DesktopError::AppleScript(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    pub fn list_running_apps_sync(&self) -> Result<Vec<String>, DesktopError> {
        let output = Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to get name of every process whose background only is false")
            .output()
            .map_err(DesktopError::Io)?;

        if output.status.success() {
            let result = String::from_utf8_lossy(&output.stdout);
            let apps: Vec<String> = result
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            Ok(apps)
        } else {
            Err(DesktopError::AppleScript(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    pub fn quit_app_sync(&self, name: &str) -> Result<String, DesktopError> {
        run_applescript(&format!("tell application \"{}\" to quit", name))
    }
}

fn run_applescript(script: &str) -> Result<String, DesktopError> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(DesktopError::Io)?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(DesktopError::AppleScript(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}