//! Web PTY Server - Session management module
//!
//! Implements the explicit PTY session model with single-use reconnect tokens (R2).

use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::time::{Duration, Instant};

use rand::Rng;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::Config;

// ============================================================
// Session State Machine (Phase 7: PTY State Preservation)
// ============================================================

/// Session state for lifecycle management
/// 
/// State transitions:
/// - Connected → Disconnected: WebSocket close
/// - Disconnected → Connected: Successful reconnect with valid token
/// - Disconnected → Idle: Grace period expires
/// - Idle → Reaping: Idle timeout expires
/// - Reaping → (removed): Session cleaned up
/// - Disconnected/Idle → ERROR: Reconnect fails with "session expired"
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Client is actively connected via WebSocket
    Connected,
    /// Client disconnected, within grace period for reconnect
    Disconnected { since: Instant },
    /// Grace period expired, waiting for idle timeout
    Idle { since: Instant },
    /// Session is being cleaned up (reject all operations)
    Reaping,
}

impl SessionState {
    /// Check if state allows reconnection
    pub fn can_reconnect(&self) -> bool {
        matches!(self, SessionState::Disconnected { .. })
    }
    
    /// Check if state allows input
    pub fn can_accept_input(&self) -> bool {
        matches!(self, SessionState::Connected)
    }
    
    /// Check if state is terminal (should be cleaned up)
    pub fn is_reaping(&self) -> bool {
        matches!(self, SessionState::Reaping)
    }
}

/// A PTY session with reconnect token
#[derive(Debug)]
pub struct PtySession {
    /// Unique session identifier
    pub session_id: Uuid,
    /// Single-use reconnect token (rotated on successful reconnect)
    pub reconnect_token: String,
    /// When token was issued (for TTL expiry)
    pub token_issued_at: Instant,
    /// When session was created
    pub created_at: Instant,
    /// Last activity timestamp
    pub last_seen: Instant,
    /// Client IP address
    pub client_ip: IpAddr,
    /// Session state machine (Phase 7)
    pub state: SessionState,
    /// Output queue for buffering PTY output
    pub output_queue: VecDeque<Vec<u8>>,
    /// Total bytes in output queue
    pub output_queue_bytes: usize,
    /// Drop counter for backpressure (R9: drops=0 is CI invariant)
    pub output_drops: u64,
    /// Rate-limit timestamps for read-only notices (R4)
    pub notice_times: HashMap<String, Instant>,
    /// Channel to send input to PTY
    pub input_tx: Option<mpsc::Sender<Vec<u8>>>,
    /// Channel to receive output from PTY
    pub output_rx: Option<mpsc::Receiver<Vec<u8>>>,
    /// Last known terminal columns (for resize on reconnect)
    pub last_cols: u16,
    /// Last known terminal rows (for resize on reconnect)
    pub last_rows: u16,
}

impl PtySession {
    /// Generate a cryptographically random reconnect token
    pub fn generate_token() -> String {
        let mut rng = rand::thread_rng();
        let bytes: [u8; 32] = rng.gen();
        base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
    }
    
    /// Rotate the reconnect token (R2: single-use tokens)
    pub fn rotate_token(&mut self) -> String {
        let new_token = Self::generate_token();
        self.reconnect_token = new_token.clone();
        new_token
    }
    
    /// Check if read-only notice should be shown (R4: rate-limited)
    pub fn should_show_notice(&mut self, input_class: &str) -> bool {
        let now = Instant::now();
        let last = self.notice_times.get(input_class);
        if last.map_or(true, |t| now.duration_since(*t) > Duration::from_secs(5)) {
            self.notice_times.insert(input_class.to_string(), now);
            true
        } else {
            false
        }
    }
    
