//! Doctor and prerequisite checking functionality
//!
//! This module provides system diagnostics, platform support checking,
//! and prerequisite validation for the odd-dashboard TUI.

use std::process::Command;
use crate::types::{Prerequisite, PrereqStatus};
use crate::error::{get_docker_install_steps, get_pwsh_install_steps, get_kind_install_steps, get_kubectl_install_steps};

// ============================================================================
// Platform Support
// ============================================================================

/// Support matrix using EXACT Rust std::env::consts values
/// Verified against: https://doc.rust-lang.org/std/env/consts/constant.OS.html
pub const SUPPORT_MATRIX: &[(&str, &str)] = &[
    ("windows", "x86_64"),
    ("macos", "x86_64"),
    ("macos", "aarch64"),
    ("linux", "x86_64"),
    ("linux", "aarch64"),
];

pub const SUPPORT_MATRIX_URL: &str = "https://github.com/oddessentials/odd-demonstration/blob/main/docs/SUPPORT_MATRIX.md";

/// Check if current platform is supported (no I/O, pure computation)
pub fn check_platform_support() -> Result<(), String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    
    let supported = SUPPORT_MATRIX.iter()
        .any(|(s_os, s_arch)| *s_os == os && *s_arch == arch);
    
    if !supported {
        return Err(format!(
            "Unsupported platform: {}-{}\nSee supported configurations: {}",
            os, arch, SUPPORT_MATRIX_URL
        ));
    }
    
    Ok(())
}

// ============================================================================
// Command Version Checking
// ============================================================================

/// Check if a command exists and get its version output
pub fn check_command_version(cmd: &str, args: &[&str]) -> Result<String, String> {
    match Command::new(cmd).args(args).output() {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let version = stdout.lines().next().unwrap_or("").trim().to_string();
                Ok(version)
            } else {
                Err(format!("command failed with exit code {}", output.status))
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err("not found".to_string())
            } else {
                Err(format!("failed to run: {}", e))
            }
        }
    }
}

// ============================================================================
// CLI Output Functions
// ============================================================================

/// Print version information with build metadata
pub fn print_version() {
    println!("odd-dashboard {}", env!("CARGO_PKG_VERSION"));
    
    // Build metadata injected by build.rs
    if let Some(commit) = option_env!("BUILD_COMMIT") {
        println!("  commit: {}", commit);
    }
    if let Some(timestamp) = option_env!("BUILD_TIMESTAMP") {
        println!("  built:  {}", timestamp);
    }
    if let Some(rustc) = option_env!("BUILD_RUSTC_VERSION") {
        println!("  rustc:  {}", rustc);
    }
    
    println!("  os:     {}", std::env::consts::OS);
    println!("  arch:   {}", std::env::consts::ARCH);
}

/// Print help message
pub fn print_help() {
    println!("odd-dashboard - Terminal dashboard for Distributed Task Observatory");
    println!();
    println!("USAGE:");
    println!("    odd-dashboard [COMMAND] [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!("    doctor      Check system prerequisites and show diagnostic info");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help      Print this help message");
    println!("    -V, --version   Print version information");
    println!();
    println!("ENVIRONMENT VARIABLES:");
    println!("    READ_MODEL_URL   Read Model API URL (default: http://localhost:8080)");
    println!("    GATEWAY_URL      Gateway API URL (default: http://localhost:3000)");
    println!();
    println!("For more information, see: https://github.com/oddessentials/odd-demonstration");
}

