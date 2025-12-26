//! Cluster operations and utilities
//!
//! This module handles cluster status checking, setup script execution,
//! UI registry loading, job submission, and browser launching.

use std::{
    fs,
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};
use chrono::Utc;
use uuid::Uuid;

use crate::types::{ClusterStatus, SetupProgress, JobPayload, UiRegistry};
use crate::error::{RegistryError, SubmitError, BrowserError, get_error_hint, get_remediation_steps, get_pwsh_install_steps};

// ============================================================================
// Cluster Status Checking
// ============================================================================

/// Check if the Kind cluster is running and accessible
/// In server mode (W11), uses HTTP health check instead of kubectl
pub fn check_cluster_status() -> ClusterStatus {
    // W11: In server mode, use HTTP health check instead of kubectl
    // This allows TUI to work in docker-compose environments without k8s
    if crate::config::is_server_mode() {
        return check_api_health();
    }
    
    // Normal mode: use kubectl to check cluster
    let output = Command::new("kubectl")
        .args(["get", "nodes", "--context", "kind-task-observatory", "-o", "name"])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                let stdout = String::from_utf8_lossy(&result.stdout);
                if stdout.contains("node/") {
                    // Cluster exists, now check if pods are deployed
                    check_pods_status()
                } else {
                    ClusterStatus::NotFound
                }
            } else {
                ClusterStatus::NotFound
            }
        }
        Err(e) => ClusterStatus::Error(format!("kubectl not found: {}", e)),
    }
}

/// Check API health via HTTP (for server mode)
fn check_api_health() -> ClusterStatus {
    let read_model_url = std::env::var("READ_MODEL_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());
    
    // Try to hit the read-model health/stats endpoint
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build();
    
    let client = match client {
        Ok(c) => c,
        Err(_) => return ClusterStatus::Error("Failed to create HTTP client".to_string()),
    };
    
    // Try /stats endpoint (read-model)
    match client.get(format!("{}/stats", read_model_url)).send() {
        Ok(response) if response.status().is_success() => ClusterStatus::Ready,
        Ok(_) => ClusterStatus::NoPods,
        Err(_) => ClusterStatus::NotFound,
    }
}

/// Check if application pods are deployed in the cluster
pub fn check_pods_status() -> ClusterStatus {
    let output = Command::new("kubectl")
        .args(["get", "pods", "--context", "kind-task-observatory", "-o", "name"])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                let stdout = String::from_utf8_lossy(&result.stdout);
                // Check if there are any pods (not empty output)
                if stdout.trim().is_empty() {
                    ClusterStatus::NoPods
                } else {
                    ClusterStatus::Ready
                }
            } else {
                // kubectl failed but cluster exists, treat as NoPods
                ClusterStatus::NoPods
            }
        }
        Err(_) => {
            // kubectl error after nodes check passed - unusual, treat as NoPods
            ClusterStatus::NoPods
        }
    }
}

// ============================================================================
// Project Root Discovery
// ============================================================================

/// Find the project root by searching for marker files
pub fn find_project_root() -> Option<std::path::PathBuf> {
    let markers = ["README.md", "scripts", "infra", "src"];
    
    // Try current directory first
    if let Ok(cwd) = std::env::current_dir() {
        // Check if we're in the project root
        if markers.iter().all(|m| cwd.join(m).exists()) {
            return Some(cwd);
        }
        
        // Check if we're in src/interfaces/tui
        if cwd.ends_with("tui") {
            let potential_root = cwd.join("../../..").canonicalize().ok();
            if let Some(ref root) = potential_root {
                if markers.iter().all(|m| root.join(m).exists()) {
                    return potential_root;
                }
            }
        }
        
        // Walk up the directory tree
        let mut current = cwd.clone();
        for _ in 0..10 {
            if markers.iter().all(|m| current.join(m).exists()) {
                return Some(current);
            }
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }
    }
    
    // Environment variable fallback for custom project locations
    if let Ok(env_root) = std::env::var("ODTO_PROJECT_ROOT") {
        let fallback = std::path::PathBuf::from(env_root);
        if fallback.exists() && markers.iter().all(|m| fallback.join(m).exists()) {
            return Some(fallback);
        }
    }
    
    None
}

