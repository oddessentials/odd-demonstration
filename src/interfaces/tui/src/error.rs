//! Error handling and remediation functions
//!
//! This module contains error types and functions for generating
//! user-friendly error messages and platform-specific remediation steps.

// ============================================================================
// Error Types
// ============================================================================

/// Registry load error categories for consistent handling
#[derive(Debug, Clone, PartialEq)]
pub enum RegistryError {
    NotFound(String),
    Malformed(String),
    InvalidEntry(String),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::NotFound(msg) => write!(f, "Registry not found: {}", msg),
            RegistryError::Malformed(msg) => write!(f, "Registry malformed: {}", msg),
            RegistryError::InvalidEntry(msg) => write!(f, "Invalid entry: {}", msg),
        }
    }
}

/// Job submission error categories
#[derive(Debug, Clone, PartialEq)]
pub enum SubmitError {
    Timeout,
    ConnectionRefused,
    ValidationFailed(String),
    ServerError(u16, String),
    NetworkError(String),
}

impl std::fmt::Display for SubmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubmitError::Timeout => write!(f, "Gateway timeout (2s) - cluster may be starting or unavailable"),
            SubmitError::ConnectionRefused => write!(f, "Cannot connect to Gateway - ensure cluster is running"),
            SubmitError::ValidationFailed(msg) => write!(f, "Validation failed: {}", msg),
            SubmitError::ServerError(code, body) => write!(f, "Gateway returned {}: {}", code, body),
            SubmitError::NetworkError(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

/// Browser launch error categories
#[derive(Debug, Clone, PartialEq)]
pub enum BrowserError {
    NotAvailable(String),
    EnvironmentRestricted(String),
    LaunchFailed(String),
}

impl std::fmt::Display for BrowserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrowserError::NotAvailable(msg) => write!(f, "No browser available: {}", msg),
            BrowserError::EnvironmentRestricted(msg) => write!(f, "Environment restriction: {}", msg),
            BrowserError::LaunchFailed(msg) => write!(f, "Launch failed: {}", msg),
        }
    }
}

// ============================================================================
// Error Hint Functions
// ============================================================================

/// Get a user-friendly hint based on the error message
pub fn get_error_hint(message: &str) -> String {
    let msg_lower = message.to_lowercase();
    
    if msg_lower.contains("docker") {
        "Docker Desktop may not be running".to_string()
    } else if msg_lower.contains("kind") || msg_lower.contains("cluster") {
        "Kind cluster tool may need to be installed".to_string()
    } else if msg_lower.contains("kubectl") {
        "kubectl may need to be installed".to_string()
    } else if msg_lower.contains("timeout") || msg_lower.contains("timed out") {
        "Operation timed out - services may be slow to start".to_string()
    } else if msg_lower.contains("port") || msg_lower.contains("address already in use") {
        "Port conflict detected - another application may be using the port".to_string()
    } else if msg_lower.contains("permission") || msg_lower.contains("access denied") {
        "Permission issue - may need elevated privileges".to_string()
    } else {
        "An unexpected error occurred".to_string()
    }
}

/// Get remediation steps based on the error message (cross-platform)
pub fn get_remediation_steps(message: &str) -> Vec<String> {
    let msg_lower = message.to_lowercase();
    
    if msg_lower.contains("docker") {
        vec![
            "1. Open Docker Desktop".to_string(),
            "2. Wait for it to fully start (whale icon stable)".to_string(),
            "3. Press any key, then 'L' to retry".to_string(),
        ]
    } else if msg_lower.contains("kind") {
        get_kind_install_steps()
    } else if msg_lower.contains("kubectl") {
        get_kubectl_install_steps()
    } else if msg_lower.contains("port") {
        get_port_conflict_steps()
    } else {
        get_generic_error_steps()
    }
}

// ============================================================================
// Platform-Specific Installation Steps
// ============================================================================

/// Get platform-specific Docker Desktop installation steps
pub fn get_docker_install_steps() -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        vec![
            "Install Docker Desktop:".to_string(),
            "  1. Download from: https://www.docker.com/products/docker-desktop".to_string(),
            "  2. Open the .dmg and drag Docker to Applications".to_string(),
            "  3. Launch Docker Desktop and wait for it to start".to_string(),
            "  Or via Homebrew: brew install --cask docker".to_string(),
        ]
    }
    #[cfg(target_os = "linux")]
    {
        vec![
            "Install Docker:".to_string(),
            "  Ubuntu/Debian: sudo apt-get install docker.io".to_string(),
            "  Fedora: sudo dnf install docker".to_string(),
            "  Start: sudo systemctl start docker".to_string(),
            "  Add user to docker group: sudo usermod -aG docker $USER".to_string(),
        ]
    }
    #[cfg(target_os = "windows")]
    {
        vec![
            "Install Docker Desktop:".to_string(),
            "  winget install Docker.DockerDesktop".to_string(),
            "Or download from: https://www.docker.com/products/docker-desktop".to_string(),
            "After install, restart and ensure Docker Desktop is running".to_string(),
        ]
    }
}

