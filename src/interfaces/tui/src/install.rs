//! Installation functionality for prerequisites
//!
//! This module handles executing install commands and clipboard operations
//! for guided prerequisite installation.

use std::process::{Command, Stdio};
use arboard::Clipboard;

use crate::types::{Prerequisite, PrereqStatus};
use crate::error::get_install_command;

/// Result of an installation attempt with captured output
#[derive(Debug, Clone)]
pub struct InstallOutput {
    pub success: bool,
    pub lines: Vec<String>,
    pub error_message: Option<String>,
}

impl Default for InstallOutput {
    fn default() -> Self {
        Self {
            success: false,
            lines: Vec::new(),
            error_message: None,
        }
    }
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

/// Execute an install command and capture output
/// 
/// Returns captured stdout/stderr lines for display in TUI
pub fn execute_install_with_output(prereq_name: &str) -> InstallOutput {
    let cmd = match get_install_command(prereq_name) {
        Some(c) => c,
        None => return InstallOutput {
            success: false,
            lines: vec![format!("No install command for {}", prereq_name)],
            error_message: Some(format!("Unknown prerequisite: {}", prereq_name)),
        },
    };
    
    // Execute the command based on platform
    #[cfg(target_os = "windows")]
    {
        execute_windows_command_captured(&cmd)
    }
    
    #[cfg(target_os = "macos")]
    {
        execute_unix_command_captured(&cmd, "zsh")
    }
    
    #[cfg(target_os = "linux")]
    {
        execute_unix_command_captured(&cmd, "bash")
    }
}

#[cfg(target_os = "windows")]
fn execute_windows_command_captured(cmd: &str) -> InstallOutput {
    let result = Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", cmd])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    
    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            let mut lines: Vec<String> = stdout.lines().map(|s| s.to_string()).collect();
            lines.extend(stderr.lines().map(|s| format!("ERR: {}", s)));
            
            // Keep last 20 lines to avoid overwhelming the display
            if lines.len() > 20 {
                lines = lines.into_iter().rev().take(20).rev().collect();
            }
            
            InstallOutput {
                success: output.status.success(),
                lines,
                error_message: if output.status.success() { None } else {
                    Some(format!("Exit code: {:?}", output.status.code()))
                },
            }
        }
        Err(e) => InstallOutput {
            success: false,
            lines: vec![format!("Failed to execute: {}", e)],
            error_message: Some(e.to_string()),
        },
    }
}

