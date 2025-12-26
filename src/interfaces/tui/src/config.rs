//! Application configuration and runtime mode detection
//!
//! This module provides environment-based configuration for the TUI,
//! including server mode detection for container deployments.

/// Check if running in server mode (skip prereq checks)
/// 
/// **Container-only behavior**: Local developer runs default to full
/// prerequisite checks unless this flag is explicitly set.
/// 
/// When `ODD_DASHBOARD_SERVER_MODE=1` or `=true`:
/// - Prerequisite checks (Docker/kubectl/kind/pwsh) are bypassed
/// - No subprocess spawning for tool detection
/// - PrerequisiteSetup view is skipped
/// - Prominent warning banner is displayed
/// 
/// # Contract
/// The PTY server (`web-pty-server`) automatically sets this flag when
/// spawning the TUI in a container environment.
pub fn is_server_mode() -> bool {
    std::env::var("ODD_DASHBOARD_SERVER_MODE")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_server_mode_off_by_default() {
        // Ensure we're testing with flag unset
        std::env::remove_var("ODD_DASHBOARD_SERVER_MODE");
        assert!(!is_server_mode());
    }
    
    #[test]
    fn test_server_mode_enabled_with_1() {
        std::env::set_var("ODD_DASHBOARD_SERVER_MODE", "1");
        assert!(is_server_mode());
        std::env::remove_var("ODD_DASHBOARD_SERVER_MODE");
    }
    
    #[test]
    fn test_server_mode_enabled_with_true() {
        std::env::set_var("ODD_DASHBOARD_SERVER_MODE", "true");
        assert!(is_server_mode());
        std::env::remove_var("ODD_DASHBOARD_SERVER_MODE");
    }
    
    #[test]
    fn test_server_mode_enabled_case_insensitive() {
        std::env::set_var("ODD_DASHBOARD_SERVER_MODE", "TRUE");
        assert!(is_server_mode());
        std::env::remove_var("ODD_DASHBOARD_SERVER_MODE");
    }
    
    #[test]
    fn test_server_mode_disabled_with_0() {
        std::env::set_var("ODD_DASHBOARD_SERVER_MODE", "0");
        assert!(!is_server_mode());
        std::env::remove_var("ODD_DASHBOARD_SERVER_MODE");
    }
    
    #[test]
    fn test_server_mode_disabled_with_other() {
        std::env::set_var("ODD_DASHBOARD_SERVER_MODE", "false");
        assert!(!is_server_mode());
        std::env::remove_var("ODD_DASHBOARD_SERVER_MODE");
    }
}