/// Get platform-specific pwsh installation steps
pub fn get_pwsh_install_steps() -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        vec![
            "Install PowerShell Core via Homebrew:".to_string(),
            "  brew install powershell".to_string(),
            "Then restart your terminal and retry".to_string(),
        ]
    }
    #[cfg(target_os = "linux")]
    {
        vec![
            "Install PowerShell Core:".to_string(),
            "  Ubuntu/Debian: sudo apt-get install -y powershell".to_string(),
            "  Fedora: sudo dnf install -y powershell".to_string(),
            "  Or: https://aka.ms/install-powershell".to_string(),
            "Then restart your terminal and retry".to_string(),
        ]
    }
    #[cfg(target_os = "windows")]
    {
        vec![
            "Install PowerShell 7:".to_string(),
            "  winget install Microsoft.PowerShell".to_string(),
            "Then restart your terminal and retry".to_string(),
        ]
    }
}

/// Get platform-specific kind installation steps
pub fn get_kind_install_steps() -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        vec![
            "Install kind via Homebrew:".to_string(),
            "  brew install kind".to_string(),
            "Restart terminal and retry".to_string(),
        ]
    }
    #[cfg(target_os = "linux")]
    {
        vec![
            "Install kind:".to_string(),
            "  curl -Lo ./kind https://kind.sigs.k8s.io/dl/v0.20.0/kind-linux-amd64".to_string(),
            "  chmod +x ./kind && sudo mv ./kind /usr/local/bin/kind".to_string(),
            "  Or: go install sigs.k8s.io/kind@latest".to_string(),
            "Restart terminal and retry".to_string(),
        ]
    }
    #[cfg(target_os = "windows")]
    {
        vec![
            "Install kind: winget install Kubernetes.kind".to_string(),
            "Or: choco install kind".to_string(),
            "Restart terminal and retry".to_string(),
        ]
    }
}

/// Get platform-specific kubectl installation steps
pub fn get_kubectl_install_steps() -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        vec![
            "Install kubectl via Homebrew:".to_string(),
            "  brew install kubectl".to_string(),
            "Restart terminal and retry".to_string(),
        ]
    }
    #[cfg(target_os = "linux")]
    {
        vec![
            "Install kubectl:".to_string(),
            "  curl -LO \"https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl\"".to_string(),
            "  chmod +x kubectl && sudo mv kubectl /usr/local/bin/".to_string(),
            "  Or via snap: sudo snap install kubectl --classic".to_string(),
            "Restart terminal and retry".to_string(),
        ]
    }
    #[cfg(target_os = "windows")]
    {
        vec![
            "Install kubectl: winget install Kubernetes.kubectl".to_string(),
            "Or: choco install kubernetes-cli".to_string(),
            "Restart terminal and retry".to_string(),
        ]
    }
}

/// Get platform-specific port conflict steps
pub fn get_port_conflict_steps() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        vec![
            "Check for conflicting applications:".to_string(),
            "  netstat -ano | findstr :3000".to_string(),
            "  netstat -ano | findstr :8080".to_string(),
            "Stop the conflicting application and retry".to_string(),
        ]
    }
    #[cfg(not(target_os = "windows"))]
    {
        vec![
            "Check for conflicting applications:".to_string(),
            "  lsof -i :3000".to_string(),
            "  lsof -i :8080".to_string(),
            "Stop the conflicting application and retry".to_string(),
        ]
    }
}

/// Get generic error steps (cross-platform)
pub fn get_generic_error_steps() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        vec![
            "Review the error messages above".to_string(),
            "Try running: .\\scripts\\start-all.ps1".to_string(),
            "Check Docker Desktop is running".to_string(),
        ]
    }
    #[cfg(not(target_os = "windows"))]
    {
        vec![
            "Review the error messages above".to_string(),
            "Try running: pwsh ./scripts/start-all.ps1".to_string(),
            "Check Docker Desktop is running".to_string(),
        ]
    }
}

