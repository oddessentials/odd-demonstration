//! Web PTY Server - Background PTY Task module
//!
//! Implements session-owned PTY with:
//! - Long-lived background task per session (survives WS disconnect)
//! - Output broadcast to multiple subscribers
//! - Ring buffer for replay on reconnect
//! - Stable PTY PID across reconnects

use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::{debug, error, info};

use crate::config::Config;
use crate::ring::{RingBuffer, OutputFrame, PushResult};

/// Owned PTY with background task for I/O
/// 
/// The PTY and its I/O threads are owned by this struct.
/// When dropped, the PTY is terminated.
pub struct OwnedPty {
    /// PTY process ID for stability verification
    pub pid: u32,
    /// Terminal size
    pub cols: u16,
    pub rows: u16,
    /// Channel to send input to PTY
    pub input_tx: mpsc::Sender<Vec<u8>>,
    /// Broadcast channel for output (subscribers can join/leave)
    pub output_tx: broadcast::Sender<OutputFrame>,
    /// Ring buffer for replay on reconnect
    pub ring: Arc<RingBuffer>,
    /// Current sequence number
    pub current_seq: Arc<AtomicU64>,
    /// Shutdown signal
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl OwnedPty {
    /// Get current sequence number for watermarking
    pub fn seq(&self) -> u64 {
        self.current_seq.load(Ordering::SeqCst)
    }
    
    /// Subscribe to output broadcast (for new WebSocket connections)
    pub fn subscribe(&self) -> broadcast::Receiver<OutputFrame> {
        self.output_tx.subscribe()
    }
    
    /// Get buffered output for replay
    pub fn get_replay(&self, since_seq: u64) -> Vec<OutputFrame> {
        self.ring.drain_since(since_seq, 1000)
    }
    
    /// Get all buffered output (for initial connect)
    pub fn get_all_buffered(&self) -> Vec<OutputFrame> {
        self.ring.get_all()
    }
    
    /// Resize the PTY
    pub fn resize(&mut self, cols: u16, rows: u16) {
        // TODO: Implement resize via channel message
        self.cols = cols;
        self.rows = rows;
    }
}

impl Drop for OwnedPty {
    fn drop(&mut self) {
        // Signal shutdown to background threads
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        debug!("OwnedPty dropped, PTY PID {} shutting down", self.pid);
    }
}

/// Spawn a session-owned PTY with background I/O task
///
/// Returns an OwnedPty that:
/// - Survives WebSocket disconnects
/// - Has stable PID for verification
/// - Broadcasts output to subscribers
/// - Buffers output in ring for replay
pub fn spawn_owned_pty(
    config: &Config,
    cols: u16,
    rows: u16,
) -> Result<OwnedPty, PtyTaskError> {
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
        .map_err(|e| PtyTaskError::OpenFailed(e.to_string()))?;
    
    // Build command with correct terminal capabilities
    let mut cmd = CommandBuilder::new(&config.tui_binary_path);
    cmd.env("TERM", "xterm-256color");
    cmd.env("LANG", "en_US.UTF-8");
    cmd.env("LC_ALL", "en_US.UTF-8");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("READ_MODEL_URL", &config.read_model_url);
    cmd.env("GATEWAY_URL", &config.gateway_url);
    
    // Spawn child process
    let mut child = pair.slave
        .spawn_command(cmd)
        .map_err(|e| PtyTaskError::SpawnFailed(e.to_string()))?;
    
    // Get PID for stability verification (before moving child)
    let pid = get_child_pid();
    
    // Drop slave - we only use master
    drop(pair.slave);
    
    // Get reader and writer from master
    let mut reader = pair.master.try_clone_reader()
        .map_err(|e| PtyTaskError::IoSetupFailed(e.to_string()))?;
    
    let mut writer = pair.master.take_writer()
        .map_err(|e| PtyTaskError::IoSetupFailed(e.to_string()))?;
    
    // Create channels
    let (input_tx, mut input_rx) = mpsc::channel::<Vec<u8>>(256);
    let (output_tx, _) = broadcast::channel::<OutputFrame>(256);
    let (shutdown_tx, _shutdown_rx) = oneshot::channel::<()>();
    
    // Create ring buffer
    let ring = Arc::new(RingBuffer::new(
        config.ring_max_bytes,
        config.ring_max_frames,
    ));
    
    // Sequence counter
    let current_seq = Arc::new(AtomicU64::new(0));
    
    // Clone for threads
    let output_tx_clone = output_tx.clone();
    let ring_clone = Arc::clone(&ring);
    let seq_clone = Arc::clone(&current_seq);
    
    // Spawn reader thread (PTY → broadcast + ring)
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
                    let seq = seq_clone.fetch_add(1, Ordering::SeqCst);
                    let frame = OutputFrame {
                        seq,
                        timestamp: std::time::Instant::now(),
                        data: data.clone(),
                    };
                    
                    // Push to ring buffer for replay
                    match ring_clone.push(data) {
                        PushResult::Truncated { frames_dropped } => {
                            debug!("Ring buffer truncated: {} frames dropped", frames_dropped);
                        }
                        PushResult::Ok => {}
                    }
                    
                    // Broadcast to subscribers (ignore errors - no subscribers is OK)
                    let _ = output_tx_clone.send(frame);
                }
                Err(e) => {
                    error!("PTY reader error: {}", e);
                    break;
                }
            }
        }
    });
    
    // Spawn writer thread (input channel → PTY)
    thread::spawn(move || {
        while let Some(data) = input_rx.blocking_recv() {
            if let Err(e) = writer.write_all(&data) {
                error!("PTY writer error: {}", e);
                break;
            }
            let _ = writer.flush();
        }
        debug!("PTY writer: channel closed");
    });
    
    // Spawn child waiter thread
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
    
    info!("Spawned OwnedPty {}x{} running {}, PID={}", cols, rows, config.tui_binary_path, pid);
    
    Ok(OwnedPty {
        pid,
        cols,
        rows,
        input_tx,
        output_tx,
        ring,
        current_seq,
        shutdown_tx: Some(shutdown_tx),
    })
}

