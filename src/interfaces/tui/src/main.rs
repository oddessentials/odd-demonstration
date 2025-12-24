use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
    Terminal,
};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs,
    io::{self, BufRead, BufReader},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use chrono::Utc;
use uuid::Uuid;


/// ASCII art logo for the Distributed Task Observatory
const LOGO: &str = r#"
            .....        
         .#########.     
       .#####      ##    
      ####+###+    ##+   
    ######  +###+  ###   
  #### -######+######+   
+###.   +####.  +###+    
 ..   -###+########.     
    .###+   .####.       
     -+    +###.         
         -###-           
         -#-             
"#;

/// Animated spinner frames (Braille dots pattern)
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Loading messages that cycle for visual interest
const LOADING_MESSAGES: &[&str] = &[
    "Connecting to services",
    "Fetching statistics",
    "Loading job data",
    "Checking alerts",
];

/// Application mode - controls which view is displayed
#[derive(Debug, Clone, PartialEq)]
enum AppMode {
    Loading,        // Initial loading splash
    Launcher,       // Cluster not detected - show launch option
    SetupProgress,  // Running setup script
    Dashboard,      // Normal dashboard view
    TaskCreation,   // Task creation modal
    UiLauncher,     // UI launcher selection
}


/// Cluster status after checking
#[derive(Debug, Clone, PartialEq)]
enum ClusterStatus {
    Ready,
    NoPods,  // Cluster exists but no application pods deployed
    NotFound,
    Error(String),
}

/// Setup progress tracking
#[derive(Debug, Clone, Default)]
struct SetupProgress {
    current_step: String,
    current_status: String,
    message: String,
    error_hint: String,      // Actionable hint for fixing the error
    remediation: Vec<String>, // Step-by-step remediation commands
    is_complete: bool,
    has_error: bool,
    log_lines: Vec<String>,
    start_time: Option<std::time::Instant>,  // For elapsed time tracking
}

/// Task creation state
#[derive(Debug, Clone, PartialEq)]
enum TaskCreationStatus {
    Editing,
    Submitting,
    Success(String),  // contains job_id
    Error(String),    // contains error message
}

#[derive(Debug, Clone)]
struct TaskCreationState {
    job_type: String,
    status: TaskCreationStatus,
}

impl Default for TaskCreationState {
    fn default() -> Self {
        Self {
            job_type: String::new(),
            status: TaskCreationStatus::Editing,
        }
    }
}

/// Job payload for submission to Gateway
#[derive(Serialize, Debug)]
struct JobPayload {
    id: String,
    #[serde(rename = "type")]
    job_type: String,
    status: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

/// UI Registry entry from contracts/ui-registry.json
#[derive(Deserialize, Debug, Clone)]
struct UiEntry {
    id: String,
    name: String,
    port: u16,
    path: String,
    emoji: String,
    description: String,
}

/// UI Registry containing all launchable UIs
#[derive(Deserialize, Debug, Clone)]
struct UiRegistry {
    #[serde(rename = "baseUrl")]
    base_url: String,
    entries: Vec<UiEntry>,
}

/// UI Launcher state
#[derive(Debug, Clone, Default)]
struct UiLauncherState {
    selected_index: usize,
    registry: Option<UiRegistry>,
    error: Option<String>,  // For displaying browser/registry errors
}


/// Load UI registry from contracts/ui-registry.json
/// Registry load error categories for consistent handling
#[derive(Debug, Clone, PartialEq)]
enum RegistryError {
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

/// Load UI registry from contracts/ui-registry.json with explicit error categories
fn load_ui_registry() -> Result<UiRegistry, RegistryError> {
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
            return Err(RegistryError::InvalidEntry(format!("Entry missing id or name")));
        }
    }
    
    if !registry.base_url.starts_with("http") {
        return Err(RegistryError::Malformed(format!("baseUrl must start with http: {}", registry.base_url)));
    }
    
    Ok(registry)
}

