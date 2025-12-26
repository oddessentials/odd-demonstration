//! Web PTY Server - WebSocket protocol types
//!
//! Defines the message protocol between browser and PTY server.

use serde::{Deserialize, Serialize};

/// Messages sent from client to server
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ClientMessage {
    /// Keyboard/terminal input
    Input { data: String },
    /// Terminal resize event
    Resize { cols: u16, rows: u16 },
    /// Ping for keepalive (optional, pong is automatic)
    Ping,
    /// Reconnect to existing session (Phase 7)
    Reconnect {
        /// Session ID to reconnect to
        session: String,
        /// Reconnect token (single-use)
        token: String,
        /// Last sequence number received (for watermark-based replay)
        #[serde(skip_serializing_if = "Option::is_none")]
        last_seq: Option<u64>,
    },
}

/// Messages sent from server to client
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ServerMessage {
    /// Session established with new token
    Session {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "reconnectToken")]
        reconnect_token: String,
    },
    /// Reconnected to existing session with rotated token
    Reconnected {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "reconnectToken")]
        reconnect_token: String,
        /// Last known terminal size (for resize on reconnect)
        #[serde(skip_serializing_if = "Option::is_none")]
        restore_size: Option<TerminalSize>,
    },
    /// Terminal output data (with sequence number for watermarking)
    Output { 
        data: String,
        /// Monotonic sequence number for replay watermarking (Phase 7)
        #[serde(skip_serializing_if = "Option::is_none")]
        seq: Option<u64>,
    },
    /// Error message
    Error { message: String, code: String },
    /// Pong response to client ping
    Pong,
    /// Read-only mode notice (R4)
    Notice { message: String },
    
    // === Phase 7: Replay Protocol ===
    
    /// Begin replay sequence (sent before replaying buffered output)
    #[serde(rename = "replay_begin")]
    ReplayBegin {
        /// First sequence number in replay
        from_seq: u64,
    },
    /// End replay sequence (live output resumes after this)
    #[serde(rename = "replay_end")]
    ReplayEnd {
        /// Last sequence number in replay
        last_seq: u64,
    },
    /// Buffer truncated notice (during replay)
    #[serde(rename = "buffer_truncated")]
    BufferTruncated {
        /// Number of frames that were dropped
        frames_dropped: u64,
    },
}

/// Terminal size for reconnect restore
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}

/// Error codes for protocol errors
pub mod error_codes {
    pub const AUTH_REQUIRED: &str = "AUTH_REQUIRED";
    pub const AUTH_FAILED: &str = "AUTH_FAILED";
    pub const SESSION_NOT_FOUND: &str = "SESSION_NOT_FOUND";
    pub const INVALID_TOKEN: &str = "INVALID_TOKEN";
    pub const GLOBAL_CAP: &str = "GLOBAL_CAP";
    pub const PER_IP_CAP: &str = "PER_IP_CAP";
    pub const PTY_SPAWN_FAILED: &str = "PTY_SPAWN_FAILED";
    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
}

impl ServerMessage {
    /// Create an error response
    pub fn error(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
            code: code.into(),
        }
    }
    
    /// Create a read-only notice
    pub fn read_only_notice(action: &str) -> Self {
        Self::Notice {
            message: format!("⚠ Read-only mode: {} disabled", action),
        }
    }
}

/// Input classes for read-only filtering (R4)
pub mod input_class {
    /// New task creation
    pub const NEW_TASK: &str = "new_task";
    /// Launch cluster
    pub const LAUNCH: &str = "launch";
    /// Prerequisite installation
    pub const INSTALL: &str = "install";
    /// Text input in modals
    pub const MODAL_INPUT: &str = "modal_input";
}

/// Check if an input character is a mutating action in read-only mode
pub fn classify_input(input: &str) -> Option<&'static str> {
    match input {
        "n" | "N" => Some(input_class::NEW_TASK),
        "l" | "L" => Some(input_class::LAUNCH),
        "\r" | "\n" => Some(input_class::MODAL_INPUT), // Enter in modals
        _ => None,
    }
}

