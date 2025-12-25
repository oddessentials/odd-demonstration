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
    pub message: Option<String>,         // Feedback message for user
    pub output_lines: Vec<String>,       // Captured install output for display
    pub is_installing: bool,             // Whether install is currently running
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
        if let Ok(response) = reqwest::blocking::get(format!("{}/jobs/recent", self.api_url)) {
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

    // ========== PrerequisiteSetupState Tests ==========

    #[test]
    fn test_prerequisite_setup_state_default() {
        let state = PrerequisiteSetupState::default();
        assert!(state.prerequisites.is_empty());
        assert_eq!(state.selected_index, 0);
        assert!(state.install_action.is_none());
        assert!(state.current_install.is_none());
        assert!(state.message.is_none());
        assert!(state.output_lines.is_empty());
        assert!(!state.is_installing);
    }

    #[test]
    fn test_prerequisite_setup_state_with_output() {
        let mut state = PrerequisiteSetupState::default();
        state.output_lines = vec![
            "Downloading...".to_string(),
            "Installing...".to_string(),
            "Done!".to_string(),
        ];
        assert_eq!(state.output_lines.len(), 3);
        assert!(state.output_lines[0].contains("Downloading"));
    }

    #[test]
    fn test_prerequisite_setup_state_installing_flag() {
        let mut state = PrerequisiteSetupState::default();
        assert!(!state.is_installing);
        state.is_installing = true;
        assert!(state.is_installing);
    }

    #[test]
    fn test_prerequisite_setup_state_message() {
        let mut state = PrerequisiteSetupState::default();
        state.message = Some("✓ Docker installed successfully!".to_string());
        assert!(state.message.is_some());
        assert!(state.message.as_ref().unwrap().starts_with("✓"));
    }

    #[test]
    fn test_prerequisite_setup_state_selected_navigation() {
        let mut state = PrerequisiteSetupState::default();
        state.prerequisites = vec![
            Prerequisite {
                name: "docker".to_string(),
                version: None,
                status: PrereqStatus::Missing,
                install_cmd: vec![],
            },
            Prerequisite {
                name: "kubectl".to_string(),
                version: None,
                status: PrereqStatus::Missing,
                install_cmd: vec![],
            },
        ];
        assert_eq!(state.selected_index, 0);
        state.selected_index = 1;
        assert_eq!(state.selected_index, 1);
        assert_eq!(state.prerequisites[state.selected_index].name, "kubectl");
    }

    #[test]
    fn test_prerequisite_clone() {
        let prereq = Prerequisite {
            name: "docker".to_string(),
            version: Some("24.0.0".to_string()),
            status: PrereqStatus::Installed,
            install_cmd: vec!["brew install docker".to_string()],
        };
        let cloned = prereq.clone();
        assert_eq!(cloned.name, prereq.name);
        assert_eq!(cloned.version, prereq.version);
        assert_eq!(cloned.status, prereq.status);
        assert_eq!(cloned.install_cmd, prereq.install_cmd);
    }

    // ========== API Endpoint Contract Tests ==========
    // These tests document the expected API contracts between TUI and read-model

    #[test]
    fn test_api_endpoints_contract() {
        // Document the expected read-model API endpoints
        // The refresh() method in App uses these endpoints
        let expected_stats_endpoint = "/stats";
        let expected_jobs_endpoint = "/jobs/recent";  // NOT /jobs - see read-model main.go
        
        assert_eq!(expected_stats_endpoint, "/stats");
        assert_eq!(expected_jobs_endpoint, "/jobs/recent");
    }

    #[test]
    fn test_job_struct_matches_read_model_response() {
        // Verify Job struct can deserialize read-model response format
        let json = r#"{"id":"550e8400-e29b-41d4-a716-446655440099","type":"TEST","status":"COMPLETED","createdAt":"2025-01-01T12:00:00Z"}"#;
        let job: Result<Job, _> = serde_json::from_str(json);
        assert!(job.is_ok());
        let job = job.unwrap();
        assert_eq!(job.id, "550e8400-e29b-41d4-a716-446655440099");
        assert_eq!(job.job_type, "TEST");
        assert_eq!(job.status, "COMPLETED");
    }

    #[test]
    fn test_stats_struct_matches_read_model_response() {
        // Verify Stats struct can deserialize read-model response format
        let json = r#"{"totalJobs":10,"completedJobs":8,"failedJobs":2,"lastEventTime":"2025-01-01T12:00:00Z"}"#;
        let stats: Result<Stats, _> = serde_json::from_str(json);
        assert!(stats.is_ok());
        let stats = stats.unwrap();
        assert_eq!(stats.total_jobs, 10);
        assert_eq!(stats.completed_jobs, 8);
        assert_eq!(stats.failed_jobs, 2);
    }

    // ========== JobPayload Serialization Tests ==========

    #[test]
    fn test_job_payload_serialization() {
        let payload = JobPayload {
            id: "test-id-123".to_string(),
            job_type: "TEST_JOB".to_string(),
            status: "PENDING".to_string(),
            created_at: "2025-01-01T12:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"id\":\"test-id-123\""));
        assert!(json.contains("\"type\":\"TEST_JOB\""));
        assert!(json.contains("\"status\":\"PENDING\""));
        assert!(json.contains("\"createdAt\":\"2025-01-01T12:00:00Z\""));
    }

    #[test]
    fn test_job_payload_field_rename() {
        // Verify serde rename attributes work correctly
        let payload = JobPayload {
            id: "id".to_string(),
            job_type: "TYPE".to_string(),
            status: "status".to_string(),
            created_at: "time".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        // Must use camelCase per Gateway API contract
        assert!(json.contains("\"type\""));
        assert!(json.contains("\"createdAt\""));
        assert!(!json.contains("\"job_type\""));
        assert!(!json.contains("\"created_at\""));
    }

    // ========== UiEntry Deserialization Tests ==========

    #[test]
    fn test_ui_entry_deserialization() {
        let json = r#"{"id":"grafana","name":"Grafana","port":3001,"path":"/","emoji":"📊","description":"Metrics"}"#;
        let entry: UiEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.id, "grafana");
        assert_eq!(entry.name, "Grafana");
        assert_eq!(entry.port, 3001);
        assert_eq!(entry.path, "/");
        assert_eq!(entry.emoji, "📊");
        assert_eq!(entry.description, "Metrics");
    }

    #[test]
    fn test_ui_entry_clone() {
        let entry = UiEntry {
            id: "test".to_string(),
            name: "Test".to_string(),
            port: 8080,
            path: "/test".to_string(),
            emoji: "🧪".to_string(),
            description: "Test entry".to_string(),
        };
        let cloned = entry.clone();
        assert_eq!(cloned.id, entry.id);
        assert_eq!(cloned.port, entry.port);
    }

    // ========== UiRegistry Deserialization Tests ==========

    #[test]
    fn test_ui_registry_deserialization() {
        let json = r#"{"baseUrl":"http://localhost","entries":[{"id":"test","name":"Test","port":80,"path":"/","emoji":"🔧","description":"Test"}]}"#;
        let registry: UiRegistry = serde_json::from_str(json).unwrap();
        assert_eq!(registry.base_url, "http://localhost");
        assert_eq!(registry.entries.len(), 1);
        assert_eq!(registry.entries[0].id, "test");
    }

    #[test]
    fn test_ui_registry_empty_entries() {
        let json = r#"{"baseUrl":"http://localhost","entries":[]}"#;
        let registry: UiRegistry = serde_json::from_str(json).unwrap();
        assert!(registry.entries.is_empty());
    }

    // ========== AlertLabels and Alert Tests ==========

    #[test]
    fn test_alert_labels_all_optional() {
        let json = r#"{}"#;
        let labels: AlertLabels = serde_json::from_str(json).unwrap();
        assert!(labels.alertname.is_none());
        assert!(labels.severity.is_none());
        assert!(labels.service.is_none());
    }

    #[test]
    fn test_alert_labels_partial() {
        let json = r#"{"alertname":"TestAlert"}"#;
        let labels: AlertLabels = serde_json::from_str(json).unwrap();
        assert_eq!(labels.alertname, Some("TestAlert".to_string()));
        assert!(labels.severity.is_none());
    }

    #[test]
    fn test_alert_full() {
        let json = r#"{"labels":{"alertname":"HighCPU","severity":"warning","service":"processor"}}"#;
        let alert: Alert = serde_json::from_str(json).unwrap();
        assert_eq!(alert.labels.alertname, Some("HighCPU".to_string()));
        assert_eq!(alert.labels.severity, Some("warning".to_string()));
        assert_eq!(alert.labels.service, Some("processor".to_string()));
    }

    #[test]
    fn test_alert_clone() {
        let alert = Alert {
            labels: AlertLabels {
                alertname: Some("Test".to_string()),
                severity: None,
                service: None,
            },
        };
        let cloned = alert.clone();
        assert_eq!(cloned.labels.alertname, alert.labels.alertname);
    }

    // ========== TaskCreationStatus Tests ==========

    #[test]
    fn test_task_creation_status_success() {
        let status = TaskCreationStatus::Success("job-123".to_string());
        if let TaskCreationStatus::Success(id) = status {
            assert_eq!(id, "job-123");
        } else {
            panic!("Expected Success variant");
        }
    }

    #[test]
    fn test_task_creation_status_error() {
        let status = TaskCreationStatus::Error("connection failed".to_string());
        if let TaskCreationStatus::Error(msg) = status {
            assert!(msg.contains("connection"));
        } else {
            panic!("Expected Error variant");
        }
    }

    #[test]
    fn test_task_creation_status_submitting() {
        let status = TaskCreationStatus::Submitting;
        assert_eq!(status, TaskCreationStatus::Submitting);
    }

    // ========== ClusterStatus Tests ==========

    #[test]
    fn test_cluster_status_error_message() {
        let status = ClusterStatus::Error("kubectl not found".to_string());
        if let ClusterStatus::Error(msg) = status {
            assert!(msg.contains("kubectl"));
        } else {
            panic!("Expected Error variant");
        }
    }

    // ========== UiLauncherState Tests ==========

    #[test]
    fn test_ui_launcher_state_default() {
        let state = UiLauncherState::default();
        assert_eq!(state.selected_index, 0);
        assert!(state.registry.is_none());
        assert!(state.error.is_none());
    }

    #[test]
    fn test_ui_launcher_state_with_error() {
        let mut state = UiLauncherState::default();
        state.error = Some("Browser unavailable".to_string());
        assert!(state.error.is_some());
        assert!(state.error.unwrap().contains("Browser"));
    }

    // ========== SetupProgress Tests ==========

    #[test]
    fn test_setup_progress_fields() {
        let mut progress = SetupProgress::default();
        progress.current_step = "deploying".to_string();
        progress.current_status = "running".to_string();
        progress.message = "Deploying services...".to_string();
        progress.error_hint = "Check Docker".to_string();
        progress.remediation = vec!["Restart Docker".to_string()];
        
        assert_eq!(progress.current_step, "deploying");
        assert_eq!(progress.current_status, "running");
        assert!(!progress.remediation.is_empty());
    }

    #[test]
    fn test_setup_progress_start_time() {
        let mut progress = SetupProgress::default();
        assert!(progress.start_time.is_none());
        progress.start_time = Some(std::time::Instant::now());
        assert!(progress.start_time.is_some());
    }

    // ========== Constants Validation Tests ==========

    #[test]
    fn test_spinner_frames_are_braille() {
        for frame in SPINNER_FRAMES {
            // Braille characters are in Unicode range U+2800..U+28FF
            for c in frame.chars() {
                let code = c as u32;
                assert!(code >= 0x2800 && code <= 0x28FF, "Expected Braille character");
            }
        }
    }

    #[test]
    fn test_loading_messages_not_empty() {
        for msg in LOADING_MESSAGES {
            assert!(!msg.is_empty());
            assert!(msg.len() > 5);
        }
    }

    #[test]
    fn test_app_version_is_semver_like() {
        // APP_VERSION should look like X.Y.Z
        assert!(!APP_VERSION.is_empty());
        let parts: Vec<&str> = APP_VERSION.split('.').collect();
        assert!(parts.len() >= 2, "Version should have at least major.minor");
    }

    #[test]
    fn test_max_alert_retries_reasonable() {
        assert!(MAX_ALERT_RETRIES >= 1);
        assert!(MAX_ALERT_RETRIES <= 10);
    }
}

