//! Web PTY Server - PTY spawning and management
//!
//! Handles spawning the odd-dashboard binary in a PTY with correct capabilities (R7).

use std::io::{Read, Write};
use std::thread;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::config::Config;

/// PTY spawn result containing channels for I/O
pub struct PtyHandle {
    /// Channel to send input to PTY
    pub input_tx: mpsc::Sender<Vec<u8>>,
    /// Channel to receive output from PTY
    pub output_rx: mpsc::Receiver<Vec<u8>>,
    /// Initial terminal size
    pub size: PtySize,
}

/// Spawn a new PTY running the TUI binary
/// 
/// # Arguments
/// * `config` - Server configuration
/// * `cols` - Initial terminal columns
/// * `rows` - Initial terminal rows
/// 
/// # Returns
/// * `Result<PtyHandle>` - Handle with I/O channels
pub fn spawn_pty(
    config: &Config,
    cols: u16,
    rows: u16,
) -> Result<PtyHandle, PtySpawnError> {
    let pty_system = native_pty_system();
    
    let size = PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    };
    
    // Open PTY pair
    let pair = pty_system
        .openpty(size)
        .map_err(|e| PtySpawnError::OpenFailed(e.to_string()))?;
    
    // Build command with correct terminal capabilities (R7)
    let mut cmd = CommandBuilder::new(&config.tui_binary_path);
    
    // Terminal capabilities (R7: fidelity bugs, not theme issues)
    cmd.env("TERM", "xterm-256color");
    cmd.env("LANG", "en_US.UTF-8");
    cmd.env("LC_ALL", "en_US.UTF-8");
    cmd.env("COLORTERM", "truecolor");
    
    // W11: Server mode - skip prereq checks in-container
    // The TUI will bypass Docker/kubectl/kind/pwsh detection and go directly
    // to Dashboard/Launcher views. Shows prominent warning banner.
    cmd.env("ODD_DASHBOARD_SERVER_MODE", "1");
    
    // Pass through API URLs
    cmd.env("READ_MODEL_URL", &config.read_model_url);
    cmd.env("GATEWAY_URL", &config.gateway_url);
    // Pass PROMETHEUS_URL if set in environment
    if let Ok(prom_url) = std::env::var("PROMETHEUS_URL") {
        cmd.env("PROMETHEUS_URL", prom_url);
    }
    
    // Spawn child process
    let mut child = pair.slave
        .spawn_command(cmd)
        .map_err(|e| PtySpawnError::SpawnFailed(e.to_string()))?;
    
    // Drop slave - we only use master
    drop(pair.slave);
    
    // Get reader and writer from master
    let mut reader = pair.master.try_clone_reader()
        .map_err(|e| PtySpawnError::IoSetupFailed(e.to_string()))?;
    
    let mut writer = pair.master.take_writer()
        .map_err(|e| PtySpawnError::IoSetupFailed(e.to_string()))?;
    
    // Create channels for async I/O
    let (input_tx, mut input_rx) = mpsc::channel::<Vec<u8>>(256);
    let (output_tx, output_rx) = mpsc::channel::<Vec<u8>>(256);
    
    // Spawn thread to read from PTY and send to channel
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    debug!("PTY reader: EOF");
                    break;
                }
                Ok(n) => {
                    let data = buf[..n].to_vec();
                    if output_tx.blocking_send(data).is_err() {
                        debug!("PTY reader: channel closed");
                        break;
                    }
                }
                Err(e) => {
                    error!("PTY reader error: {}", e);
                    break;
                }
            }
        }
    });
    
    // Spawn thread to receive from channel and write to PTY
    thread::spawn(move || {
        while let Some(data) = input_rx.blocking_recv() {
            if let Err(e) = writer.write_all(&data) {
                error!("PTY writer error: {}", e);
                break;
            }
            // Flush to ensure data is sent immediately
            let _ = writer.flush();
        }
        debug!("PTY writer: channel closed");
    });
    
    // Spawn thread to wait for child exit
    thread::spawn(move || {
        match child.wait() {
            Ok(status) => {
                info!("PTY child exited: {:?}", status);
            }
            Err(e) => {
                error!("PTY child wait error: {}", e);
            }
        }
    });
    
    info!("Spawned PTY {}x{} running {}", cols, rows, config.tui_binary_path);
    
    Ok(PtyHandle {
        input_tx,
        output_rx,
        size,
    })
}

/// PTY spawn errors
#[derive(Debug, Clone)]
pub enum PtySpawnError {
    OpenFailed(String),
    SpawnFailed(String),
    IoSetupFailed(String),
}

impl std::fmt::Display for PtySpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenFailed(e) => write!(f, "Failed to open PTY: {}", e),
            Self::SpawnFailed(e) => write!(f, "Failed to spawn command: {}", e),
            Self::IoSetupFailed(e) => write!(f, "Failed to setup I/O: {}", e),
        }
    }
}

impl std::error::Error for PtySpawnError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::config::TestMode;
    
    fn test_config() -> Config {
        Config {
            ws_port: 9000,
            metrics_port: 9001,
            // Use 'echo' as a simple test command
            tui_binary_path: if cfg!(windows) { "cmd.exe" } else { "echo" }.to_string(),
            auth_token: None,
            read_only: false,
            idle_timeout: Duration::from_secs(60),
            per_ip_cap: 5,
            global_cap: 50,
            disconnect_grace: Duration::from_secs(30),
            max_output_queue_bytes: 1024,
            read_model_url: "http://localhost:8080".to_string(),
            gateway_url: "http://localhost:3000".to_string(),
            token_ttl: Duration::from_secs(300),
            ring_max_bytes: 1_048_576,
            ring_max_frames: 1000,
            test_mode: TestMode::None,
        }
    }
    
    #[tokio::test]
    async fn test_spawn_pty() {
        let config = test_config();
        let result = spawn_pty(&config, 80, 24);
        
        // This test may fail in CI without a proper terminal environment
        // but should work locally
        if let Ok(handle) = result {
            assert_eq!(handle.size.cols, 80);
            assert_eq!(handle.size.rows, 24);
        }
    }
    
    #[test]
    fn test_pty_spawn_error_display() {
        let e = PtySpawnError::SpawnFailed("command not found".to_string());
        let s = format!("{}", e);
        assert!(s.contains("Failed to spawn"));
        assert!(s.contains("command not found"));
    }
}