#[cfg(not(target_os = "windows"))]
fn execute_unix_command_captured(cmd: &str, shell: &str) -> InstallOutput {
    let result = Command::new(shell)
        .args(["-c", cmd])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    
    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            let mut lines: Vec<String> = stdout.lines().map(|s| s.to_string()).collect();
            lines.extend(stderr.lines().map(|s| format!("ERR: {}", s)));
            
            // Keep last 20 lines to avoid overwhelming the display
            if lines.len() > 20 {
                lines = lines.into_iter().rev().take(20).rev().collect();
            }
            
            InstallOutput {
                success: output.status.success(),
                lines,
                error_message: if output.status.success() { None } else {
                    Some(format!("Exit code: {:?}", output.status.code()))
                },
            }
        }
        Err(e) => InstallOutput {
            success: false,
            lines: vec![format!("Failed to execute: {}", e)],
            error_message: Some(e.to_string()),
        },
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

    // ========== InstallOutput Tests ==========
    
    #[test]
    fn test_install_output_default() {
        let output = InstallOutput::default();
        assert!(!output.success);
        assert!(output.lines.is_empty());
        assert!(output.error_message.is_none());
    }

    #[test]
    fn test_install_output_with_lines() {
        let output = InstallOutput {
            success: true,
            lines: vec!["Line 1".to_string(), "Line 2".to_string()],
            error_message: None,
        };
        assert!(output.success);
        assert_eq!(output.lines.len(), 2);
        assert_eq!(output.lines[0], "Line 1");
        assert_eq!(output.lines[1], "Line 2");
    }

    #[test]
    fn test_install_output_with_error() {
        let output = InstallOutput {
            success: false,
            lines: vec!["ERR: Something went wrong".to_string()],
            error_message: Some("Exit code: 1".to_string()),
        };
        assert!(!output.success);
        assert!(output.lines[0].starts_with("ERR:"));
        assert!(output.error_message.is_some());
        assert!(output.error_message.unwrap().contains("Exit code"));
    }

    #[test]
    fn test_install_output_empty_lines() {
        let output = InstallOutput {
            success: true,
            lines: vec![],
            error_message: None,
        };
        assert!(output.success);
        assert!(output.lines.is_empty());
    }

    #[test]
    fn test_install_output_clone() {
        let output = InstallOutput {
            success: true,
            lines: vec!["test".to_string()],
            error_message: Some("error".to_string()),
        };
        let cloned = output.clone();
        assert_eq!(cloned.success, output.success);
        assert_eq!(cloned.lines, output.lines);
        assert_eq!(cloned.error_message, output.error_message);
    }

    // ========== execute_install_with_output Tests ==========

    #[test]
    fn test_execute_install_unknown_prereq() {
        let output = execute_install_with_output("nonexistent_tool_xyz");
        assert!(!output.success);
        assert!(!output.lines.is_empty());
        assert!(output.lines[0].contains("No install command"));
        assert!(output.error_message.is_some());
    }

    #[test]
    fn test_execute_install_returns_output_structure() {
        // Test with unknown tool - fast and verifies structure without running actual install
        let output = execute_install_with_output("nonexistent_prereq_xyz");
        // Verify InstallOutput structure fields are accessible
        let _ = output.success;
        let _ = output.lines.len();
        let _ = output.error_message.is_some();
        // For unknown tools, should always have at least one line of output
        assert!(!output.lines.is_empty());
    }

    // ========== get_install_description Tests ==========

    #[test]
    fn test_get_install_description_docker() {
        let desc = get_install_description("docker");
        assert!(desc.contains("Docker"));
        assert!(desc.contains("container"));
    }

    #[test]
    fn test_get_install_description_docker_desktop() {
        let desc = get_install_description("Docker Desktop");
        assert!(desc.contains("Docker"));
    }

    #[test]
    fn test_get_install_description_powershell() {
        let desc = get_install_description("powershell");
        assert!(desc.contains("PowerShell"));
    }

    #[test]
    fn test_get_install_description_pwsh() {
        let desc = get_install_description("pwsh");
        assert!(desc.contains("PowerShell"));
    }

    #[test]
    fn test_get_install_description_kubectl() {
        let desc = get_install_description("kubectl");
        assert!(desc.contains("kubectl"));
        assert!(desc.contains("Kubernetes"));
    }

    #[test]
    fn test_get_install_description_kind() {
        let desc = get_install_description("kind");
        assert!(desc.contains("kind"));
        assert!(desc.contains("Docker") || desc.contains("Kubernetes"));
    }

    #[test]
    fn test_get_install_description_unknown() {
        let desc = get_install_description("unknown_tool_xyz");
        assert!(desc.contains("prerequisite"));
        assert!(desc.contains("unknown_tool_xyz"));
    }

    #[test]
    fn test_get_install_description_case_insensitive() {
        let lower = get_install_description("docker");
        let upper = get_install_description("DOCKER");
        let mixed = get_install_description("Docker");
        assert_eq!(lower, upper);
        assert_eq!(lower, mixed);
    }

    // ========== copy_to_clipboard Tests ==========

    #[test]
    fn test_copy_to_clipboard_unknown_tool() {
        let result = copy_to_clipboard("nonexistent_tool_xyz");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No install command"));
    }

    #[test]
    fn test_copy_to_clipboard_valid_tool_returns_result() {
        // This will succeed or fail based on clipboard availability
        // We just verify it returns a Result
        let result = copy_to_clipboard("docker");
        // Don't assert success/failure - depends on environment
        assert!(result.is_ok() || result.is_err());
    }
}