    /// Queue output data with backpressure (R9)
    pub fn queue_output(&mut self, data: Vec<u8>, max_bytes: usize) {
        let data_len = data.len();
        
        // Apply backpressure: drop oldest if over limit
        while self.output_queue_bytes + data_len > max_bytes && !self.output_queue.is_empty() {
            if let Some(dropped) = self.output_queue.pop_front() {
                self.output_queue_bytes -= dropped.len();
                self.output_drops += 1;
            }
        }
        
        self.output_queue.push_back(data);
        self.output_queue_bytes += data_len;
    }
    
    /// Drain output queue for sending to client
    pub fn drain_output(&mut self) -> Vec<Vec<u8>> {
        let items: Vec<_> = self.output_queue.drain(..).collect();
        self.output_queue_bytes = 0;
        items
    }
}

/// Session manager handling all PTY sessions
pub struct SessionManager {
    sessions: HashMap<Uuid, PtySession>,
    ip_session_counts: HashMap<IpAddr, usize>,
    config: Config,
}

impl SessionManager {
    pub fn new(config: Config) -> Self {
        Self {
            sessions: HashMap::new(),
            ip_session_counts: HashMap::new(),
            config,
        }
    }
    
    /// Check if a new session can be created (R3: caps)
    pub fn can_create_session(&self, client_ip: IpAddr) -> Result<(), SessionError> {
        // Check global cap
        if self.sessions.len() >= self.config.global_cap {
            return Err(SessionError::GlobalCapReached);
        }
        
        // Check per-IP cap
        let ip_count = self.ip_session_counts.get(&client_ip).copied().unwrap_or(0);
        if ip_count >= self.config.per_ip_cap {
            return Err(SessionError::PerIpCapReached);
        }
        
        Ok(())
    }
    
    /// Create a new PTY session
    pub fn create_session(&mut self, client_ip: IpAddr) -> Result<(Uuid, String), SessionError> {
        self.can_create_session(client_ip)?;
        
        let session_id = Uuid::new_v4();
        let reconnect_token = PtySession::generate_token();
        
        let session = PtySession {
            session_id,
            reconnect_token: reconnect_token.clone(),
            token_issued_at: Instant::now(),
            created_at: Instant::now(),
            last_seen: Instant::now(),
            client_ip,
            state: SessionState::Connected,
            output_queue: VecDeque::new(),
            output_queue_bytes: 0,
            output_drops: 0,
            notice_times: HashMap::new(),
            input_tx: None,
            output_rx: None,
            last_cols: 80,  // Default size, updated on first resize
            last_rows: 24,
        };
        
        self.sessions.insert(session_id, session);
        *self.ip_session_counts.entry(client_ip).or_insert(0) += 1;
        
        info!("Created session {} for {}", session_id, client_ip);
        Ok((session_id, reconnect_token))
    }
    
    /// Attempt to reconnect to an existing session (R2: token validation + rotation)
    /// 
    /// # State Machine
    /// - Only allows reconnect from `Disconnected` state
    /// - Rejects if in `Idle` or `Reaping` (session expired)
    /// - Validates token and TTL before allowing reconnect
    pub fn reconnect_session(
        &mut self,
        session_id: Uuid,
        token: &str,
        client_ip: IpAddr,
    ) -> Result<String, SessionError> {
        let session = self.sessions.get_mut(&session_id)
            // Return same error for "not found" and "bad token" to avoid leaking existence
            .ok_or(SessionError::InvalidToken)?;
        
        // Check state machine - only Disconnected allows reconnect
        match session.state {
            SessionState::Connected => {
                warn!("Reconnect attempted for already-connected session {}", session_id);
                return Err(SessionError::InvalidToken);
            }
            SessionState::Disconnected { .. } => {
                // OK - can reconnect
            }
            SessionState::Idle { .. } | SessionState::Reaping => {
                warn!("Reconnect attempted for expired session {}", session_id);
                return Err(SessionError::SessionExpired);
            }
        }
        
        // Validate token (same error as "not found" to avoid leaking)
        if session.reconnect_token != token {
            warn!("Invalid reconnect token for session {}", session_id);
            return Err(SessionError::InvalidToken);
        }
        
        // Check token TTL
        if session.token_issued_at.elapsed() > self.config.token_ttl {
            warn!("Expired reconnect token for session {}", session_id);
            return Err(SessionError::InvalidToken);
        }
        
        // Validate client IP matches
        if session.client_ip != client_ip {
            warn!("IP mismatch on reconnect for session {}", session_id);
            return Err(SessionError::IpMismatch);
        }
        
        // Rotate token (R2: single-use) and reset TTL
        let new_token = session.rotate_token();
        session.token_issued_at = Instant::now();
        session.state = SessionState::Connected;
        session.last_seen = Instant::now();
        
        info!("Reconnected session {} (token rotated)", session_id);
        Ok(new_token)
    }
    