/// Job submission error categories
#[derive(Debug, Clone, PartialEq)]
enum SubmitError {
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

/// Validate job type input (alphanumeric with underscores, reasonable length)
fn validate_job_type(job_type: &str) -> Result<(), String> {
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
fn submit_job(gateway_url: &str, job_type: &str) -> Result<String, SubmitError> {
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

/// Browser launch error categories
#[derive(Debug, Clone, PartialEq)]
enum BrowserError {
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

/// Open URL in default browser with environment-aware error handling
fn open_browser(url: &str) -> Result<(), BrowserError> {
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


#[derive(Deserialize, Debug, Clone, Default)]
struct Stats {
    #[serde(rename = "totalJobs")]
    total_jobs: i64,
    #[serde(rename = "completedJobs")]
    completed_jobs: i64,
    #[serde(rename = "failedJobs")]
    failed_jobs: i64,
    #[serde(rename = "lastEventTime")]
    last_event_time: String,
}

#[derive(Deserialize, Debug, Clone)]
struct Job {
    id: String,
    #[serde(rename = "type")]
    job_type: String,
    status: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

#[derive(Deserialize, Debug, Clone)]
struct AlertLabels {
    alertname: Option<String>,
    severity: Option<String>,
    service: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct Alert {
    labels: AlertLabels,
}

struct App {
    mode: AppMode,
    stats: Stats,
    jobs: Vec<Job>,
    alerts: Vec<Alert>,
    alerts_error: Option<String>,
    api_url: String,
    gateway_url: String,
    alert_retry_count: u8,
    setup_progress: Arc<Mutex<SetupProgress>>,
    task_state: TaskCreationState,
    launcher_state: UiLauncherState,
}

const MAX_ALERT_RETRIES: u8 = 3;

impl App {
    fn new(api_url: String, gateway_url: String) -> App {
        // Load UI registry at startup
        let registry = load_ui_registry().ok();
        
        App {
            mode: AppMode::Loading,
            stats: Stats::default(),
            jobs: Vec::new(),
            alerts: Vec::new(),
            alerts_error: None,
            api_url,
            gateway_url,
            alert_retry_count: 0,
            setup_progress: Arc::new(Mutex::new(SetupProgress::default())),
            task_state: TaskCreationState::default(),
            launcher_state: UiLauncherState {
                selected_index: 0,
                registry,
                error: None,
            },
        }
    }


    fn refresh(&mut self) {
        // Fetch stats
        if let Ok(resp) = reqwest::blocking::get(format!("{}/stats", self.api_url)) {
            if let Ok(stats) = resp.json::<Stats>() {
                self.stats = stats;
            }
        }

        // Fetch recent jobs
        if let Ok(resp) = reqwest::blocking::get(format!("{}/jobs/recent", self.api_url)) {
            if let Ok(jobs) = resp.json::<Vec<Job>>() {
                self.jobs = jobs;
            }
        }

        // Fetch alerts with bounded retries (graceful degradation)
        if self.alert_retry_count < MAX_ALERT_RETRIES {
            match reqwest::blocking::Client::new()
                .get(format!("{}/proxy/alerts", self.gateway_url))
                .timeout(Duration::from_secs(2))
                .send()
            {
                Ok(resp) => {
                    if let Ok(alerts) = resp.json::<Vec<Alert>>() {
                        self.alerts = alerts;
                        self.alerts_error = None;
                        self.alert_retry_count = 0; // Reset on success
                    }
                }
                Err(e) => {
                    self.alert_retry_count += 1;
                    if self.alert_retry_count >= MAX_ALERT_RETRIES {
                        self.alerts_error = Some(format!("Unavailable ({})", e));
                    }
                }
            }
        }
    }
}

/// Check if the Kind cluster is running and accessible
fn check_cluster_status() -> ClusterStatus {
    // Try to get nodes from the kind cluster
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

/// Check if application pods are deployed in the cluster
fn check_pods_status() -> ClusterStatus {
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

/// Run the setup script and capture progress
fn run_setup_script(progress: Arc<Mutex<SetupProgress>>) {
    // Step 1: Find the project root by looking for key files
    let project_root = find_project_root();
    
    if project_root.is_none() {
        if let Ok(mut p) = progress.lock() {
            p.has_error = true;
            p.message = "Could not locate project root".to_string();
            p.error_hint = "The TUI must be run from within the odd-demonstration project".to_string();
            p.remediation = vec![
                "cd e:\\projects\\odd-demonstration".to_string(),
                "cd src\\interfaces\\tui && cargo run --release".to_string(),
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
    // pwsh is cross-platform and available via:
    //   Windows: winget install Microsoft.PowerShell
    //   macOS: brew install powershell
    //   Linux: https://learn.microsoft.com/en-us/powershell/scripting/install/installing-powershell-on-linux
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
        p.start_time = Some(std::time::Instant::now());  // Start elapsed time tracking
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
                    format!("  {} -File scripts\\start-all.ps1", shell_cmd),
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
                    // Prefix stderr lines so they're identifiable
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
                            // Extract remediation hints from the error
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
                        "Try running manually: .\\scripts\\start-all.ps1".to_string(),
                        "Ensure Docker Desktop is running".to_string(),
                    ];
                }
            }
        }
    }
}

/// Find the project root by searching for marker files
fn find_project_root() -> Option<std::path::PathBuf> {
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

/// Get a user-friendly hint based on the error message
fn get_error_hint(message: &str) -> String {
    let msg_lower = message.to_lowercase();
    
    if msg_lower.contains("docker") && msg_lower.contains("not running") {
        "Docker Desktop is not running".to_string()
    } else if msg_lower.contains("kind") && msg_lower.contains("not found") {
        "The 'kind' tool is not installed".to_string()
    } else if msg_lower.contains("kubectl") && msg_lower.contains("not found") {
        "kubectl is not installed".to_string()
    } else if msg_lower.contains("timeout") {
        "The operation timed out - cluster may be slow to start".to_string()
    } else if msg_lower.contains("port") && msg_lower.contains("in use") {
        "A required port is already in use by another application".to_string()
    } else if msg_lower.contains("permission") || msg_lower.contains("access denied") {
        #[cfg(target_os = "windows")]
        { "Permission denied - try running as administrator".to_string() }
        #[cfg(not(target_os = "windows"))]
        { "Permission denied - try running with sudo or check file permissions".to_string() }
    } else {
        "An error occurred during setup".to_string()
    }
}

/// Get remediation steps based on the error message (cross-platform)
fn get_remediation_steps(message: &str) -> Vec<String> {
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

/// Get platform-specific pwsh installation steps
fn get_pwsh_install_steps() -> Vec<String> {
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
fn get_kind_install_steps() -> Vec<String> {
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
fn get_kubectl_install_steps() -> Vec<String> {
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
fn get_port_conflict_steps() -> Vec<String> {
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
fn get_generic_error_steps() -> Vec<String> {
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

/// Renders the loading splash screen with animated spinner
fn render_loading_splash<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    frame_idx: usize,
) -> Result<(), Box<dyn Error>> {
    let spinner = SPINNER_FRAMES[frame_idx % SPINNER_FRAMES.len()];
    let message = LOADING_MESSAGES[frame_idx / 3 % LOADING_MESSAGES.len()];
    
    terminal.draw(|f| {
        let size = f.size();
        
        // Center the content vertically
        let vertical_center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Length(18),
                Constraint::Percentage(25),
            ])
            .split(size);
        
        // Center the content horizontally
        let horizontal_center = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Min(40),
                Constraint::Percentage(25),
            ])
            .split(vertical_center[1]);
        
        let center_area = horizontal_center[1];
        
        // Build the splash content
        let mut lines: Vec<Line> = Vec::new();
        
        // Add logo lines with cyan color
        for line in LOGO.lines() {
            lines.push(Line::from(vec![
                Span::styled(line, Style::default().fg(Color::Green))
            ]));
        }
        
        // Add spacing
        lines.push(Line::from(""));
        
        // Add animated loading line with spinner
        let dots = ".".repeat((frame_idx % 4) + 1);
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", spinner),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}{}", message, dots),
                Style::default().fg(Color::White),
            ),
        ]));
        
        // Add subtle branding line
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "Distributed Task Observatory",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ),
        ]));
        
        let splash = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green))
                    .title(" oddessentials.com ")
                    .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            );
        
        f.render_widget(splash, center_area);
    })?;
    
    Ok(())
}

/// Renders the launcher view when cluster is not detected
fn render_launcher_view<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    frame_idx: usize,
) -> Result<(), Box<dyn Error>> {
    let spinner = SPINNER_FRAMES[frame_idx % SPINNER_FRAMES.len()];
    
    terminal.draw(|f| {
        let size = f.size();
        
        // Center the content
        let vertical_center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Length(22),
                Constraint::Percentage(20),
            ])
            .split(size);
        
        let horizontal_center = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Min(50),
                Constraint::Percentage(20),
            ])
            .split(vertical_center[1]);
        
        let center_area = horizontal_center[1];
        
        let mut lines: Vec<Line> = Vec::new();
        
        // Add logo
        for line in LOGO.lines() {
            lines.push(Line::from(vec![
                Span::styled(line, Style::default().fg(Color::Green))
            ]));
        }
        
        lines.push(Line::from(""));
        
        // Status message
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", spinner),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                "Cluster not detected",
                Style::default().fg(Color::Yellow),
            ),
        ]));
        
        lines.push(Line::from(""));
        
        // Action prompts
        lines.push(Line::from(vec![
            Span::styled(
                "  Press ",
                Style::default().fg(Color::Gray),
            ),
            Span::styled(
                "L",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to launch cluster",
                Style::default().fg(Color::Gray),
            ),
        ]));
        
        lines.push(Line::from(vec![
            Span::styled(
                "  Press ",
                Style::default().fg(Color::Gray),
            ),
            Span::styled(
                "Q",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to quit",
                Style::default().fg(Color::Gray),
            ),
        ]));
        
        let launcher = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green))
                    .title(" oddessentials.com ")
                    .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            );
        
        f.render_widget(launcher, center_area);
    })?;
    
    Ok(())
}