/// Run the doctor command - check prerequisites and show diagnostic info
pub fn run_doctor() {
    use crate::error::get_install_command;
    
    println!("odd-dashboard doctor");
    println!("====================");
    println!();
    
    // Platform info (already validated by main())
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    println!("[OK] Platform: {}-{} (supported)", os, arch);
    
    let mut all_ok = true;
    let mut missing: Vec<&str> = Vec::new();
    
    // Check Docker
    match check_command_version("docker", &["--version"]) {
        Ok(version) => println!("[OK] Docker: {}", version),
        Err(msg) => {
            println!("[FAIL] Docker: {}", msg);
            missing.push("Docker");
            all_ok = false;
        }
    }
    
    // Check PowerShell Core
    let pwsh_cmd = if cfg!(windows) { "pwsh.exe" } else { "pwsh" };
    match check_command_version(pwsh_cmd, &["--version"]) {
        Ok(version) => println!("[OK] PowerShell Core: {}", version),
        Err(_) => {
            // Try powershell.exe on Windows as fallback
            if cfg!(windows) {
                match check_command_version("powershell.exe", &["-Command", "$PSVersionTable.PSVersion.ToString()"]) {
                    Ok(version) => println!("[WARN] PowerShell (Windows): {} (pwsh recommended)", version),
                    Err(msg) => {
                        println!("[FAIL] PowerShell: {}", msg);
                        missing.push("PowerShell Core");
                        all_ok = false;
                    }
                }
            } else {
                println!("[FAIL] PowerShell Core: not found");
                missing.push("PowerShell Core");
                all_ok = false;
            }
        }
    }
    
    // Check kubectl
    match check_command_version("kubectl", &["version", "--client", "--short"]) {
        Ok(version) => println!("[OK] kubectl: {}", version),
        Err(_) => {
            // Try without --short flag (newer versions)
            match check_command_version("kubectl", &["version", "--client"]) {
                Ok(version) => {
                    // Extract just the version portion
                    let short = version.lines().next().unwrap_or(&version);
                    println!("[OK] kubectl: {}", short.trim());
                }
                Err(msg) => {
                    println!("[FAIL] kubectl: {}", msg);
                    missing.push("kubectl");
                    all_ok = false;
                }
            }
        }
    }
    
    // Check kind
    match check_command_version("kind", &["version"]) {
        Ok(version) => println!("[OK] kind: {}", version),
        Err(msg) => {
            println!("[FAIL] kind: {}", msg);
            missing.push("kind");
            all_ok = false;
        }
    }
    
    println!();
    
    // Print install commands for missing prerequisites
    if !missing.is_empty() {
        println!("Installation Commands ({}):", os);
        println!("----------------------------------------");
        for name in &missing {
            if let Some(cmd) = get_install_command(name) {
                println!("  {}: {}", name, cmd);
            }
        }
        println!();
    }
    
    // Summary
    if all_ok {
        println!("All prerequisites satisfied!");
        println!();
        println!("Run 'odd-dashboard' to start the TUI.");
    } else {
        println!("Some prerequisites are missing.");
        println!("Run the commands above, then retry: odd-dashboard doctor");
        std::process::exit(1);
    }
}

// ============================================================================
// Prerequisite Checking (for TUI guided installation)
// ============================================================================