    /// Mark session as disconnected (transitions to Disconnected state)
    pub fn disconnect_session(&mut self, session_id: Uuid) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            session.state = SessionState::Disconnected { since: Instant::now() };
            debug!("Session {} → Disconnected", session_id);
        }
    }
    
    /// Get a session by ID
    pub fn get_session(&mut self, session_id: Uuid) -> Option<&mut PtySession> {
        self.sessions.get_mut(&session_id)
    }
    
    /// Cleanup idle and disconnected sessions (R3) - State machine version
    /// 
    /// State transitions:
    /// - Disconnected + grace expired → Idle
    /// - Idle + idle timeout → Reaping (then removed)
    pub fn cleanup(&mut self) -> CleanupStats {
        let now = Instant::now();
        let mut stats = CleanupStats::default();
        let mut to_transition: Vec<(Uuid, SessionState)> = Vec::new();
        let mut to_remove: Vec<Uuid> = Vec::new();
        
        // First pass: determine state transitions and removals
        for (id, s) in self.sessions.iter() {
            match s.state {
                SessionState::Connected => {
                    // Check for idle timeout (no activity)
                    if now.duration_since(s.last_seen) > self.config.idle_timeout {
                        to_transition.push((*id, SessionState::Reaping));
                    }
                }
                SessionState::Disconnected { since } => {
                    // Check for grace period expiry
                    if now.duration_since(since) > self.config.disconnect_grace {
                        to_transition.push((*id, SessionState::Idle { since: now }));
                    }
                }
                SessionState::Idle { since } => {
                    // Check for idle timeout
                    if now.duration_since(since) > self.config.idle_timeout {
                        to_transition.push((*id, SessionState::Reaping));
                    }
                }
                SessionState::Reaping => {
                    // Ready for removal
                    to_remove.push(*id);
                }
            }
        }
        
        // Apply state transitions
        for (id, new_state) in to_transition {
            if let Some(session) = self.sessions.get_mut(&id) {
                debug!("Session {} → {:?}", id, new_state);
                session.state = new_state;
            }
        }
        
        // Remove reaping sessions
        for session_id in to_remove {
            if let Some(session) = self.sessions.remove(&session_id) {
                // Decrement IP count
                if let Some(count) = self.ip_session_counts.get_mut(&session.client_ip) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        self.ip_session_counts.remove(&session.client_ip);
                    }
                }
                stats.removed += 1;
                debug!("Reaped session {}", session_id);
            }
        }
        
        stats.active = self.sessions.len();
        stats
    }
    
    /// Get metrics for monitoring
    pub fn get_metrics(&self) -> SessionMetrics {
        let mut metrics = SessionMetrics::default();
        metrics.active_sessions = self.sessions.len();
        
        for session in self.sessions.values() {
            metrics.total_output_queue_bytes += session.output_queue_bytes;
            metrics.total_output_drops += session.output_drops;
            
            // Phase 7: count by state
            match session.state {
                SessionState::Connected => metrics.connected_count += 1,
                SessionState::Disconnected { .. } => metrics.disconnected_count += 1,
                SessionState::Idle { .. } => metrics.idle_count += 1,
                SessionState::Reaping => metrics.reaping_count += 1,
            }
        }
        
        metrics
    }
}

