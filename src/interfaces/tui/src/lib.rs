//! odd-dashboard TUI library
//!
//! This library provides the modular components for the odd-dashboard
//! terminal user interface.

pub mod types;
pub mod error;
pub mod doctor;
pub mod cluster;
pub mod install;

// Re-export commonly used items for convenience
pub use types::{
    App, AppMode, ClusterStatus, SetupProgress, 
    TaskCreationState, TaskCreationStatus, JobPayload,
    UiEntry, UiRegistry, UiLauncherState,
    Stats, Job, Alert, AlertLabels,
    Prerequisite, PrereqStatus, InstallAction, PrerequisiteSetupState,
    LOGO, SPINNER_FRAMES, LOADING_MESSAGES, APP_VERSION, MAX_ALERT_RETRIES,
};

pub use error::{
    RegistryError, SubmitError, BrowserError,
    get_error_hint, get_remediation_steps,
    get_docker_install_steps, get_pwsh_install_steps, 
    get_kind_install_steps, get_kubectl_install_steps,
    get_port_conflict_steps, get_generic_error_steps,
    get_install_command,
};

pub use doctor::{
    SUPPORT_MATRIX, SUPPORT_MATRIX_URL,
    check_platform_support, check_command_version,
    print_version, print_help, run_doctor,
    check_all_prerequisites, has_missing_prerequisites, missing_prereq_count,
};

pub use cluster::{
    check_cluster_status, check_pods_status,
    find_project_root, run_setup_script,
    load_ui_registry, open_browser,
    validate_job_type, submit_job,
};

pub use install::{
    InstallResult,
    copy_to_clipboard, execute_install, get_install_description,
};
