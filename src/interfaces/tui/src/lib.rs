//! odd-dashboard TUI library
//!
//! This library provides the modular components for the odd-dashboard
//! terminal user interface.

pub mod cluster;
pub mod config;
pub mod doctor;
pub mod error;
pub mod install;
pub mod types;

// Re-export commonly used items for convenience
pub use types::{
    Alert, AlertLabels, App, AppMode, ClusterStatus, InstallAction, Job, JobPayload,
    PortForwardRegistry, PortForwardStatus, PrereqStatus, Prerequisite, PrerequisiteSetupState,
    SetupProgress, ShutdownProgress, Stats, TaskCreationState, TaskCreationStatus, UiEntry,
    UiLauncherState, UiRegistry, APP_VERSION, CLUSTER_NAME, KUBECTL_CONTEXT, LOADING_MESSAGES,
    LOGO, MAX_ALERT_RETRIES, SPINNER_FRAMES,
};

pub use error::{
    get_docker_install_steps, get_error_hint, get_generic_error_steps, get_install_command,
    get_kind_install_steps, get_kubectl_install_steps, get_port_conflict_steps,
    get_pwsh_install_steps, get_remediation_steps, BrowserError, RegistryError, SubmitError,
};

pub use doctor::{
    check_all_prerequisites, check_command_version, check_platform_support,
    has_missing_prerequisites, missing_prereq_count, print_help, print_version, run_doctor,
    SUPPORT_MATRIX, SUPPORT_MATRIX_URL,
};

pub use cluster::{
    check_cluster_status, check_pods_status, check_port_forwards, delete_cluster,
    ensure_port_forwards, find_project_root, load_ui_registry, open_browser, run_setup_script,
    run_shutdown, start_port_forward_tracked, stop_port_forwards, submit_job, validate_job_type,
};

pub use install::{
    copy_to_clipboard, execute_install_with_output, get_install_description, InstallOutput,
};

pub use config::is_server_mode;