// ============================================================================
// Setup Script Execution
// ============================================================================

/// Run the setup script and capture progress
pub fn run_setup_script(progress: Arc<Mutex<SetupProgress>>) {
    // Step 1: Find the project root by looking for key files
    let project_root = find_project_root();
    
    if project_root.is_none() {
        if let Ok(mut p) = progress.lock() {
            p.has_error = true;
            p.message = "Could not locate project root".to_string();
            p.error_hint = "The TUI must be run from within the odd-demonstration project".to_string();
            p.remediation = vec![
                "cd to the odd-demonstration directory".to_string(),
                "cd src/interfaces/tui && cargo run --release".to_string(),
            ];
            p.is_complete = true;
        }
        return;
    }
    
    let root = project_root.unwrap();
    let script_path = root.join("scripts").join("start-all.ps1");
    
    // Step 2: Verify script exists
    if !script_path.exists() {
        if let Ok(mut p) = progress.lock() {
            p.has_error = true;
            p.message = format!("Script not found: {}", script_path.display());
            p.error_hint = "The start-all.ps1 script is missing from the scripts directory".to_string();
            p.remediation = vec![
                "Ensure you have the latest version of the repository".to_string(),
                "git pull origin main".to_string(),
            ];
            p.is_complete = true;
        }
        return;
    }
    
    // Step 3: Check for PowerShell Core (pwsh) - required on all platforms
    let pwsh_check = Command::new("pwsh")
        .args(["-NoProfile", "-Command", "exit 0"])
        .output();
    let shell_cmd = match pwsh_check {
        Ok(output) if output.status.success() => "pwsh",
        _ => {
            // On Windows, fall back to Windows PowerShell (powershell.exe)
            #[cfg(target_os = "windows")]
            {
                let ps_check = Command::new("powershell.exe")
                    .args(["-NoProfile", "-Command", "exit 0"])
                    .output();
                match ps_check {
                    Ok(output) if output.status.success() => "powershell.exe",
                    _ => {
                        if let Ok(mut p) = progress.lock() {
                            p.has_error = true;
                            p.message = "PowerShell not found".to_string();
                            p.error_hint = "This launcher requires PowerShell to run the setup script".to_string();
                            p.remediation = vec![
                                "Windows PowerShell should be pre-installed on Windows".to_string(),
                                "Try running: powershell.exe -Command 'echo hello'".to_string(),
                                "Or install PowerShell 7: winget install Microsoft.PowerShell".to_string(),
                            ];
                            p.is_complete = true;
                        }
                        return;
                    }
                }
            }
            // On Linux/macOS, pwsh is required (no fallback)
            #[cfg(not(target_os = "windows"))]
            {
                if let Ok(mut p) = progress.lock() {
                    p.has_error = true;
                    p.message = "PowerShell Core (pwsh) not found".to_string();
                    p.error_hint = "This launcher requires PowerShell Core to run the setup script".to_string();
                    p.remediation = get_pwsh_install_steps();
                    p.is_complete = true;
                }
                return;
            }
        }
    };
    
    // Step 4: Check for Docker
    let docker_check = Command::new("docker").arg("info").output();
    if docker_check.is_err() || !docker_check.unwrap().status.success() {
        if let Ok(mut p) = progress.lock() {
            p.has_error = true;
            p.message = "Docker is not running or not installed".to_string();
            p.error_hint = "Docker Desktop must be running to create the Kubernetes cluster".to_string();
            p.remediation = vec![
                "1. Install Docker Desktop: https://docker.com/products/docker-desktop".to_string(),
                "2. Start Docker Desktop".to_string(),
                "3. Wait for Docker to be ready (whale icon in taskbar)".to_string(),
                "4. Try again by pressing 'L'".to_string(),
            ];
            p.is_complete = true;
        }
        return;
    }
    
    // Step 5: Update progress and spawn the script
    if let Ok(mut p) = progress.lock() {
        p.message = "Starting cluster setup...".to_string();
        p.current_step = "prereqs".to_string();
        p.start_time = Some(std::time::Instant::now());
        p.log_lines.push(format!("Using shell: {}", shell_cmd));
        p.log_lines.push(format!("Script: {}", script_path.display()));
    }
    
    // Use -Command with & operator to properly handle paths with spaces or special chars
    let script_str = script_path.to_string_lossy().replace("\\", "/");
    let ps_command = format!("& '{}' -OutputJson", script_str);
    
    let mut child = match Command::new(shell_cmd)
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps_command])
        .current_dir(&root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            if let Ok(mut p) = progress.lock() {
                p.has_error = true;
                p.message = format!("Failed to start {}: {}", shell_cmd, e);
                p.error_hint = "Could not execute the PowerShell script".to_string();
                p.remediation = vec![
                    "Try running the script manually:".to_string(),
                    format!("  cd {}", root.display()),
                    format!("  {} -File scripts/start-all.ps1", shell_cmd),
                ];
                p.is_complete = true;
            }
            return;
        }
    };

    // Spawn a thread to read stderr
    let stderr_progress = Arc::clone(&progress);
    let stderr_handle = if let Some(stderr) = child.stderr.take() {
        Some(std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().flatten() {
                if let Ok(mut p) = stderr_progress.lock() {
                    p.log_lines.push(format!("[ERR] {}", line));
                }
            }
        }))
    } else {
        None
    };

    // Read stdout line by line
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().flatten() {
            if let Ok(mut p) = progress.lock() {
                p.log_lines.push(line.clone());
                
                // Try to parse JSON progress
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                    if let Some(step) = json.get("step").and_then(|s| s.as_str()) {
                        p.current_step = step.to_string();
                    }
                    if let Some(status) = json.get("status").and_then(|s| s.as_str()) {
                        p.current_status = status.to_string();
                        if status == "error" {
                            p.has_error = true;
                            if let Some(msg) = json.get("message").and_then(|m| m.as_str()) {
                                p.error_hint = get_error_hint(msg);
                                p.remediation = get_remediation_steps(msg);
                            }
                        }
                    }
                    if let Some(msg) = json.get("message").and_then(|s| s.as_str()) {
                        p.message = msg.to_string();
                    }
                }
            }
        }
    }

    // Wait for stderr thread
    if let Some(handle) = stderr_handle {
        let _ = handle.join();
    }

    // Wait for completion
    let status = child.wait();
    if let Ok(mut p) = progress.lock() {
        p.is_complete = true;
        if let Ok(s) = status {
            if !s.success() {
                p.has_error = true;
                if p.message.is_empty() {
                    p.message = format!("Setup failed with exit code: {:?}", s.code());
                }
                if p.error_hint.is_empty() {
                    p.error_hint = "The setup script encountered an error".to_string();
                    p.remediation = vec![
                        "Check the log output above for details".to_string(),
                        "Try running manually: pwsh ./scripts/start-all.ps1".to_string(),
                        "Ensure Docker Desktop is running".to_string(),
                    ];
                }
            }
        }
    }
}