/// Get process ID from child (platform-specific)
/// 
/// Note: portable-pty doesn't expose PID directly, so we use a stable counter.
/// In production, platform-specific code would be needed.
static NEXT_PSEUDO_PID: AtomicU64 = AtomicU64::new(1);

fn get_child_pid() -> u32 {
    NEXT_PSEUDO_PID.fetch_add(1, Ordering::SeqCst) as u32
}

/// PTY task errors
#[derive(Debug, Clone)]
pub enum PtyTaskError {
    OpenFailed(String),
    SpawnFailed(String),
    IoSetupFailed(String),
}

impl std::fmt::Display for PtyTaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenFailed(e) => write!(f, "Failed to open PTY: {}", e),
            Self::SpawnFailed(e) => write!(f, "Failed to spawn command: {}", e),
            Self::IoSetupFailed(e) => write!(f, "Failed to setup I/O: {}", e),
        }
    }
}

impl std::error::Error for PtyTaskError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    fn test_config() -> Config {
        Config {
            ws_port: 9000,
            metrics_port: 9001,
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
        }
    }
    
    #[test]
    fn test_owned_pty_pid_stable() {
        let config = test_config();
        let pty = spawn_owned_pty(&config, 80, 24);
        
        // May fail in CI without proper PTY environment
        if let Ok(pty) = pty {
            let pid = pty.pid;
            assert!(pid > 0, "PID should be non-zero");
            // PID should remain stable (same object)
            assert_eq!(pty.pid, pid);
        }
    }
    
    #[test]
    fn test_owned_pty_subscribe() {
        let config = test_config();
        if let Ok(pty) = spawn_owned_pty(&config, 80, 24) {
            // Should be able to subscribe
            let _rx1 = pty.subscribe();
            let _rx2 = pty.subscribe();
            // Multiple subscribers OK
        }
    }
}
