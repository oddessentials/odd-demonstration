//! Type definitions for the odd-dashboard TUI
//! 
//! This module contains all shared data structures, enums, and constants
//! used across the TUI application.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

// ============================================================================
// Constants
// ============================================================================

/// ASCII art logo for the Distributed Task Observatory
pub const LOGO: &str = r#"
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
pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Loading messages that cycle for visual interest
pub const LOADING_MESSAGES: &[&str] = &[
    "Connecting to services",
    "Fetching statistics",
    "Loading job data",
    "Checking alerts",
];

/// Application version from VERSION file (set by build.rs)
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum retries for alert fetching
pub const MAX_ALERT_RETRIES: u8 = 3;

// ============================================================================
// Application State Types
// ============================================================================

/// Application mode - controls which view is displayed
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Loading,           // Initial loading splash
    Launcher,          // Cluster not detected - show launch option
    SetupProgress,     // Running setup script
    Dashboard,         // Normal dashboard view
    TaskCreation,      // Task creation modal
    UiLauncher,        // UI launcher selection
    PrerequisiteSetup, // Guided prerequisite installation
}

/// Cluster status after checking
#[derive(Debug, Clone, PartialEq)]
pub enum ClusterStatus {
    Ready,
    NoPods,  // Cluster exists but no application pods deployed
    NotFound,
    Error(String),
}

/// Setup progress tracking
#[derive(Debug, Clone, Default)]
pub struct SetupProgress {
    pub current_step: String,
    pub current_status: String,
    pub message: String,
    pub error_hint: String,       // Actionable hint for fixing the error
    pub remediation: Vec<String>, // Step-by-step remediation commands
    pub is_complete: bool,
    pub has_error: bool,
    pub log_lines: Vec<String>,
    pub start_time: Option<std::time::Instant>,  // For elapsed time tracking
}

// ============================================================================
// Task Creation Types
// ============================================================================

/// Task creation state
#[derive(Debug, Clone, PartialEq)]
pub enum TaskCreationStatus {
    Editing,
    Submitting,
    Success(String),  // contains job_id
    Error(String),    // contains error message
}