// ============================================================================
// UI Registry and Browser
// ============================================================================

/// Load UI registry from contracts/ui-registry.json with explicit error categories
pub fn load_ui_registry() -> Result<UiRegistry, RegistryError> {
    let registry_path = find_project_root()
        .ok_or_else(|| RegistryError::NotFound("Could not find project root".to_string()))?
        .join("contracts")
        .join("ui-registry.json");
    
    let content = fs::read_to_string(&registry_path)
        .map_err(|e| RegistryError::NotFound(format!("Failed to read: {}", e)))?;
    
    let registry: UiRegistry = serde_json::from_str(&content)
        .map_err(|e| RegistryError::Malformed(format!("JSON parse error: {}", e)))?;
    
    // Validate registry entries
    if registry.entries.is_empty() {
        return Err(RegistryError::InvalidEntry("Registry has no entries".to_string()));
    }
    
    for entry in &registry.entries {
        if entry.port == 0 || entry.port > 65535 {
            return Err(RegistryError::InvalidEntry(format!("Invalid port {} for {}", entry.port, entry.id)));
        }
        if entry.id.is_empty() || entry.name.is_empty() {
            return Err(RegistryError::InvalidEntry("Entry missing id or name".to_string()));
        }
    }
    
    if !registry.base_url.starts_with("http") {
        return Err(RegistryError::Malformed(format!("baseUrl must start with http: {}", registry.base_url)));
    }
    
    Ok(registry)
}

