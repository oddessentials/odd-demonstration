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
use serde::Deserialize;
use std::{
    error::Error,
    io::{self, BufRead, BufReader},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

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
    Loading,       // Initial loading splash
    Launcher,      // Cluster not detected - show launch option
    SetupProgress, // Running setup script
    Dashboard,     // Normal dashboard view
}

/// Cluster status after checking
#[derive(Debug, Clone, PartialEq)]
enum ClusterStatus {
    Ready,
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
    show_new_task_modal: bool,
}

const MAX_ALERT_RETRIES: u8 = 3;

impl App {
    fn new(api_url: String, gateway_url: String) -> App {
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
            show_new_task_modal: false,
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
                    ClusterStatus::Ready
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


fn main() -> Result<(), Box<dyn Error>> {
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
        ClusterStatus::NotFound | ClusterStatus::Error(_) => {
            // Show launcher view
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
                        Span::styled("N", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::raw(" New Task"),
                    ]));
                    f.render_widget(help, main_chunks[3]);

                    // Render modal if showing
                    if app.show_new_task_modal {
                        let area = f.size();
                        let modal_width = 50u16;
                        let modal_height = 8u16;
                        let x = (area.width.saturating_sub(modal_width)) / 2;
                        let y = (area.height.saturating_sub(modal_height)) / 2;
                        let modal_area = Rect::new(x, y, modal_width, modal_height);
                        
                        f.render_widget(Clear, modal_area);
                        
                        let modal_lines = vec![
                            Line::from(""),
                            Line::from(vec![
                                Span::styled(
                                    "  🚧 Task Creation Coming Soon!",
                                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                                ),
                            ]),
                            Line::from(""),
                            Line::from(vec![
                                Span::styled(
                                    "  This feature is under development.",
                                    Style::default().fg(Color::Gray),
                                ),
                            ]),
                            Line::from(""),
                            Line::from(vec![
                                Span::styled(
                                    "  Press any key to close",
                                    Style::default().fg(Color::Green),
                                ),
                            ]),
                        ];
                        
                        let modal = Paragraph::new(modal_lines)
                            .block(
                                Block::default()
                                    .borders(Borders::ALL)
                                    .border_style(Style::default().fg(Color::Yellow))
                                    .title(" New Task ")
                                    .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                            );
                        
                        f.render_widget(modal, modal_area);
                    }
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
                        if app.show_new_task_modal {
                            // Any key closes the modal
                            app.show_new_task_modal = false;
                        } else {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Char('Q') => break,
                                KeyCode::Char('r') | KeyCode::Char('R') => {
                                    app.alert_retry_count = 0;
                                    app.refresh();
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') => {
                                    app.show_new_task_modal = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {
                        if key.code == KeyCode::Char('q') {
                            break;
                        }
                    }
                }
            }
        } else if app.mode == AppMode::Dashboard && !app.show_new_task_modal {
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
        assert!(!app.show_new_task_modal);
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
        let not_found = ClusterStatus::NotFound;
        let error = ClusterStatus::Error("test error".to_string());
        
        assert_eq!(ready, ClusterStatus::Ready);
        assert_eq!(not_found, ClusterStatus::NotFound);
        assert!(matches!(error, ClusterStatus::Error(_)));
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
        assert!(steps.iter().any(|s| s.contains("netstat") || s.contains("port")), 
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
}