/// Session-related errors
#[derive(Debug, Clone, PartialEq)]
pub enum SessionError {
    GlobalCapReached,
    PerIpCapReached,
    SessionNotFound,
    InvalidToken,
    IpMismatch,
    SessionExpired,  // Phase 7: session past grace/idle timeout
    PtySpawnFailed(String),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GlobalCapReached => write!(f, "Maximum session limit reached"),
            Self::PerIpCapReached => write!(f, "Maximum sessions per IP reached"),
            Self::SessionNotFound => write!(f, "Session not found"),
            Self::InvalidToken => write!(f, "Invalid reconnect token"),
            Self::IpMismatch => write!(f, "Client IP does not match session"),
            Self::SessionExpired => write!(f, "Session expired"),
            Self::PtySpawnFailed(e) => write!(f, "Failed to spawn PTY: {}", e),
        }
    }
}

impl std::error::Error for SessionError {}

/// Cleanup operation stats
#[derive(Debug, Default)]
pub struct CleanupStats {
    pub removed: usize,
    pub active: usize,
}

/// Session metrics for monitoring
#[derive(Debug, Default)]
pub struct SessionMetrics {
    pub active_sessions: usize,
    pub total_output_queue_bytes: usize,
    pub total_output_drops: u64,
    // Phase 7: state breakdown
    pub connected_count: usize,
    pub disconnected_count: usize,
    pub idle_count: usize,
    pub reaping_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use crate::config::TestMode;
    
    fn test_config() -> Config {
        Config {
            ws_port: 9000,
            metrics_port: 9001,
            tui_binary_path: "echo".to_string(),
            auth_token: None,
            read_only: false,
            idle_timeout: Duration::from_secs(60),
            per_ip_cap: 3,
            global_cap: 10,
            disconnect_grace: Duration::from_secs(5),
            max_output_queue_bytes: 1024,
            read_model_url: "http://localhost:8080".to_string(),
            gateway_url: "http://localhost:3000".to_string(),
            token_ttl: Duration::from_secs(300),
            ring_max_bytes: 1_048_576,
            ring_max_frames: 1000,
            test_mode: TestMode::None,
        }
    }
    