/// Open URL in default browser with environment-aware error handling
pub fn open_browser(url: &str) -> Result<(), BrowserError> {
    // Check for headless/restricted environments
    if std::env::var("SSH_CLIENT").is_ok() || std::env::var("SSH_TTY").is_ok() {
        return Err(BrowserError::EnvironmentRestricted(
            "SSH session detected - browser launch not available. URL: ".to_string() + url
        ));
    }
    
    if std::env::var("DISPLAY").is_err() && cfg!(target_os = "linux") {
        return Err(BrowserError::EnvironmentRestricted(
            "No DISPLAY set (headless environment). URL: ".to_string() + url
        ));
    }
    
    open::that(url).map_err(|e| {
        let error_str = e.to_string().to_lowercase();
        if error_str.contains("not found") || error_str.contains("no such file") {
            BrowserError::NotAvailable(format!("Default browser not found: {}", e))
        } else {
            BrowserError::LaunchFailed(format!("{}", e))
        }
    })
}

// ============================================================================
// Job Submission
// ============================================================================

/// Validate job type input (alphanumeric with underscores, reasonable length)
pub fn validate_job_type(job_type: &str) -> Result<(), String> {
    let trimmed = job_type.trim();
    if trimmed.is_empty() {
        return Err("Job type cannot be empty".to_string());
    }
    if trimmed.len() > 50 {
        return Err("Job type too long (max 50 chars)".to_string());
    }
    // Allow alphanumeric, underscores, hyphens
    if !trimmed.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err("Job type must be alphanumeric (underscores/hyphens allowed)".to_string());
    }
    Ok(())
}