#[derive(Debug, Clone)]
pub struct TaskCreationState {
    pub job_type: String,
    pub status: TaskCreationStatus,
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
pub struct JobPayload {
    pub id: String,
    #[serde(rename = "type")]
    pub job_type: String,
    pub status: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

// ============================================================================
// UI Registry Types
// ============================================================================

/// UI Registry entry from contracts/ui-registry.json
#[derive(Deserialize, Debug, Clone)]
pub struct UiEntry {
    pub id: String,
    pub name: String,
    pub port: u16,
    pub path: String,
    pub emoji: String,
    pub description: String,
}

/// UI Registry containing all launchable UIs
#[derive(Deserialize, Debug, Clone)]
pub struct UiRegistry {
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    pub entries: Vec<UiEntry>,
}

/// UI Launcher state
#[derive(Debug, Clone, Default)]
pub struct UiLauncherState {
    pub selected_index: usize,
    pub registry: Option<UiRegistry>,
    pub error: Option<String>,  // For displaying browser/registry errors
}

// ============================================================================
// API Response Types
// ============================================================================

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Stats {
    #[serde(rename = "totalJobs")]
    pub total_jobs: i64,
    #[serde(rename = "completedJobs")]
    pub completed_jobs: i64,
    #[serde(rename = "failedJobs")]
    pub failed_jobs: i64,
    #[serde(rename = "lastEventTime")]
    pub last_event_time: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Job {
    pub id: String,
    #[serde(rename = "type")]
    pub job_type: String,
    pub status: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AlertLabels {
    pub alertname: Option<String>,
    pub severity: Option<String>,
    pub service: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Alert {
    pub labels: AlertLabels,
}

// ============================================================================
// Prerequisite Types (for guided installation)
// ============================================================================

/// Prerequisite installation status
#[derive(Debug, Clone, PartialEq)]
pub enum PrereqStatus {
    Installed,
    Missing,
    Installing,
    InstallFailed(String),
}

/// A prerequisite tool that may need installation
#[derive(Debug, Clone)]
pub struct Prerequisite {
    pub name: String,
    pub status: PrereqStatus,
    pub version: Option<String>,
    pub install_cmd: Vec<String>,  // Platform-specific install commands
}

/// How to handle install command for user
#[derive(Debug, Clone, PartialEq)]
pub enum InstallAction {
    Execute,      // Run the command directly
    CopyToClipboard,  // Copy command to clipboard
}

/// State for prerequisite setup view
#[derive(Debug, Clone, Default)]
pub struct PrerequisiteSetupState {
    pub prerequisites: Vec<Prerequisite>,
    pub selected_index: usize,
    pub install_action: Option<InstallAction>,
    pub current_install: Option<usize>,  // Index of currently installing prereq
}

// ============================================================================
// Main Application State
// ============================================================================

pub struct App {
    pub mode: AppMode,
    pub stats: Stats,
    pub jobs: Vec<Job>,
    pub alerts: Vec<Alert>,
    pub alerts_error: Option<String>,
    pub api_url: String,
    pub gateway_url: String,
    pub alert_retry_count: u8,
    pub setup_progress: Arc<Mutex<SetupProgress>>,
    pub task_state: TaskCreationState,
    pub launcher_state: UiLauncherState,
    pub prereq_state: PrerequisiteSetupState,
}

impl App {
    pub fn new(api_url: String, gateway_url: String) -> App {
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
            launcher_state: UiLauncherState::default(),
            prereq_state: PrerequisiteSetupState::default(),
        }
    }

    pub fn refresh(&mut self) {
        // Fetch stats
        if let Ok(response) = reqwest::blocking::get(format!("{}/stats", self.api_url)) {
            if let Ok(stats) = response.json::<Stats>() {
                self.stats = stats;
            }
        }

        // Fetch jobs
        if let Ok(response) = reqwest::blocking::get(format!("{}/jobs", self.api_url)) {
            if let Ok(jobs) = response.json::<Vec<Job>>() {
                self.jobs = jobs;
            }
        }

        // Fetch alerts from Prometheus with retry logic
        let alerts_url = "http://localhost:9090/api/v1/alerts";
        match reqwest::blocking::Client::new()
            .get(alerts_url)
            .timeout(std::time::Duration::from_secs(2))
            .send()
        {
            Ok(response) => {
                if let Ok(body) = response.text() {
                    // Parse Prometheus alert response format
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(alerts) = json["data"]["alerts"].as_array() {
                            self.alerts = alerts
                                .iter()
                                .filter_map(|a| serde_json::from_value(a.clone()).ok())
                                .collect();
                            self.alerts_error = None;
                            self.alert_retry_count = 0;
                        }
                    }
                }
            }
            Err(_e) => {
                self.alert_retry_count += 1;
                if self.alert_retry_count >= MAX_ALERT_RETRIES {
                    self.alerts_error = Some(format!("Prometheus unavailable (retried {}x)", MAX_ALERT_RETRIES));
                } else {
                    self.alerts_error = Some(format!("Connecting to Prometheus... (attempt {})", self.alert_retry_count));
                }
            }
        }
    }
}

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
        let app = App::new("http://test:8080".to_string(), "http://test:3000".to_string());
        assert_eq!(app.mode, AppMode::Loading);
        assert_eq!(app.api_url, "http://test:8080");
        assert_eq!(app.gateway_url, "http://test:3000");
    }

    #[test]
    fn test_app_mode_variants() {
        // Ensure all variants are covered
        let modes = vec![
            AppMode::Loading,
            AppMode::Launcher,
            AppMode::SetupProgress,
            AppMode::Dashboard,
            AppMode::TaskCreation,
            AppMode::UiLauncher,
            AppMode::PrerequisiteSetup,
        ];
        assert_eq!(modes.len(), 7);
    }

    #[test]
    fn test_prereq_status_variants() {
        let installed = PrereqStatus::Installed;
        let missing = PrereqStatus::Missing;
        let installing = PrereqStatus::Installing;
        let failed = PrereqStatus::InstallFailed("error".to_string());
        
        assert_eq!(installed, PrereqStatus::Installed);
        assert_eq!(missing, PrereqStatus::Missing);
        assert_eq!(installing, PrereqStatus::Installing);
        assert!(matches!(failed, PrereqStatus::InstallFailed(_)));
    }

    #[test]
    fn test_install_action_variants() {
        assert_eq!(InstallAction::Execute, InstallAction::Execute);
        assert_eq!(InstallAction::CopyToClipboard, InstallAction::CopyToClipboard);
        assert_ne!(InstallAction::Execute, InstallAction::CopyToClipboard);
    }

    #[test]
    fn test_task_creation_state_default() {
        let state = TaskCreationState::default();
        assert!(state.job_type.is_empty());
        assert_eq!(state.status, TaskCreationStatus::Editing);
    }

    #[test]
    fn test_setup_progress_default() {
        let progress = SetupProgress::default();
        assert!(!progress.is_complete);
        assert!(!progress.has_error);
        assert!(progress.log_lines.is_empty());
    }

    #[test]
    fn test_logo_is_not_empty() {
        assert!(!LOGO.is_empty());
        assert!(LOGO.len() > 50);
    }

    #[test]
    fn test_spinner_frames_count() {
        assert_eq!(SPINNER_FRAMES.len(), 10);
    }

    #[test]
    fn test_loading_messages_count() {
        assert!(LOADING_MESSAGES.len() >= 4);
    }
}
