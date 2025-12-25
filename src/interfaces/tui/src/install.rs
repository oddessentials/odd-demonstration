//! Installation functionality for prerequisites
//!
//! This module handles executing install commands and clipboard operations
//! for guided prerequisite installation.

use std::process::{Command, Stdio};
use arboard::Clipboard;

use crate::types::{Prerequisite, PrereqStatus};
use crate::error::get_install_command;

/// Result of an installation attempt
#[derive(Debug, Clone)]
pub enum InstallResult {
    Success,
    Failed(String),
    Skipped,
}

/// Copy install command to clipboard
pub fn copy_to_clipboard(prereq_name: &str) -> Result<(), String> {
    let cmd = get_install_command(prereq_name)
        .ok_or_else(|| format!("No install command for {}", prereq_name))?;
    
    let mut clipboard = Clipboard::new()
        .map_err(|e| format!("Failed to access clipboard: {}", e))?;
    
    clipboard.set_text(&cmd)
        .map_err(|e| format!("Failed to copy to clipboard: {}", e))?;
    
    Ok(())
}

/// Execute an install command for a prerequisite
/// 
/// # Safety
/// This executes shell commands on the user's system. The commands are
/// platform-specific and come from the error module's install step functions.
pub fn execute_install(prereq_name: &str) -> InstallResult {
    let cmd = match get_install_command(prereq_name) {
        Some(c) => c,
        None => return InstallResult::Failed(format!("No install command for {}", prereq_name)),
    };
    
    // Execute the command based on platform
    #[cfg(target_os = "windows")]
    {
        execute_windows_command(&cmd)
    }
    
    #[cfg(target_os = "macos")]
    {
        execute_unix_command(&cmd, "zsh")
    }
    
    #[cfg(target_os = "linux")]
    {
        execute_unix_command(&cmd, "bash")
    }
}

#[cfg(target_os = "windows")]
fn execute_windows_command(cmd: &str) -> InstallResult {
    // Use PowerShell to execute the command
    let result = Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", cmd])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();
    
    match result {
        Ok(status) if status.success() => InstallResult::Success,
        Ok(status) => InstallResult::Failed(format!("Command exited with code: {:?}", status.code())),
        Err(e) => InstallResult::Failed(format!("Failed to execute: {}", e)),
    }
}

#[cfg(not(target_os = "windows"))]
fn execute_unix_command(cmd: &str, shell: &str) -> InstallResult {
    let result = Command::new(shell)
        .args(["-c", cmd])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();
    
    match result {
        Ok(status) if status.success() => InstallResult::Success,
        Ok(status) => InstallResult::Failed(format!("Command exited with code: {:?}", status.code())),
        Err(e) => InstallResult::Failed(format!("Failed to execute: {}", e)),
    }
}

/// Get user-friendly description of what will be installed
pub fn get_install_description(prereq_name: &str) -> String {
    match prereq_name.to_lowercase().as_str() {
        "docker" | "docker desktop" => "Docker Desktop - container runtime and Kubernetes support".to_string(),
        "powershell" | "powershell core" | "pwsh" => "PowerShell Core - cross-platform shell for running setup scripts".to_string(),
        "kubectl" => "kubectl - Kubernetes command-line tool".to_string(),
        "kind" => "kind - Kubernetes in Docker for local clusters".to_string(),
        _ => format!("{} - required prerequisite", prereq_name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_result_variants() {
        let success = InstallResult::Success;
        let failed = InstallResult::Failed("error".to_string());
        let skipped = InstallResult::Skipped;
        
        assert!(matches!(success, InstallResult::Success));
        assert!(matches!(failed, InstallResult::Failed(_)));
        assert!(matches!(skipped, InstallResult::Skipped));
    }

    #[test]
    fn test_get_install_description() {
        let docker_desc = get_install_description("docker");
        assert!(docker_desc.contains("Docker"));
        
        let kubectl_desc = get_install_description("kubectl");
        assert!(kubectl_desc.contains("kubectl"));
        
        let unknown_desc = get_install_description("unknown_tool");
        assert!(unknown_desc.contains("prerequisite"));
    }

    #[test]
    fn test_copy_to_clipboard_unknown_tool() {
        let result = copy_to_clipboard("nonexistent_tool_xyz");
        assert!(result.is_err());
    }
}