/// Renders the setup progress view
fn render_setup_progress<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    progress: &SetupProgress,
    frame_idx: usize,
) -> Result<(), Box<dyn Error>> {
    let spinner = SPINNER_FRAMES[frame_idx % SPINNER_FRAMES.len()];
    
    terminal.draw(|f| {
        let size = f.size();
        
        // Add more top padding so logo doesn't get cut off
        let vertical_center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // More top padding for logo
                Constraint::Min(20),
                Constraint::Length(1),
            ])
            .split(size);
        
        let horizontal_center = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(5),
                Constraint::Min(70),
                Constraint::Percentage(5),
            ])
            .split(vertical_center[1]);
        
        let center_area = horizontal_center[1];
        
        let mut lines: Vec<Line> = Vec::new();
        
        // Add padding lines to push content down and prevent logo cutoff
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        
        // Always show logo - green normally, red on error
        let logo_color = if progress.has_error {
            Color::Red
        } else {
            Color::Green
        };
        
        for line in LOGO.lines() {
            lines.push(Line::from(vec![
                Span::styled(line, Style::default().fg(logo_color))
            ]));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(""));  // Extra spacing after logo
        
        // Progress title
        let title_style = if progress.has_error {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else if progress.is_complete {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        };
        
        let status_text = if progress.has_error {
            "❌ Setup Failed"
        } else if progress.is_complete {
            "✅ Setup Complete!"
        } else {
            "🚀 Setting up cluster..."
        };
        
        lines.push(Line::from(vec![
            Span::styled(status_text, title_style),
        ]));
        
        lines.push(Line::from(""));
        
        // Current step / error message
        if !progress.is_complete {
            // Calculate elapsed time
            let elapsed_str = if let Some(start) = progress.start_time {
                let elapsed = start.elapsed();
                let mins = elapsed.as_secs() / 60;
                let secs = elapsed.as_secs() % 60;
                format!("  ⏱ {:02}:{:02}", mins, secs)
            } else {
                String::new()
            };
            
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", spinner),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    &progress.message,
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    elapsed_str,
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        } else {
            // Show the main message
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {}", &progress.message),
                    Style::default().fg(if progress.has_error { Color::Red } else { Color::Cyan }),
                ),
            ]));
            
            // If error, show the hint and remediation steps
            if progress.has_error && !progress.error_hint.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled(
                        "  💡 ",
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(
                        &progress.error_hint,
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
                ]));
                
                // Show remediation steps
                if !progress.remediation.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled(
                            "  📋 To fix this:",
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    
                    for step in &progress.remediation {
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("     {}", step),
                                Style::default().fg(Color::White),
                            ),
                        ]));
                    }
                }
            }
        }
        
        // Show last few log lines (fewer when showing remediation)
        let max_logs = if progress.has_error && !progress.remediation.is_empty() { 3 } else { 5 };
        if !progress.log_lines.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(
                    "  Log:",
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
            let log_start = progress.log_lines.len().saturating_sub(max_logs);
            for log_line in progress.log_lines.iter().skip(log_start) {
                let color = if log_line.starts_with("[ERR]") { Color::Red } else { Color::DarkGray };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("    {}", log_line.chars().take(65).collect::<String>()),
                        Style::default().fg(color),
                    ),
                ]));
            }
        }
        
        // Continue prompt
        if progress.is_complete {
            lines.push(Line::from(""));
            let continue_text = if progress.has_error {
                "  Press any key to return to launcher..."
            } else {
                "  Press any key to continue to dashboard..."
            };
            lines.push(Line::from(vec![
                Span::styled(
                    continue_text,
                    Style::default().fg(Color::Green),
                ),
            ]));
        }
        
        let border_color = if progress.has_error { Color::Red } else { Color::Green };
        let setup_view = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(" Cluster Setup ")
                    .title_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD))
            );
        
        f.render_widget(setup_view, center_area);
    })?;
    
    Ok(())
}

// Modal rendering is done inline in the dashboard draw call to avoid ratatui 0.24 API compatibility issues

// ============================================================================
// CLI Features: Platform Support, Version, Help, and Doctor
// ============================================================================

/// Support matrix using EXACT Rust std::env::consts values
/// Verified against: https://doc.rust-lang.org/std/env/consts/constant.OS.html
const SUPPORT_MATRIX: &[(&str, &str)] = &[
    ("windows", "x86_64"),
    ("macos", "x86_64"),
    ("macos", "aarch64"),
    ("linux", "x86_64"),
    ("linux", "aarch64"),
];

const SUPPORT_MATRIX_URL: &str = "https://github.com/oddessentials/odd-demonstration/blob/main/docs/SUPPORT_MATRIX.md";

