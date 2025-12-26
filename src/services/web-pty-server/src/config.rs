//! Web PTY Server - Configuration module
//!
//! Handles environment-driven configuration with startup logging.

use std::env;
use std::time::Duration;
use tracing::info;

/// Server configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct Config {
    /// WebSocket server port
    pub ws_port: u16,
    /// Metrics/health server port
    pub metrics_port: u16,
    /// Path to odd-dashboard binary
    pub tui_binary_path: String,
    /// Authentication token for WebSocket connections
    pub auth_token: Option<String>,
    /// Read-only mode (blocks mutating inputs)
    pub read_only: bool,
    /// Idle timeout before session cleanup
    pub idle_timeout: Duration,
    /// Maximum sessions per client IP
    pub per_ip_cap: usize,
    /// Maximum total sessions globally
    pub global_cap: usize,
    /// Grace period after disconnect before reaping
    pub disconnect_grace: Duration,
    /// Maximum output queue size in bytes
    pub max_output_queue_bytes: usize,
    /// Read Model URL for fallback stats
    pub read_model_url: String,
    /// Gateway URL 
    pub gateway_url: String,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let config = Self {
            ws_port: env::var("PTY_WS_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(9000),
            metrics_port: env::var("PTY_METRICS_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(9001),
            tui_binary_path: env::var("PTY_TUI_BINARY")
                .unwrap_or_else(|_| "odd-dashboard".to_string()),
            auth_token: env::var("PTY_AUTH_TOKEN").ok(),
            read_only: env::var("PTY_READ_ONLY")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            idle_timeout: Duration::from_secs(
                env::var("PTY_IDLE_TIMEOUT_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1800),
            ),
            per_ip_cap: env::var("PTY_PER_IP_CAP")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            global_cap: env::var("PTY_GLOBAL_CAP")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(50),
            disconnect_grace: Duration::from_secs(
                env::var("PTY_DISCONNECT_GRACE_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30),
            ),
            max_output_queue_bytes: env::var("PTY_MAX_OUTPUT_QUEUE_BYTES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1_048_576), // 1MB
            read_model_url: env::var("READ_MODEL_URL")
                .unwrap_or_else(|_| "http://read-model:8080".to_string()),
            gateway_url: env::var("GATEWAY_URL")
                .unwrap_or_else(|_| "http://gateway:3000".to_string()),
        };
        
        config
    }
    
    /// Log configuration at startup (R3 requirement: operators confirm from logs)
    /// Note: Auth token is NEVER logged (R5 requirement)
    pub fn log_startup(&self) {
        info!(
            "PTY config: ws_port={}, metrics_port={}, idle={}s, per_ip={}, global={}, grace={}s, queue={}B, read_only={}",
            self.ws_port,
            self.metrics_port,
            self.idle_timeout.as_secs(),
            self.per_ip_cap,
            self.global_cap,
            self.disconnect_grace.as_secs(),
            self.max_output_queue_bytes,
            self.read_only
        );
        // Note: auth_token intentionally NOT logged
        if self.auth_token.is_some() {
            info!("PTY auth: enabled (token configured)");
        } else {
            info!("PTY auth: disabled (no token configured)");
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::from_env()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_defaults() {
        // Clear env vars that might interfere
        env::remove_var("PTY_WS_PORT");
        env::remove_var("PTY_IDLE_TIMEOUT_SECS");
        env::remove_var("PTY_AUTH_TOKEN");
        
        let config = Config::from_env();
        
        assert_eq!(config.ws_port, 9000);
        assert_eq!(config.metrics_port, 9001);
        assert_eq!(config.idle_timeout, Duration::from_secs(1800));
        assert_eq!(config.per_ip_cap, 5);
        assert_eq!(config.global_cap, 50);
        assert_eq!(config.disconnect_grace, Duration::from_secs(30));
        assert_eq!(config.max_output_queue_bytes, 1_048_576);
        assert!(!config.read_only);
    }
    
    #[test]
    fn test_config_read_only_parsing() {
        env::set_var("PTY_READ_ONLY", "true");
        let config = Config::from_env();
        assert!(config.read_only);
        
        env::set_var("PTY_READ_ONLY", "1");
        let config = Config::from_env();
        assert!(config.read_only);
        
        env::set_var("PTY_READ_ONLY", "false");
        let config = Config::from_env();
        assert!(!config.read_only);
        
        env::remove_var("PTY_READ_ONLY");
    }
    
    #[test]
    fn test_config_custom_values() {
        env::set_var("PTY_WS_PORT", "8888");
        env::set_var("PTY_IDLE_TIMEOUT_SECS", "3600");
        env::set_var("PTY_PER_IP_CAP", "10");
        
        let config = Config::from_env();
        
        assert_eq!(config.ws_port, 8888);
        assert_eq!(config.idle_timeout, Duration::from_secs(3600));
        assert_eq!(config.per_ip_cap, 10);
        
        // Cleanup
        env::remove_var("PTY_WS_PORT");
        env::remove_var("PTY_IDLE_TIMEOUT_SECS");
        env::remove_var("PTY_PER_IP_CAP");
    }
}