/// Get the primary install command for a prerequisite (for clipboard/execute)
pub fn get_install_command(prereq_name: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        match prereq_name.to_lowercase().as_str() {
            "docker" | "docker desktop" => Some("brew install --cask docker".to_string()),
            "powershell" | "powershell core" | "pwsh" => Some("brew install powershell".to_string()),
            "kubectl" => Some("brew install kubectl".to_string()),
            "kind" => Some("brew install kind".to_string()),
            _ => None,
        }
    }
    #[cfg(target_os = "linux")]
    {
        match prereq_name.to_lowercase().as_str() {
            "docker" | "docker desktop" => Some("sudo apt-get install -y docker.io".to_string()),
            "powershell" | "powershell core" | "pwsh" => Some("sudo apt-get install -y powershell".to_string()),
            "kubectl" => Some("sudo snap install kubectl --classic".to_string()),
            "kind" => Some("curl -Lo ./kind https://kind.sigs.k8s.io/dl/v0.20.0/kind-linux-amd64 && chmod +x ./kind && sudo mv ./kind /usr/local/bin/kind".to_string()),
            _ => None,
        }
    }
    #[cfg(target_os = "windows")]
    {
        match prereq_name.to_lowercase().as_str() {
            "docker" | "docker desktop" => Some("winget install Docker.DockerDesktop".to_string()),
            "powershell" | "powershell core" | "pwsh" => Some("winget install Microsoft.PowerShell".to_string()),
            "kubectl" => Some("winget install Kubernetes.kubectl".to_string()),
            "kind" => Some("winget install Kubernetes.kind".to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_error_hint_docker() {
        let hint = get_error_hint("Docker daemon not running");
        assert!(hint.contains("Docker"));
    }

    #[test]
    fn test_get_error_hint_kind() {
        let hint = get_error_hint("kind cluster creation failed");
        assert!(hint.contains("Kind") || hint.contains("cluster"));
    }

    #[test]
    fn test_get_error_hint_kubectl() {
        let hint = get_error_hint("kubectl: command not found");
        assert!(hint.contains("kubectl"));
    }

    #[test]
    fn test_get_error_hint_timeout() {
        let hint = get_error_hint("operation timed out");
        assert!(hint.contains("timeout") || hint.contains("timed out"));
    }

    #[test]
    fn test_get_error_hint_port() {
        let hint = get_error_hint("address already in use: port 3000");
        assert!(hint.contains("port") || hint.contains("Port"));
    }

    #[test]
    fn test_get_error_hint_permission() {
        let hint = get_error_hint("permission denied");
        assert!(hint.contains("ermission"));
    }

    #[test]
    fn test_get_error_hint_unknown() {
        let hint = get_error_hint("something weird happened");
        assert!(hint.contains("unexpected"));
    }

    #[test]
    fn test_get_remediation_steps_docker() {
        let steps = get_remediation_steps("Docker not running");
        assert!(!steps.is_empty());
        assert!(steps.iter().any(|s| s.contains("Docker")));
    }

    #[test]
    fn test_get_remediation_steps_kind() {
        let steps = get_remediation_steps("kind: command not found");
        assert!(!steps.is_empty());
    }

    #[test]
    fn test_get_remediation_steps_port_conflict() {
        let steps = get_remediation_steps("port 3000 already in use");
        assert!(!steps.is_empty());
    }

    #[test]
    fn test_get_remediation_steps_generic() {
        let steps = get_remediation_steps("some random error");
        assert!(!steps.is_empty());
    }

    #[test]
    fn test_registry_error_display() {
        let not_found = RegistryError::NotFound("file.json".to_string());
        let malformed = RegistryError::Malformed("JSON parse".to_string());
        let invalid = RegistryError::InvalidEntry("missing id".to_string());
        
        assert!(not_found.to_string().contains("not found"));
        assert!(malformed.to_string().contains("malformed"));
        assert!(invalid.to_string().contains("Invalid"));
    }

    #[test]
    fn test_submit_error_display() {
        let timeout = SubmitError::Timeout;
        let conn = SubmitError::ConnectionRefused;
        let validation = SubmitError::ValidationFailed("bad input".to_string());
        let server = SubmitError::ServerError(400, "bad request".to_string());
        
        assert!(timeout.to_string().contains("timeout"));
        assert!(conn.to_string().contains("connect"));
        assert!(validation.to_string().contains("Validation"));
        assert!(server.to_string().contains("400"));
    }

    #[test]
    fn test_browser_error_display() {
        let restricted = BrowserError::EnvironmentRestricted("SSH".to_string());
        let not_avail = BrowserError::NotAvailable("no browser".to_string());
        let failed = BrowserError::LaunchFailed("error".to_string());
        
        assert!(restricted.to_string().contains("Environment"));
        assert!(not_avail.to_string().contains("available"));
        assert!(failed.to_string().contains("failed"));
    }

    #[test]
    fn test_get_install_command() {
        // Test that at least some commands are returned
        let docker_cmd = get_install_command("docker");
        assert!(docker_cmd.is_some());
        
        let kubectl_cmd = get_install_command("kubectl");
        assert!(kubectl_cmd.is_some());
        
        let unknown_cmd = get_install_command("unknown_tool");
        assert!(unknown_cmd.is_none());
    }

    #[test]
    fn test_get_docker_install_steps() {
        let steps = get_docker_install_steps();
        assert!(!steps.is_empty());
        assert!(steps.iter().any(|s| s.to_lowercase().contains("docker")));
    }

    #[test]
    fn test_get_pwsh_install_steps() {
        let steps = get_pwsh_install_steps();
        assert!(!steps.is_empty());
    }

    #[test]
    fn test_get_kind_install_steps() {
        let steps = get_kind_install_steps();
        assert!(!steps.is_empty());
    }

    #[test]
    fn test_get_kubectl_install_steps() {
        let steps = get_kubectl_install_steps();
        assert!(!steps.is_empty());
    }
}