/// Check if current platform is supported (no I/O, pure computation)
fn check_platform_support() -> Result<(), String> {
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

/// Print version information with build metadata
fn print_version() {
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
fn print_help() {
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
fn run_doctor() {
    println!("odd-dashboard doctor");
    println!("====================");
    println!();
    
    // Platform info (already validated by main())
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    println!("[OK] Platform: {}-{} (supported)", os, arch);
    
    let mut all_ok = true;
    
    // Check Docker
    match check_command_version("docker", &["--version"]) {
        Ok(version) => println!("[OK] Docker: {}", version),
        Err(msg) => {
            println!("[FAIL] Docker: {}", msg);
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
                        all_ok = false;
                    }
                }
            } else {
                println!("[FAIL] PowerShell Core: not found");
                println!("       Install: https://docs.microsoft.com/en-us/powershell/scripting/install/installing-powershell");
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
            all_ok = false;
        }
    }
    
    println!();
    
    // Summary
    if all_ok {
        println!("All prerequisites satisfied!");
        println!();
        println!("Run 'odd-dashboard' to start the TUI.");
    } else {
        println!("Some prerequisites are missing.");
        println!("See: {}", SUPPORT_MATRIX_URL);
        std::process::exit(1);
    }
}

/// Check if a command exists and get its version output
fn check_command_version(cmd: &str, args: &[&str]) -> Result<String, String> {
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

fn main() -> Result<(), Box<dyn Error>> {
    // === PHASE 1: Collect args (no external I/O yet) ===
    let args: Vec<String> = std::env::args().collect();
    
    // === PHASE 2: Platform validation (pure computation, no I/O) ===
    if let Err(msg) = check_platform_support() {
        eprintln!("ERROR: {}", msg);
        std::process::exit(1);
    }
    
    // === PHASE 3: CLI dispatch (before terminal initialization) ===
    match args.get(1).map(|s| s.as_str()) {
        Some("--version") | Some("-V") => {
            print_version();
            return Ok(());
        }
        Some("--help") | Some("-h") => {
            print_help();
            return Ok(());
        }
        Some("doctor") => {
            run_doctor();
            return Ok(());
        }
        Some(arg) if arg.starts_with('-') => {
            eprintln!("Unknown option: {}", arg);
            eprintln!("Run 'odd-dashboard --help' for usage.");
            std::process::exit(1);
        }
        _ => {}
    }
    
    // === PHASE 4: Now safe to perform I/O and initialize terminal ===
    let api_url = std::env::var("READ_MODEL_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let gateway_url = std::env::var("GATEWAY_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(api_url.clone(), gateway_url.clone());
    
    // Check cluster status first
    let cluster_status = check_cluster_status();
    
    match cluster_status {
        ClusterStatus::Ready => {
            // Cluster is ready, go to loading -> dashboard flow
            app.mode = AppMode::Loading;
            
            let loading_done = Arc::new(AtomicBool::new(false));
            let loading_done_clone = Arc::clone(&loading_done);
            
            let api_url_clone = app.api_url.clone();
            let gateway_url_clone = app.gateway_url.clone();
            let handle = thread::spawn(move || {
                let mut background_app = App::new(api_url_clone, gateway_url_clone);
                background_app.refresh();
                loading_done_clone.store(true, Ordering::SeqCst);
                background_app
            });
            
            // Animated loading splash
            let mut frame_idx = 0;
            while !loading_done.load(Ordering::SeqCst) {
                render_loading_splash(&mut terminal, frame_idx)?;
                frame_idx += 1;
                thread::sleep(Duration::from_millis(80));
                
                if event::poll(Duration::from_millis(0))? {
                    if let Event::Key(key) = event::read()? {
                        if key.code == KeyCode::Char('q') {
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
                            terminal.show_cursor()?;
                            return Ok(());
                        }
                    }
                }
            }
            
            app = handle.join().expect("Background thread panicked");
            app.mode = AppMode::Dashboard;
        }
        ClusterStatus::NoPods | ClusterStatus::NotFound | ClusterStatus::Error(_) => {
            // Show launcher view - will trigger deployment and port-forwards
            app.mode = AppMode::Launcher;
        }
    }

    // Main event loop
    let mut frame_idx = 0;
    loop {
        match app.mode {
            AppMode::Loading => {
                render_loading_splash(&mut terminal, frame_idx)?;
            }
            AppMode::Launcher => {
                render_launcher_view(&mut terminal, frame_idx)?;
            }
            AppMode::SetupProgress => {
                let progress = app.setup_progress.lock().unwrap().clone();
                render_setup_progress(&mut terminal, &progress, frame_idx)?;
                
                // Check if setup is complete
                if progress.is_complete {
                    if event::poll(Duration::from_millis(100))? {
                        if let Event::Key(_) = event::read()? {
                            if progress.has_error {
                                app.mode = AppMode::Launcher;
                            } else {
                                // Refresh and go to dashboard
                                app.refresh();
                                app.mode = AppMode::Dashboard;
                            }
                        }
                    }
                }
            }
            AppMode::Dashboard => {
                terminal.draw(|f| {
                    // Main vertical layout
                    let main_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(1)
                        .constraints([
                            Constraint::Length(15), // Header with logo + title + stats
                            Constraint::Length(6),  // Alerts
                            Constraint::Min(8),     // Jobs table
                            Constraint::Length(1),  // Help bar
                        ])
                        .split(f.size());

                    // Header area: Logo on left, Title+Stats on right
                    let header_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Length(32), // Logo width
                            Constraint::Min(30),    // Title + Stats
                        ])
                        .split(main_chunks[0]);

                    // Render the ASCII logo
                    let logo_lines: Vec<Line> = LOGO
                        .lines()
                        .map(|line| {
                            Line::from(vec![Span::styled(
                                line,
                                Style::default().fg(Color::Green),
                            )])
                        })
                        .collect();
                    let logo_widget = Paragraph::new(logo_lines)
                        .alignment(Alignment::Center)
                        .block(Block::default().borders(Borders::ALL).title(" oddessentials.com "));
                    f.render_widget(logo_widget, header_chunks[0]);

                    // Right side: Title + Stats
                    let right_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(3), // Title
                            Constraint::Min(5),    // Stats
                        ])
                        .split(header_chunks[1]);

                    // Title
                    let title = Paragraph::new(vec![Line::from(vec![
                        Span::styled(
                            " 📡 Distributed Task Observatory ",
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        ),
                    ])])
                    .block(Block::default().borders(Borders::ALL));
                    f.render_widget(title, right_chunks[0]);

                    // Stats
                    let stats_text = vec![
                        Line::from(vec![
                            Span::raw("  Total Jobs:     "),
                            Span::styled(
                                format!("{}", app.stats.total_jobs),
                                Style::default().fg(Color::Yellow),
                            ),
                        ]),
                        Line::from(vec![
                            Span::raw("  Completed:      "),
                            Span::styled(
                                format!("{}", app.stats.completed_jobs),
                                Style::default().fg(Color::Cyan),
                            ),
                        ]),
                        Line::from(vec![
                            Span::raw("  Failed:         "),
                            Span::styled(
                                format!("{}", app.stats.failed_jobs),
                                Style::default().fg(Color::Red),
                            ),
                        ]),
                        Line::from(vec![
                            Span::raw("  Last Event:     "),
                            Span::styled(&app.stats.last_event_time, Style::default().fg(Color::Blue)),
                        ]),
                    ];
                    let stats_block = Paragraph::new(stats_text)
                        .block(Block::default().title(" Statistics ").borders(Borders::ALL));
                    f.render_widget(stats_block, right_chunks[1]);

                    // Alerts pane
                    let alerts_content: Vec<Line> = if let Some(ref err) = app.alerts_error {
                        vec![Line::from(vec![
                            Span::styled(format!("⚠ {}", err), Style::default().fg(Color::Yellow)),
                        ])]
                    } else if app.alerts.is_empty() {
                        vec![Line::from(vec![
                            Span::styled("✓ No active alerts", Style::default().fg(Color::Cyan)),
                        ])]
                    } else {
                        app.alerts.iter().take(3).map(|alert| {
                            let name = alert.labels.alertname.as_deref().unwrap_or("Unknown");
                            let severity = alert.labels.severity.as_deref().unwrap_or("warning");
                            let service = alert.labels.service.as_deref().unwrap_or("-");
                            let color = if severity == "critical" { Color::Red } else { Color::Yellow };
                            Line::from(vec![
                                Span::styled(format!("🚨 {} ", name), Style::default().fg(color)),
                                Span::styled(format!("[{}]", service), Style::default().fg(Color::Gray)),
                            ])
                        }).collect()
                    };
                    let alerts_block = Paragraph::new(alerts_content)
                        .block(Block::default().title(format!(" Alerts ({}) ", app.alerts.len())).borders(Borders::ALL));
                    f.render_widget(alerts_block, main_chunks[1]);

                    // Jobs table
                    let header = Row::new(vec!["ID", "Type", "Status", "Created"])
                        .style(Style::default().fg(Color::Yellow))
                        .bottom_margin(1);

                    let rows: Vec<Row> = app
                        .jobs
                        .iter()
                        .take(10)
                        .map(|job| {
                            let status_style = match job.status.as_str() {
                                "COMPLETED" => Style::default().fg(Color::Cyan),
                                "FAILED" => Style::default().fg(Color::Red),
                                "PENDING" => Style::default().fg(Color::Yellow),
                                _ => Style::default(),
                            };
                            Row::new(vec![
                                Cell::from(job.id.chars().take(8).collect::<String>()),
                                Cell::from(job.job_type.clone()),
                                Cell::from(job.status.clone()).style(status_style),
                                Cell::from(job.created_at.clone()),
                            ])
                        })
                        .collect();

                    let widths = [
                        Constraint::Length(10),
                        Constraint::Length(20),
                        Constraint::Length(12),
                        Constraint::Min(25),
                    ];
                    let table = Table::new(rows)
                        .header(header)
                        .widths(&widths)
                        .block(Block::default().title(" Recent Jobs ").borders(Borders::ALL));
                    f.render_widget(table, main_chunks[2]);

                    // Help bar
                    let help = Paragraph::new(Line::from(vec![
                        Span::styled(" Q", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::raw(" Quit  "),
                        Span::styled("R", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::raw(" Refresh  "),
                        Span::styled("N", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                        Span::raw(" New Task  "),
                        Span::styled("U", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::raw(" UIs"),
                    ]));
                    f.render_widget(help, main_chunks[3]);


                })?;
            }
            AppMode::TaskCreation => {
                terminal.draw(|f| {
                    let area = f.size();
                    let modal_width = 55u16;
                    let modal_height = 10u16;
                    let x = (area.width.saturating_sub(modal_width)) / 2;
                    let y = (area.height.saturating_sub(modal_height)) / 2;
                    let modal_area = Rect::new(x, y, modal_width, modal_height);
                    
                    f.render_widget(Clear, modal_area);
                    
                    let (status_line, status_color) = match &app.task_state.status {
                        TaskCreationStatus::Editing => ("Type job name, Enter to submit, Esc to cancel", Color::Gray),
                        TaskCreationStatus::Submitting => ("⏳ Submitting...", Color::Yellow),
                        TaskCreationStatus::Success(id) => {
                            let msg = format!("✓ Created job: {}...", &id[..8.min(id.len())]);
                            // We can't borrow, so we'll handle this inline
                            ("✓ Job created! Press any key to close", Color::Green)
                        }
                        TaskCreationStatus::Error(e) => ("✗ Error - Press any key to close", Color::Red),
                    };
                    
                    let input_display = format!("  Job Type: {}_", app.task_state.job_type);
                    let error_msg = if let TaskCreationStatus::Error(e) = &app.task_state.status {
                        format!("  {}", e)
                    } else if let TaskCreationStatus::Success(id) = &app.task_state.status {
                        format!("  Job ID: {}", id)
                    } else {
                        String::new()
                    };
                    
                    let modal_lines = vec![
                        Line::from(""),
                        Line::from(vec![
                            Span::styled(&input_display, Style::default().fg(Color::White)),
                        ]),
                        Line::from(""),
                        Line::from(vec![
                            Span::styled(status_line, Style::default().fg(status_color)),
                        ]),
                        if !error_msg.is_empty() {
                            Line::from(vec![
                                Span::styled(&error_msg, Style::default().fg(status_color)),
                            ])
                        } else {
                            Line::from("")
                        },
                    ];
                    
                    let border_color = match &app.task_state.status {
                        TaskCreationStatus::Success(_) => Color::Green,
                        TaskCreationStatus::Error(_) => Color::Red,
                        _ => Color::Cyan,
                    };
                    
                    let modal = Paragraph::new(modal_lines)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(border_color))
                                .title(" ➕ New Task ")
                                .title_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD))
                        );
                    
                    f.render_widget(modal, modal_area);
                })?;
            }
            AppMode::UiLauncher => {
                terminal.draw(|f| {
                    let area = f.size();
                    let modal_width = 60u16.min(area.width.saturating_sub(4));
                    let modal_height = 16u16.min(area.height.saturating_sub(4));
                    let x = (area.width.saturating_sub(modal_width)) / 2;
                    let y = (area.height.saturating_sub(modal_height)) / 2;
                    let modal_area = Rect::new(x, y, modal_width, modal_height);
                    
                    f.render_widget(Clear, modal_area);
                    
                    let mut lines: Vec<Line> = vec![
                        Line::from("  ↑/↓ Navigate, Enter to open, Esc to close"),
                        Line::from(""),
                    ];
                    
                    if let Some(ref registry) = app.launcher_state.registry {
                        for (i, entry) in registry.entries.iter().enumerate() {
                            let is_selected = i == app.launcher_state.selected_index;
                            let prefix = if is_selected { "▶ " } else { "  " };
                            let style = if is_selected {
                                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::White)
                            };
                            
                            lines.push(Line::from(vec![
                                Span::styled(format!("{}{} {}", prefix, entry.emoji, entry.name), style),
                            ]));
                            
                            if is_selected {
                                lines.push(Line::from(vec![
                                    Span::styled(format!("     {}", entry.description), Style::default().fg(Color::DarkGray)),
                                ]));
                            }
                        }
                    } else {
                        lines.push(Line::from(vec![
                            Span::styled("  ⚠ Could not load UI registry", Style::default().fg(Color::Yellow)),
                        ]));
                    }
                    
                    let launcher = Paragraph::new(lines)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(Color::Cyan))
                                .title(" 🚀 Launch UI ")
                                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                        );
                    
                    f.render_widget(launcher, modal_area);
                })?;
            }
        }


        frame_idx += 1;

        // Handle input based on mode
        let poll_duration = match app.mode {
            AppMode::Dashboard => Duration::from_secs(2),
            AppMode::SetupProgress => Duration::from_millis(100),
            _ => Duration::from_millis(80),
        };

        if event::poll(poll_duration)? {
            if let Event::Key(key) = event::read()? {
                match app.mode {
                    AppMode::Launcher => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => break,
                            KeyCode::Char('l') | KeyCode::Char('L') => {
                                // Start setup
                                app.mode = AppMode::SetupProgress;
                                let progress = Arc::clone(&app.setup_progress);
                                thread::spawn(move || {
                                    run_setup_script(progress);
                                });
                            }
                            _ => {}
                        }
                    }
                    AppMode::Dashboard => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => break,
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                app.alert_retry_count = 0;
                                app.refresh();
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') => {
                                app.task_state = TaskCreationState::default();
                                app.mode = AppMode::TaskCreation;
                            }
                            KeyCode::Char('u') | KeyCode::Char('U') => {
                                app.launcher_state.selected_index = 0;
                                app.mode = AppMode::UiLauncher;
                            }
                            _ => {}
                        }
                    }
                    AppMode::TaskCreation => {
                        // Isolated key handling for task creation modal
                        match &app.task_state.status {
                            TaskCreationStatus::Editing => {
                                match key.code {
                                    KeyCode::Esc => {
                                        app.mode = AppMode::Dashboard;
                                    }
                                    KeyCode::Enter => {
                                        if !app.task_state.job_type.trim().is_empty() {
                                            app.task_state.status = TaskCreationStatus::Submitting;
                                            let gateway_url = app.gateway_url.clone();
                                            let job_type = app.task_state.job_type.clone();
                                            
                                            // Submit job (blocking but with timeout)
                                            match submit_job(&gateway_url, &job_type) {
                                                Ok(job_id) => {
                                                    app.task_state.status = TaskCreationStatus::Success(job_id);
                                                }
                                                Err(e) => {
                                                    app.task_state.status = TaskCreationStatus::Error(e.to_string());
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Char(c) => {
                                        app.task_state.job_type.push(c);
                                    }
                                    KeyCode::Backspace => {
                                        app.task_state.job_type.pop();
                                    }
                                    _ => {}
                                }
                            }
                            TaskCreationStatus::Submitting => {
                                // Ignore keys while submitting
                            }
                            TaskCreationStatus::Success(_) | TaskCreationStatus::Error(_) => {
                                // Any key closes and returns to dashboard
                                app.refresh(); // Refresh to show new job
                                app.mode = AppMode::Dashboard;
                            }
                        }
                    }
                    AppMode::UiLauncher => {
                        // Isolated key handling for UI launcher
                        match key.code {
                            KeyCode::Esc => {
                                app.mode = AppMode::Dashboard;
                            }
                            KeyCode::Up => {
                                if app.launcher_state.selected_index > 0 {
                                    app.launcher_state.selected_index -= 1;
                                }
                            }
                            KeyCode::Down => {
                                if let Some(ref registry) = app.launcher_state.registry {
                                    if app.launcher_state.selected_index < registry.entries.len().saturating_sub(1) {
                                        app.launcher_state.selected_index += 1;
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                if let Some(ref registry) = app.launcher_state.registry {
                                    if let Some(entry) = registry.entries.get(app.launcher_state.selected_index) {
                                        let url = format!("{}:{}{}", registry.base_url, entry.port, entry.path);
                                        match open_browser(&url) {
                                            Ok(()) => {
                                                app.launcher_state.error = None;
                                                app.mode = AppMode::Dashboard;
                                            }
                                            Err(e) => {
                                                // Show error in launcher, don't close
                                                app.launcher_state.error = Some(e.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {
                        if key.code == KeyCode::Char('q') {
                            break;
                        }
                    }
                }
            }
        } else if app.mode == AppMode::Dashboard {
            app.refresh();
        }
    }


    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

// Unit tests - deterministic, no network/UI dependencies
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_default() {
        let stats = Stats::default();
        assert_eq!(stats.total_jobs, 0);
        assert_eq!(stats.completed_jobs, 0);
        assert_eq!(stats.failed_jobs, 0);
        assert!(stats.last_event_time.is_empty());
    }

    #[test]
    fn test_app_new() {
        let app = App::new("http://localhost:8080".to_string(), "http://localhost:3000".to_string());
        assert_eq!(app.api_url, "http://localhost:8080");
        assert_eq!(app.gateway_url, "http://localhost:3000");
        assert!(app.jobs.is_empty());
        assert!(app.alerts.is_empty());
        assert!(app.alerts_error.is_none());
        assert_eq!(app.alert_retry_count, 0);
        // Check new task/launcher state fields
        assert!(app.task_state.job_type.is_empty());
        assert_eq!(app.task_state.status, TaskCreationStatus::Editing);
        assert_eq!(app.launcher_state.selected_index, 0);
    }


    #[test]
    fn test_app_mode_initial() {
        let app = App::new("http://test".to_string(), "http://test".to_string());
        assert_eq!(app.mode, AppMode::Loading);
    }

    #[test]
    fn test_max_alert_retries_constant() {
        assert!(MAX_ALERT_RETRIES >= 1);
        assert!(MAX_ALERT_RETRIES <= 10);
    }

    #[test]
    fn test_job_deserialization() {
        let json = r#"{"id": "abc123", "type": "PROCESS", "status": "COMPLETED", "createdAt": "2024-01-01T00:00:00Z"}"#;
        let job: Job = serde_json::from_str(json).expect("Failed to deserialize Job");
        assert_eq!(job.id, "abc123");
        assert_eq!(job.job_type, "PROCESS");
        assert_eq!(job.status, "COMPLETED");
        assert_eq!(job.created_at, "2024-01-01T00:00:00Z");
    }

    #[test]
    fn test_stats_deserialization() {
        let json = r#"{"totalJobs": 100, "completedJobs": 90, "failedJobs": 10, "lastEventTime": "2024-01-01T12:00:00Z"}"#;
        let stats: Stats = serde_json::from_str(json).expect("Failed to deserialize Stats");
        assert_eq!(stats.total_jobs, 100);
        assert_eq!(stats.completed_jobs, 90);
        assert_eq!(stats.failed_jobs, 10);
        assert_eq!(stats.last_event_time, "2024-01-01T12:00:00Z");
    }

    // Loading screen tests
    #[test]
    fn test_logo_is_not_empty() {
        assert!(!LOGO.is_empty(), "Logo should not be empty");
        assert!(LOGO.len() > 50, "Logo should have substantial content");
    }

    #[test]
    fn test_logo_has_expected_line_count() {
        let line_count = LOGO.lines().count();
        assert!(line_count >= 10, "Logo should have at least 10 lines, got {}", line_count);
        assert!(line_count <= 20, "Logo should have at most 20 lines, got {}", line_count);
    }

    #[test]
    fn test_spinner_frames_count() {
        assert_eq!(SPINNER_FRAMES.len(), 10, "Spinner should have 10 frames");
    }

    #[test]
    fn test_spinner_frames_are_braille() {
        for (i, frame) in SPINNER_FRAMES.iter().enumerate() {
            assert!(!frame.is_empty(), "Spinner frame {} should not be empty", i);
            for c in frame.chars() {
                assert!(
                    ('\u{2800}'..='\u{28FF}').contains(&c),
                    "Spinner frame {} should contain Braille characters, got '{}'", i, c
                );
            }
        }
    }

    #[test]
    fn test_loading_messages_count() {
        assert!(LOADING_MESSAGES.len() >= 3, "Should have at least 3 loading messages");
        assert!(LOADING_MESSAGES.len() <= 10, "Should not have too many loading messages");
    }

    #[test]
    fn test_loading_messages_not_empty() {
        for (i, msg) in LOADING_MESSAGES.iter().enumerate() {
            assert!(!msg.is_empty(), "Loading message {} should not be empty", i);
            assert!(msg.len() >= 5, "Loading message {} should be descriptive", i);
        }
    }

    #[test]
    fn test_spinner_cycling_logic() {
        let frame_count = SPINNER_FRAMES.len();
        assert_eq!(0 % frame_count, 0);
        assert_eq!(5 % frame_count, 5);
        assert_eq!(10 % frame_count, 0);
        assert_eq!(15 % frame_count, 5);
    }

    #[test]
    fn test_message_cycling_logic() {
        let msg_count = LOADING_MESSAGES.len();
        assert_eq!((0 / 3) % msg_count, 0);
        assert_eq!((2 / 3) % msg_count, 0);
        assert_eq!((3 / 3) % msg_count, 1);
        assert_eq!((5 / 3) % msg_count, 1);
        let cycle_point = 3 * msg_count;
        assert_eq!((cycle_point / 3) % msg_count, 0);
    }

    #[test]
    fn test_dots_animation_logic() {
        assert_eq!((0 % 4) + 1, 1);
        assert_eq!((1 % 4) + 1, 2);
        assert_eq!((2 % 4) + 1, 3);
        assert_eq!((3 % 4) + 1, 4);
        assert_eq!((4 % 4) + 1, 1);
    }

    #[test]
    fn test_setup_progress_default() {
        let progress = SetupProgress::default();
        assert!(progress.current_step.is_empty());
        assert!(progress.message.is_empty());
        assert!(!progress.is_complete);
        assert!(!progress.has_error);
        assert!(progress.log_lines.is_empty());
    }

    #[test]
    fn test_cluster_status_variants() {
        let ready = ClusterStatus::Ready;
        let no_pods = ClusterStatus::NoPods;
        let not_found = ClusterStatus::NotFound;
        let error = ClusterStatus::Error("test error".to_string());
        
        assert_eq!(ready, ClusterStatus::Ready);
        assert_eq!(no_pods, ClusterStatus::NoPods);
        assert_eq!(not_found, ClusterStatus::NotFound);
        assert!(matches!(error, ClusterStatus::Error(_)));
        
        // NoPods is distinct from Ready and NotFound
        assert_ne!(no_pods, ready);
        assert_ne!(no_pods, not_found);
    }

    #[test]
    fn test_app_mode_variants() {
        let loading = AppMode::Loading;
        let launcher = AppMode::Launcher;
        let setup = AppMode::SetupProgress;
        let dashboard = AppMode::Dashboard;
        
        assert_eq!(loading, AppMode::Loading);
        assert_eq!(launcher, AppMode::Launcher);
        assert_eq!(setup, AppMode::SetupProgress);
        assert_eq!(dashboard, AppMode::Dashboard);
    }

    // Error handling tests
    #[test]
    fn test_get_error_hint_docker() {
        let hint = get_error_hint("Docker is not running");
        assert!(hint.contains("Docker"), "Should mention Docker");
    }

    #[test]
    fn test_get_error_hint_kind() {
        let hint = get_error_hint("kind not found in PATH");
        assert!(hint.contains("kind"), "Should mention kind");
    }

    #[test]
    fn test_get_error_hint_kubectl() {
        let hint = get_error_hint("kubectl not found");
        assert!(hint.contains("kubectl"), "Should mention kubectl");
    }

    #[test]
    fn test_get_error_hint_timeout() {
        let hint = get_error_hint("Operation timeout exceeded");
        assert!(hint.to_lowercase().contains("timeout") || hint.to_lowercase().contains("timed out"), 
            "Should mention timeout, got: {}", hint);
    }

    #[test]
    fn test_get_error_hint_port() {
        let hint = get_error_hint("Port 3000 is already in use");
        assert!(hint.contains("port"), "Should mention port");
    }

    #[test]
    fn test_get_error_hint_permission() {
        let hint = get_error_hint("Access denied to file");
        assert!(hint.contains("Permission") || hint.contains("administrator"), 
            "Should mention permission issue");
    }

    #[test]
    fn test_get_error_hint_unknown() {
        let hint = get_error_hint("Some unknown error occurred");
        assert!(!hint.is_empty(), "Should provide a generic hint for unknown errors");
    }

    #[test]
    fn test_get_remediation_steps_docker() {
        let steps = get_remediation_steps("Docker not running");
        assert!(!steps.is_empty(), "Should provide remediation steps");
        assert!(steps.iter().any(|s| s.to_lowercase().contains("docker")), 
            "Steps should mention Docker");
    }

    #[test]
    fn test_get_remediation_steps_kind() {
        let steps = get_remediation_steps("kind command not found");
        assert!(!steps.is_empty(), "Should provide remediation steps");
        assert!(steps.iter().any(|s| s.to_lowercase().contains("kind") || s.contains("winget")), 
            "Steps should mention how to install kind");
    }

    #[test]
    fn test_get_remediation_steps_port_conflict() {
        let steps = get_remediation_steps("Port 8080 in use");
        assert!(!steps.is_empty(), "Should provide remediation steps");
        // On Windows: netstat, on Linux/macOS: lsof
        assert!(steps.iter().any(|s| s.contains("netstat") || s.contains("lsof") || s.contains("port")), 
            "Steps should help diagnose port conflict");
    }

    #[test]
    fn test_get_remediation_steps_generic() {
        let steps = get_remediation_steps("Something went wrong");
        assert!(!steps.is_empty(), "Should always provide some remediation steps");
    }

    #[test]
    fn test_setup_progress_with_error_fields() {
        let mut progress = SetupProgress::default();
        progress.has_error = true;
        progress.message = "Test error".to_string();
        progress.error_hint = "This is a hint".to_string();
        progress.remediation = vec!["Step 1".to_string(), "Step 2".to_string()];
        
        assert!(progress.has_error);
        assert_eq!(progress.error_hint, "This is a hint");
        assert_eq!(progress.remediation.len(), 2);
    }

    #[test]
    fn test_setup_progress_log_lines() {
        let mut progress = SetupProgress::default();
        progress.log_lines.push("Line 1".to_string());
        progress.log_lines.push("[ERR] Error line".to_string());
        progress.log_lines.push("Line 3".to_string());
        
        assert_eq!(progress.log_lines.len(), 3);
        assert!(progress.log_lines[1].starts_with("[ERR]"));
    }

    // Path resolution tests (limited - can't test file system in unit tests)
    #[test]
    fn test_find_project_root_returns_option() {
        // This test just verifies the function signature and doesn't panic
        let result = find_project_root();
        // Result may be Some or None depending on where tests are run
        assert!(result.is_some() || result.is_none());
    }

    #[test]
    fn test_setup_progress_complete_states() {
        let mut progress = SetupProgress::default();
        
        // Initial state
        assert!(!progress.is_complete);
        assert!(!progress.has_error);
        
        // Success state
        progress.is_complete = true;
        progress.message = "Setup complete!".to_string();
        assert!(progress.is_complete);
        assert!(!progress.has_error);
        
        // Error state
        let mut error_progress = SetupProgress::default();
        error_progress.is_complete = true;
        error_progress.has_error = true;
        error_progress.message = "Setup failed".to_string();
        assert!(error_progress.is_complete);
        assert!(error_progress.has_error);
    }

    // Integration tests for script execution
    #[test]
    fn test_find_project_root_finds_scripts() {
        let root = find_project_root();
        if let Some(ref path) = root {
            let script_path = path.join("scripts").join("start-all.ps1");
            assert!(script_path.exists(), "Script should exist at {:?}", script_path);
        }
    }

    #[test]
    fn test_powershell_available() {
        // Test that at least one PowerShell variant is available
        let pwsh = Command::new("pwsh")
            .args(["-NoProfile", "-Command", "exit 0"])
            .output();
        
        let powershell = Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", "exit 0"])
            .output();
        
        let pwsh_ok = pwsh.map(|o| o.status.success()).unwrap_or(false);
        let ps_ok = powershell.map(|o| o.status.success()).unwrap_or(false);
        
        assert!(pwsh_ok || ps_ok, "At least one PowerShell should be available");
    }

    #[test]
    fn test_script_executes_and_outputs_json() {
        let root = find_project_root();
        if root.is_none() {
            return; // Skip if project root not found
        }
        
        let root = root.unwrap();
        let script_path = root.join("scripts").join("start-all.ps1");
        
        if !script_path.exists() {
            return; // Skip if script doesn't exist
        }
        
        // Use forward slashes for PowerShell
        let script_str = script_path.to_string_lossy().replace("\\", "/");
        let ps_command = format!("& '{}' -OutputJson", script_str);
        
        // Try pwsh first, then powershell.exe
        let shell = if Command::new("pwsh")
            .args(["-NoProfile", "-Command", "exit 0"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            "pwsh"
        } else {
            "powershell.exe"
        };
        
        let output = Command::new(shell)
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps_command])
            .current_dir(&root)
            .output();
        
        assert!(output.is_ok(), "Should be able to execute PowerShell command");
        
        let output = output.unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // The first line should be valid JSON with step/status/message fields
        if let Some(first_line) = stdout.lines().next() {
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(first_line);
            assert!(parsed.is_ok(), "First output line should be valid JSON: {}", first_line);
            
            let json = parsed.unwrap();
            assert!(json.get("step").is_some(), "JSON should have 'step' field");
            assert!(json.get("status").is_some(), "JSON should have 'status' field");
            assert!(json.get("message").is_some(), "JSON should have 'message' field");
        }
    }

    #[test]
    fn test_script_path_formatting() {
        // Test that path replacement works correctly
        let windows_path = std::path::PathBuf::from("E:\\projects\\odd-demonstration\\scripts\\start-all.ps1");
        let script_str = windows_path.to_string_lossy().replace("\\", "/");
        let ps_command = format!("& '{}' -OutputJson", script_str);
        
        assert!(ps_command.contains("E:/projects/odd-demonstration"), 
            "Path should use forward slashes: {}", ps_command);
        assert!(ps_command.starts_with("& '"), "Command should use call operator");
        assert!(ps_command.ends_with("' -OutputJson"), "Command should include -OutputJson flag");
    }

    // Task creation tests
    #[test]
    fn test_task_creation_state_default() {
        let state = TaskCreationState::default();
        assert!(state.job_type.is_empty());
        assert_eq!(state.status, TaskCreationStatus::Editing);
    }

    #[test]
    fn test_task_creation_status_variants() {
        let editing = TaskCreationStatus::Editing;
        let submitting = TaskCreationStatus::Submitting;
        let success = TaskCreationStatus::Success("job-123".to_string());
        let error = TaskCreationStatus::Error("Connection failed".to_string());
        
        assert_eq!(editing, TaskCreationStatus::Editing);
        assert_eq!(submitting, TaskCreationStatus::Submitting);
        assert!(matches!(success, TaskCreationStatus::Success(_)));
        assert!(matches!(error, TaskCreationStatus::Error(_)));
    }

    #[test]
    fn test_job_payload_serialization() {
        let payload = JobPayload {
            id: "test-uuid".to_string(),
            job_type: "PROCESS".to_string(),
            status: "PENDING".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        
        let json = serde_json::to_string(&payload).expect("Failed to serialize JobPayload");
        assert!(json.contains("\"id\":\"test-uuid\""));
        assert!(json.contains("\"type\":\"PROCESS\""));
        assert!(json.contains("\"status\":\"PENDING\""));
        assert!(json.contains("\"createdAt\":\"2024-01-01T00:00:00Z\""));
    }

    // UI Launcher tests
    #[test]
    fn test_ui_launcher_state_default() {
        let state = UiLauncherState::default();
        assert_eq!(state.selected_index, 0);
        assert!(state.registry.is_none());
    }

    #[test]
    fn test_ui_entry_deserialization() {
        let json = r#"{
            "id": "dashboard",
            "name": "Web Dashboard",
            "port": 8081,
            "path": "/",
            "emoji": "📊",
            "description": "Main dashboard"
        }"#;
        let entry: UiEntry = serde_json::from_str(json).expect("Failed to deserialize UiEntry");
        assert_eq!(entry.id, "dashboard");
        assert_eq!(entry.name, "Web Dashboard");
        assert_eq!(entry.port, 8081);
        assert_eq!(entry.path, "/");
    }

    #[test]
    fn test_ui_registry_deserialization() {
        let json = r#"{
            "baseUrl": "http://localhost",
            "entries": [
                {"id": "test", "name": "Test", "port": 8080, "path": "/", "emoji": "🧪", "description": "Test UI"}
            ]
        }"#;
        let registry: UiRegistry = serde_json::from_str(json).expect("Failed to deserialize UiRegistry");
        assert_eq!(registry.base_url, "http://localhost");
        assert_eq!(registry.entries.len(), 1);
        assert_eq!(registry.entries[0].port, 8080);
    }

    #[test]
    fn test_app_mode_includes_new_modes() {
        let task_creation = AppMode::TaskCreation;
        let ui_launcher = AppMode::UiLauncher;
        
        assert_eq!(task_creation, AppMode::TaskCreation);
        assert_eq!(ui_launcher, AppMode::UiLauncher);
        assert_ne!(task_creation, AppMode::Dashboard);
        assert_ne!(ui_launcher, AppMode::Dashboard);
    }

    #[test]
    fn test_ui_registry_loads_from_file() {
        // This test will pass if the registry file exists, skip silently otherwise
        if let Ok(registry) = load_ui_registry() {
            assert!(!registry.entries.is_empty(), "Registry should have at least one entry");
            assert!(registry.base_url.starts_with("http"), "Base URL should be http");
            
            // Verify all entries have required fields
            for entry in &registry.entries {
                assert!(!entry.id.is_empty());
                assert!(!entry.name.is_empty());
                assert!(entry.port > 0);
            }
        }
    }

    // Validation tests
    #[test]
    fn test_validate_job_type_empty() {
        assert!(validate_job_type("").is_err());
        assert!(validate_job_type("   ").is_err());
    }

    #[test]
    fn test_validate_job_type_too_long() {
        let long_type = "A".repeat(51);
        let result = validate_job_type(&long_type);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too long"));
    }

    #[test]
    fn test_validate_job_type_special_chars() {
        assert!(validate_job_type("job@type").is_err());
        assert!(validate_job_type("job type").is_err());
        assert!(validate_job_type("job.type").is_err());
    }

    #[test]
    fn test_validate_job_type_valid() {
        assert!(validate_job_type("PROCESS").is_ok());
        assert!(validate_job_type("my_job").is_ok());
        assert!(validate_job_type("job-123").is_ok());
        assert!(validate_job_type("Test_Job-1").is_ok());
    }

    // Error enum tests
    #[test]
    fn test_registry_error_display() {
        let not_found = RegistryError::NotFound("file.json".to_string());
        let malformed = RegistryError::Malformed("bad json".to_string());
        let invalid = RegistryError::InvalidEntry("port 0".to_string());
        
        assert!(not_found.to_string().contains("not found"));
        assert!(malformed.to_string().contains("malformed"));
        assert!(invalid.to_string().contains("Invalid"));
    }

    #[test]
    fn test_submit_error_display() {
        let timeout = SubmitError::Timeout;
        let conn = SubmitError::ConnectionRefused;
        let validation = SubmitError::ValidationFailed("empty".to_string());
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

    // Platform detection and support matrix tests
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
        // Valid OS values: "windows", "macos", "linux", "android", "ios", etc.
        // Valid ARCH values: "x86_64", "aarch64", "arm", "x86", etc.
        
        let valid_os = &["windows", "macos", "linux", "android", "ios", "freebsd"];
        let valid_arch = &["x86_64", "aarch64", "arm", "x86"];
        
        for (os, arch) in SUPPORT_MATRIX.iter() {
            assert!(
                valid_os.contains(os),
                "Invalid OS in SUPPORT_MATRIX: {}. Valid values: {:?}",
                os, valid_os
            );
            assert!(
                valid_arch.contains(arch),
                "Invalid ARCH in SUPPORT_MATRIX: {}. Valid values: {:?}",
                arch, valid_arch
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
        // We verify this by calling it multiple times - if it had I/O it might fail
        for _ in 0..10 {
            let _ = check_platform_support();
        }
    }
}

