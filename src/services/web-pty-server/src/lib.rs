//! Web PTY Server Library
//!
//! PTY broker for streaming odd-dashboard TUI to xterm.js via WebSocket.

pub mod config;
pub mod session;
pub mod protocol;
pub mod auth;
pub mod pty;

// Re-exports for convenience
pub use config::Config;
pub use session::{SessionManager, PtySession, SessionError, SessionMetrics, CleanupStats};
pub use protocol::{ClientMessage, ServerMessage, error_codes, input_class};
pub use auth::{authenticate, AuthResult, AuthError, parse_reconnect_params, parse_auth_param};
pub use pty::{spawn_pty, PtyHandle, PtySpawnError};