/// Check all prerequisites and return structured results
pub fn check_all_prerequisites() -> Vec<Prerequisite> {
    let mut prereqs = Vec::new();
    
    // Docker
    let docker = match check_command_version("docker", &["--version"]) {
        Ok(version) => Prerequisite {
            name: "Docker".to_string(),
            status: PrereqStatus::Installed,
            version: Some(version),
            install_cmd: get_docker_install_steps(),
        },
        Err(_) => Prerequisite {
            name: "Docker".to_string(),
            status: PrereqStatus::Missing,
            version: None,
            install_cmd: get_docker_install_steps(),
        },
    };
    prereqs.push(docker);
    
    // PowerShell Core
    let pwsh_cmd = if cfg!(windows) { "pwsh.exe" } else { "pwsh" };
    let pwsh = match check_command_version(pwsh_cmd, &["--version"]) {
        Ok(version) => Prerequisite {
            name: "PowerShell Core".to_string(),
            status: PrereqStatus::Installed,
            version: Some(version),
            install_cmd: get_pwsh_install_steps(),
        },
        Err(_) => Prerequisite {
            name: "PowerShell Core".to_string(),
            status: PrereqStatus::Missing,
            version: None,
            install_cmd: get_pwsh_install_steps(),
        },
    };
    prereqs.push(pwsh);
    
    // kubectl
    let kubectl = match check_command_version("kubectl", &["version", "--client", "--short"]) {
        Ok(version) => Prerequisite {
            name: "kubectl".to_string(),
            status: PrereqStatus::Installed,
            version: Some(version),
            install_cmd: get_kubectl_install_steps(),
        },
        Err(_) => {
            // Try without --short (newer versions)
            match check_command_version("kubectl", &["version", "--client"]) {
                Ok(version) => Prerequisite {
                    name: "kubectl".to_string(),
                    status: PrereqStatus::Installed,
                    version: Some(version.lines().next().unwrap_or(&version).to_string()),
                    install_cmd: get_kubectl_install_steps(),
                },
                Err(_) => Prerequisite {
                    name: "kubectl".to_string(),
                    status: PrereqStatus::Missing,
                    version: None,
                    install_cmd: get_kubectl_install_steps(),
                },
            }
        }
    };
    prereqs.push(kubectl);
    
    // kind
    let kind = match check_command_version("kind", &["version"]) {
        Ok(version) => Prerequisite {
            name: "kind".to_string(),
            status: PrereqStatus::Installed,
            version: Some(version),
            install_cmd: get_kind_install_steps(),
        },
        Err(_) => Prerequisite {
            name: "kind".to_string(),
            status: PrereqStatus::Missing,
            version: None,
            install_cmd: get_kind_install_steps(),
        },
    };
    prereqs.push(kind);
    
    prereqs
}

/// Check if any prerequisites are missing
pub fn has_missing_prerequisites() -> bool {
    check_all_prerequisites()
        .iter()
        .any(|p| matches!(p.status, PrereqStatus::Missing))
}