/// Check if input should be blocked in read-only mode
pub fn is_blocked_in_read_only(input: &str) -> bool {
    classify_input(input).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_client_message_deserialize_input() {
        let json = r#"{"type":"input","data":"Hello"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        
        match msg {
            ClientMessage::Input { data } => assert_eq!(data, "Hello"),
            _ => panic!("Expected Input message"),
        }
    }
    
    #[test]
    fn test_client_message_deserialize_resize() {
        let json = r#"{"type":"resize","cols":120,"rows":40}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        
        match msg {
            ClientMessage::Resize { cols, rows } => {
                assert_eq!(cols, 120);
                assert_eq!(rows, 40);
            }
            _ => panic!("Expected Resize message"),
        }
    }
    
    #[test]
    fn test_server_message_serialize_session() {
        let msg = ServerMessage::Session {
            session_id: "abc123".to_string(),
            reconnect_token: "token456".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        
        assert!(json.contains(r#""type":"session""#));
        assert!(json.contains(r#""sessionId":"abc123""#));
        assert!(json.contains(r#""reconnectToken":"token456""#));
    }
    
    #[test]
    fn test_server_message_serialize_output() {
        let msg = ServerMessage::Output {
            data: "\x1b[32mHello\x1b[0m".to_string(),
            seq: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        
        assert!(json.contains(r#""type":"output""#));
        assert!(json.contains(r#""data":""#));
        // seq should not be serialized when None
        assert!(!json.contains("seq"));
    }
    
    #[test]
    fn test_server_message_output_with_seq() {
        let msg = ServerMessage::Output {
            data: "test".to_string(),
            seq: Some(42),
        };
        let json = serde_json::to_string(&msg).unwrap();
        
        assert!(json.contains(r#""seq":42"#));
    }
    
    #[test]
    fn test_replay_begin_end_messages() {
        let begin = ServerMessage::ReplayBegin { from_seq: 10 };
        let end = ServerMessage::ReplayEnd { last_seq: 20 };
        
        let begin_json = serde_json::to_string(&begin).unwrap();
        let end_json = serde_json::to_string(&end).unwrap();
        
        assert!(begin_json.contains(r#""type":"replay_begin""#));
        assert!(begin_json.contains(r#""from_seq":10"#));
        assert!(end_json.contains(r#""type":"replay_end""#));
        assert!(end_json.contains(r#""last_seq":20"#));
    }
    
    #[test]
    fn test_server_message_error_helper() {
        let msg = ServerMessage::error("Test error", error_codes::AUTH_FAILED);
        
        match msg {
            ServerMessage::Error { message, code } => {
                assert_eq!(message, "Test error");
                assert_eq!(code, error_codes::AUTH_FAILED);
            }
            _ => panic!("Expected Error message"),
        }
    }
    
    #[test]
    fn test_read_only_notice() {
        let msg = ServerMessage::read_only_notice("Task creation");
        
        match msg {
            ServerMessage::Notice { message } => {
                assert!(message.contains("Read-only mode"));
                assert!(message.contains("Task creation"));
            }
            _ => panic!("Expected Notice message"),
        }
    }
    
    #[test]
    fn test_classify_input_mutating() {
        assert_eq!(classify_input("n"), Some(input_class::NEW_TASK));
        assert_eq!(classify_input("N"), Some(input_class::NEW_TASK));
        assert_eq!(classify_input("l"), Some(input_class::LAUNCH));
        assert_eq!(classify_input("L"), Some(input_class::LAUNCH));
        assert_eq!(classify_input("\r"), Some(input_class::MODAL_INPUT));
    }
    
    #[test]
    fn test_classify_input_allowed() {
        assert_eq!(classify_input("q"), None);
        assert_eq!(classify_input("r"), None);
        assert_eq!(classify_input("u"), None);
        assert_eq!(classify_input("\x1b"), None); // Escape
    }
    
    #[test]
    fn test_is_blocked_in_read_only() {
        assert!(is_blocked_in_read_only("n"));
        assert!(is_blocked_in_read_only("L"));
        assert!(!is_blocked_in_read_only("q"));
        assert!(!is_blocked_in_read_only("r"));
    }
}