    fn test_ip() -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
    }
    
    #[test]
    fn test_create_session() {
        let mut manager = SessionManager::new(test_config());
        let result = manager.create_session(test_ip());
        
        assert!(result.is_ok());
        let (session_id, token) = result.unwrap();
        assert!(!token.is_empty());
        assert_eq!(manager.sessions.len(), 1);
        
        let session = manager.get_session(session_id).unwrap();
        assert_eq!(session.state, SessionState::Connected);
        assert_eq!(session.client_ip, test_ip());
    }
    
    #[test]
    fn test_per_ip_cap() {
        let mut manager = SessionManager::new(test_config());
        let ip = test_ip();
        
        // Create up to cap
        for _ in 0..3 {
            assert!(manager.create_session(ip).is_ok());
        }
        
        // Next should fail
        let result = manager.create_session(ip);
        assert_eq!(result.unwrap_err(), SessionError::PerIpCapReached);
    }
    
    #[test]
    fn test_global_cap() {
        let mut config = test_config();
        config.global_cap = 2;
        let mut manager = SessionManager::new(config);
        
        // Different IPs to avoid per-IP cap
        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));
        let ip3 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3));
        
        assert!(manager.create_session(ip1).is_ok());
        assert!(manager.create_session(ip2).is_ok());
        
        let result = manager.create_session(ip3);
        assert_eq!(result.unwrap_err(), SessionError::GlobalCapReached);
    }
    
    #[test]
    fn test_reconnect_with_valid_token() {
        let mut manager = SessionManager::new(test_config());
        let ip = test_ip();
        
        let (session_id, token) = manager.create_session(ip).unwrap();
        manager.disconnect_session(session_id);
        
        let result = manager.reconnect_session(session_id, &token, ip);
        assert!(result.is_ok());
        
        // Token should be rotated
        let new_token = result.unwrap();
        assert_ne!(new_token, token);
    }
    
    #[test]
    fn test_reconnect_with_invalid_token() {
        let mut manager = SessionManager::new(test_config());
        let ip = test_ip();
        
        let (session_id, _token) = manager.create_session(ip).unwrap();
        
        let result = manager.reconnect_session(session_id, "wrong-token", ip);
        assert_eq!(result.unwrap_err(), SessionError::InvalidToken);
    }
    
    #[test]
    fn test_reconnect_single_use_token() {
        let mut manager = SessionManager::new(test_config());
        let ip = test_ip();
        
        let (session_id, token) = manager.create_session(ip).unwrap();
        manager.disconnect_session(session_id);
        
        // First reconnect succeeds
        let new_token = manager.reconnect_session(session_id, &token, ip).unwrap();
        
        // Second reconnect with old token fails (R2: single-use)
        manager.disconnect_session(session_id);
        let result = manager.reconnect_session(session_id, &token, ip);
        assert_eq!(result.unwrap_err(), SessionError::InvalidToken);
        
        // Reconnect with new token succeeds
        let result = manager.reconnect_session(session_id, &new_token, ip);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_output_queue_backpressure() {
        let mut manager = SessionManager::new(test_config());
        let ip = test_ip();
        let (session_id, _) = manager.create_session(ip).unwrap();
        
        let session = manager.get_session(session_id).unwrap();
        
        // Queue data up to limit
        session.queue_output(vec![0u8; 500], 1024);
        assert_eq!(session.output_queue_bytes, 500);
        assert_eq!(session.output_drops, 0);
        
        session.queue_output(vec![0u8; 500], 1024);
        assert_eq!(session.output_queue_bytes, 1000);
        assert_eq!(session.output_drops, 0);
        
        // This should trigger drop
        session.queue_output(vec![0u8; 500], 1024);
        assert!(session.output_drops > 0);
    }
    
    #[test]
    fn test_read_only_notice_rate_limiting() {
        let mut manager = SessionManager::new(test_config());
        let ip = test_ip();
        let (session_id, _) = manager.create_session(ip).unwrap();
        
        let session = manager.get_session(session_id).unwrap();
        
        // First notice should show
        assert!(session.should_show_notice("N"));
        
        // Immediate repeat should not show
        assert!(!session.should_show_notice("N"));
        
        // Different class should show
        assert!(session.should_show_notice("L"));
    }
    
    #[test]
    fn test_cleanup_idle_sessions() {
        let mut config = test_config();
        config.idle_timeout = Duration::from_millis(1);
        let mut manager = SessionManager::new(config);
        
        let (session_id, _) = manager.create_session(test_ip()).unwrap();
        
        // Wait for idle timeout
        std::thread::sleep(Duration::from_millis(10));
        
        // Phase 7 state machine: Connected (idle) → Reaping (first cleanup)
        let stats1 = manager.cleanup();
        assert_eq!(stats1.removed, 0); // Not removed yet, just transitioned to Reaping
        assert!(manager.get_session(session_id).is_some()); // Still exists
        
        // Reaping → removed (second cleanup)
        let stats2 = manager.cleanup();
        assert_eq!(stats2.removed, 1);
        assert!(manager.get_session(session_id).is_none());
    }
    
    #[test]
    fn test_metrics() {
        let mut manager = SessionManager::new(test_config());
        let ip = test_ip();
        
        manager.create_session(ip).unwrap();
        manager.create_session(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))).unwrap();
        
        let metrics = manager.get_metrics();
        assert_eq!(metrics.active_sessions, 2);
        assert_eq!(metrics.total_output_drops, 0);
    }
}