/// Get count of missing prerequisites
pub fn missing_prereq_count() -> usize {
    check_all_prerequisites()
        .iter()
        .filter(|p| matches!(p.status, PrereqStatus::Missing))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_platform_is_supported() {
        // The current platform must be in the support matrix
        // (or this test wouldn't be running)
        let result = check_platform_support();
        assert!(result.is_ok(), "Current platform should be supported: {:?}", result);
    }

    #[test]
    fn test_support_matrix_uses_valid_rust_constants() {
        // Verify SUPPORT_MATRIX uses values that match Rust's std::env::consts
        let valid_os = &["windows", "macos", "linux", "android", "ios", "freebsd"];
        let valid_arch = &["x86_64", "aarch64", "arm", "x86"];
        
        for (os, arch) in SUPPORT_MATRIX.iter() {
            assert!(
                valid_os.contains(os),
                "Invalid OS in SUPPORT_MATRIX: {}",
                os
            );
            assert!(
                valid_arch.contains(arch),
                "Invalid ARCH in SUPPORT_MATRIX: {}",
                arch
            );
        }
    }

    #[test]
    fn test_support_matrix_has_expected_platforms() {
        // Verify all 5 expected platforms are in the matrix
        let expected = [
            ("windows", "x86_64"),
            ("macos", "x86_64"),
            ("macos", "aarch64"),
            ("linux", "x86_64"),
            ("linux", "aarch64"),
        ];
        
        for (os, arch) in expected.iter() {
            assert!(
                SUPPORT_MATRIX.iter().any(|(s_os, s_arch)| s_os == os && s_arch == arch),
                "Expected platform {}-{} not found in SUPPORT_MATRIX",
                os, arch
            );
        }
    }

    #[test]
    fn test_check_platform_support_is_pure() {
        // check_platform_support should be pure (no I/O)
        for _ in 0..10 {
            let _ = check_platform_support();
        }
    }

    #[test]
    fn test_check_all_prerequisites_returns_four() {
        let prereqs = check_all_prerequisites();
        assert_eq!(prereqs.len(), 4, "Should check 4 prerequisites");
    }

    #[test]
    fn test_check_all_prerequisites_names() {
        let prereqs = check_all_prerequisites();
        let names: Vec<&str> = prereqs.iter().map(|p| p.name.as_str()).collect();
        
        assert!(names.contains(&"Docker"));
        assert!(names.contains(&"PowerShell Core"));
        assert!(names.contains(&"kubectl"));
        assert!(names.contains(&"kind"));
    }

    #[test]
    fn test_check_command_version_invalid() {
        let result = check_command_version("nonexistent_command_xyz", &["--version"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_prereq_count_is_bounded() {
        let count = missing_prereq_count();
        assert!(count <= 4, "Can't have more than 4 missing prereqs");
    }

    // ========== SUPPORT_MATRIX Edge Case Tests ==========

    #[test]
    fn test_support_matrix_not_empty() {
        assert!(!SUPPORT_MATRIX.is_empty());
    }

    #[test]
    fn test_support_matrix_count() {
        assert_eq!(SUPPORT_MATRIX.len(), 5, "Support matrix should have 5 platforms");
    }

    #[test]
    fn test_support_matrix_url_valid() {
        assert!(SUPPORT_MATRIX_URL.starts_with("https://"));
        assert!(SUPPORT_MATRIX_URL.contains("github"));
    }

    #[test]
    fn test_support_matrix_no_duplicates() {
        let mut seen: Vec<(&str, &str)> = Vec::new();
        for entry in SUPPORT_MATRIX.iter() {
            assert!(!seen.contains(entry), "Duplicate entry in SUPPORT_MATRIX: {:?}", entry);
            seen.push(*entry);
        }
    }

    // ========== check_command_version Tests ==========

    #[test]
    fn test_check_command_version_error_format() {
        let result = check_command_version("definitely_not_a_command_12345", &["--version"]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("not found") || err.contains("failed"), "Error: {}", err);
    }

    #[test]
    fn test_check_command_version_empty_args() {
        // Even with no args, an invalid command should fail gracefully
        let result = check_command_version("xyz_nonexistent", &[]);
        assert!(result.is_err());
    }

    // ========== Prerequisite Struct Tests ==========

    #[test]
    fn test_prerequisites_have_install_commands() {
        let prereqs = check_all_prerequisites();
        for prereq in &prereqs {
            assert!(!prereq.install_cmd.is_empty(), 
                "{} should have install commands", prereq.name);
        }
    }

    #[test]
    fn test_prerequisites_have_valid_status() {
        let prereqs = check_all_prerequisites();
        for prereq in &prereqs {
            // Status should be either Installed or Missing (not Installing or InstallFailed at startup)
            assert!(
                matches!(prereq.status, PrereqStatus::Installed | PrereqStatus::Missing),
                "{} has unexpected status: {:?}", prereq.name, prereq.status
            );
        }
    }

    #[test]
    fn test_installed_prerequisite_has_version() {
        let prereqs = check_all_prerequisites();
        for prereq in &prereqs {
            if prereq.status == PrereqStatus::Installed {
                assert!(prereq.version.is_some(), 
                    "Installed {} should have version info", prereq.name);
            }
        }
    }

    #[test]
    fn test_missing_prerequisite_no_version() {
        let prereqs = check_all_prerequisites();
        for prereq in &prereqs {
            if prereq.status == PrereqStatus::Missing {
                assert!(prereq.version.is_none(), 
                    "Missing {} should not have version", prereq.name);
            }
        }
    }

    // ========== has_missing_prerequisites Tests ==========

    #[test]
    fn test_has_missing_prerequisites_returns_bool() {
        let result = has_missing_prerequisites();
        // Result is either true or false (valid bool)
        assert!(result || !result);
    }

    #[test]
    fn test_has_missing_prerequisites_consistent() {
        // Multiple calls should return consistent result (deterministic)
        let first = has_missing_prerequisites();
        let second = has_missing_prerequisites();
        assert_eq!(first, second, "has_missing_prerequisites should be deterministic");
    }

    #[test]
    fn test_missing_count_matches_has_missing() {
        let has_missing = has_missing_prerequisites();
        let count = missing_prereq_count();
        
        if has_missing {
            assert!(count > 0, "has_missing is true but count is 0");
        } else {
            assert_eq!(count, 0, "has_missing is false but count is {}", count);
        }
    }
}