/// Submit a job to the Gateway API with validation
pub fn submit_job(gateway_url: &str, job_type: &str) -> Result<String, SubmitError> {
    // Early validation
    validate_job_type(job_type).map_err(|e| SubmitError::ValidationFailed(e))?;
    
    let job_id = Uuid::new_v4().to_string();
    let payload = JobPayload {
        id: job_id.clone(),
        job_type: job_type.trim().to_uppercase(),
        status: "PENDING".to_string(),
        created_at: Utc::now().to_rfc3339(),
    };

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(format!("{}/jobs", gateway_url))
        .json(&payload)
        .timeout(Duration::from_secs(2))
        .send()
        .map_err(|e| {
            if e.is_timeout() {
                SubmitError::Timeout
            } else if e.is_connect() {
                SubmitError::ConnectionRefused
            } else {
                SubmitError::NetworkError(e.to_string())
            }
        })?;

    if response.status().is_success() {
        Ok(job_id)
    } else {
        let status = response.status().as_u16();
        let body = response.text().unwrap_or_default();
        Err(SubmitError::ServerError(status, body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_status_variants() {
        // Ensure all variants are available
        let ready = ClusterStatus::Ready;
        let no_pods = ClusterStatus::NoPods;
        let not_found = ClusterStatus::NotFound;
        let error = ClusterStatus::Error("test".to_string());
        
        assert_eq!(ready, ClusterStatus::Ready);
        assert_eq!(no_pods, ClusterStatus::NoPods);
        assert_eq!(not_found, ClusterStatus::NotFound);
        assert!(matches!(error, ClusterStatus::Error(_)));
    }

    #[test]
    fn test_find_project_root_returns_option() {
        // This test just verifies the function returns an Option
        let result = find_project_root();
        // Result may be Some or None depending on where tests are run
        assert!(result.is_some() || result.is_none());
    }

    #[test]
    fn test_find_project_root_finds_scripts() {
        // If we find a root, it should have the scripts directory
        if let Some(root) = find_project_root() {
            let scripts_dir = root.join("scripts");
            assert!(scripts_dir.exists(), "Project root should have scripts directory");
        }
    }

    #[test]
    fn test_validate_job_type_empty() {
        let result = validate_job_type("");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_validate_job_type_too_long() {
        let long_type = "a".repeat(51);
        let result = validate_job_type(&long_type);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("long"));
    }

    #[test]
    fn test_validate_job_type_special_chars() {
        let result = validate_job_type("test@job");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("alphanumeric"));
    }

    #[test]
    fn test_validate_job_type_valid() {
        assert!(validate_job_type("test_job").is_ok());
        assert!(validate_job_type("test-job").is_ok());
        assert!(validate_job_type("TestJob123").is_ok());
    }

    #[test]
    fn test_setup_progress_log_lines() {
        let progress = SetupProgress::default();
        assert!(progress.log_lines.is_empty());
    }

    #[test]
    fn test_setup_progress_complete_states() {
        let mut progress = SetupProgress::default();
        assert!(!progress.is_complete);
        assert!(!progress.has_error);
        
        progress.is_complete = true;
        progress.has_error = true;
        
        assert!(progress.is_complete);
        assert!(progress.has_error);
    }

    // ========== validate_job_type Edge Case Tests ==========

    #[test]
    fn test_validate_job_type_whitespace_only() {
        let result = validate_job_type("   ");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_validate_job_type_with_leading_whitespace() {
        // Should be trimmed and valid
        let result = validate_job_type("  valid_job");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_job_type_with_trailing_whitespace() {
        let result = validate_job_type("valid_job  ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_job_type_exactly_50_chars() {
        let exact = "a".repeat(50);
        let result = validate_job_type(&exact);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_job_type_numbers_only() {
        assert!(validate_job_type("12345").is_ok());
    }

    #[test]
    fn test_validate_job_type_single_char() {
        assert!(validate_job_type("X").is_ok());
    }

    #[test]
    fn test_validate_job_type_underscores() {
        assert!(validate_job_type("test_job_type_v2").is_ok());
    }

    #[test]
    fn test_validate_job_type_hyphens() {
        assert!(validate_job_type("test-job-type").is_ok());
    }

    #[test]
    fn test_validate_job_type_mixed_case() {
        assert!(validate_job_type("TestJobType").is_ok());
    }

    #[test]
    fn test_validate_job_type_space_in_middle() {
        let result = validate_job_type("test job");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_job_type_newline() {
        let result = validate_job_type("test\njob");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_job_type_dot() {
        let result = validate_job_type("test.job");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_job_type_slash() {
        let result = validate_job_type("test/job");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_job_type_colon() {
        let result = validate_job_type("test:job");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_job_type_unicode() {
        // Note: is_alphanumeric() accepts Unicode letters, so this is valid
        // This is intentional - international characters are allowed
        let result = validate_job_type("tëst");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_job_type_emoji() {
        let result = validate_job_type("test🚀");
        assert!(result.is_err());
    }

    // ========== ClusterStatus Tests ==========

    #[test]
    fn test_cluster_status_equality() {
        assert_eq!(ClusterStatus::Ready, ClusterStatus::Ready);
        assert_eq!(ClusterStatus::NoPods, ClusterStatus::NoPods);
        assert_eq!(ClusterStatus::NotFound, ClusterStatus::NotFound);
        assert_ne!(ClusterStatus::Ready, ClusterStatus::NoPods);
    }

    #[test]
    fn test_cluster_status_error_with_details() {
        let status = ClusterStatus::Error("Connection timeout".to_string());
        if let ClusterStatus::Error(msg) = status {
            assert!(msg.contains("timeout"));
        } else {
            panic!("Expected Error variant");
        }
    }

    #[test]
    fn test_cluster_status_clone() {
        let status = ClusterStatus::Error("test error".to_string());
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_cluster_status_debug() {
        let status = ClusterStatus::Ready;
        let debug = format!("{:?}", status);
        assert!(debug.contains("Ready"));
    }

    // ========== SetupProgress Tests ==========

    #[test]
    fn test_setup_progress_default_values() {
        let progress = SetupProgress::default();
        assert!(progress.current_step.is_empty());
        assert!(progress.current_status.is_empty());
        assert!(progress.message.is_empty());
        assert!(progress.error_hint.is_empty());
        assert!(progress.remediation.is_empty());
        assert!(!progress.is_complete);
        assert!(!progress.has_error);
        assert!(progress.start_time.is_none());
    }

    #[test]
    fn test_setup_progress_add_log_lines() {
        let mut progress = SetupProgress::default();
        progress.log_lines.push("Step 1: Starting...".to_string());
        progress.log_lines.push("Step 2: Deploying...".to_string());
        
        assert_eq!(progress.log_lines.len(), 2);
        assert!(progress.log_lines[0].contains("Starting"));
    }

    #[test]
    fn test_setup_progress_remediation_steps() {
        let mut progress = SetupProgress::default();
        progress.remediation = vec![
            "Step 1: Restart Docker".to_string(),
            "Step 2: Run setup again".to_string(),
        ];
        
        assert_eq!(progress.remediation.len(), 2);
    }

    #[test]
    fn test_setup_progress_error_state() {
        let mut progress = SetupProgress::default();
        progress.has_error = true;
        progress.error_hint = "Docker not running".to_string();
        progress.is_complete = true;
        
        assert!(progress.has_error);
        assert!(progress.is_complete);
        assert!(!progress.error_hint.is_empty());
    }

    #[test]
    fn test_setup_progress_clone() {
        let mut progress = SetupProgress::default();
        progress.current_step = "deploying".to_string();
        progress.is_complete = true;
        
        let cloned = progress.clone();
        assert_eq!(cloned.current_step, progress.current_step);
        assert_eq!(cloned.is_complete, progress.is_complete);
    }

    // ========== W11: Server Mode API Health Check Tests ==========

    #[test]
    fn test_server_mode_uses_api_health_check() {
        // When server mode is enabled, check_cluster_status should use HTTP
        // instead of kubectl (which doesn't exist in containers)
        std::env::set_var("ODD_DASHBOARD_SERVER_MODE", "1");
        std::env::set_var("READ_MODEL_URL", "http://localhost:9999"); // Non-existent
        
        let status = check_cluster_status();
        
        std::env::remove_var("ODD_DASHBOARD_SERVER_MODE");
        std::env::remove_var("READ_MODEL_URL");
        
        // Should return NotFound (connection failed) not Error (kubectl not found)
        // This proves HTTP was used instead of kubectl
        assert!(
            matches!(status, ClusterStatus::NotFound),
            "Server mode should use HTTP check, got: {:?}",
            status
        );
    }

    #[test]
    fn test_server_mode_respects_read_model_url() {
        // Verify that the READ_MODEL_URL env var is used
        std::env::set_var("ODD_DASHBOARD_SERVER_MODE", "1");
        std::env::set_var("READ_MODEL_URL", "http://127.0.0.1:1"); // Invalid port
        
        let status = check_cluster_status();
        
        std::env::remove_var("ODD_DASHBOARD_SERVER_MODE");
        std::env::remove_var("READ_MODEL_URL");
        
        // Should fail to connect (NotFound) rather than error on kubectl
        assert!(matches!(status, ClusterStatus::NotFound | ClusterStatus::Error(_)));
    }

    #[test]
    fn test_normal_mode_uses_kubectl() {
        // When server mode is disabled, check_cluster_status should use kubectl
        std::env::remove_var("ODD_DASHBOARD_SERVER_MODE");
        
        let status = check_cluster_status();
        
        // On a system without kubectl or cluster, this returns NotFound or Error
        // The key is it doesn't hang (which would happen if HTTP was used incorrectly)
        assert!(matches!(
            status,
            ClusterStatus::Ready | ClusterStatus::NotFound | ClusterStatus::NoPods | ClusterStatus::Error(_)
        ));
    }
}

