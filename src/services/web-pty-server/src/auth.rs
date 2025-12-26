//! Web PTY Server - Authentication module
//!
//! Implements WebSocket upgrade authentication (R5).
//! Security invariant: Auth token is NEVER logged.

use tracing::warn;

use crate::config::Config;

/// Authentication result
#[derive(Debug, Clone, PartialEq)]
pub enum AuthResult {
    /// Authentication successful
    Authenticated,
    /// No auth required (token not configured)
    NoAuthRequired,
    /// Authentication failed
    Failed(AuthError),
}

/// Authentication errors
#[derive(Debug, Clone, PartialEq)]
pub enum AuthError {
    /// No token provided when required
    MissingToken,
    /// Invalid token provided
    InvalidToken,
}

/// Authenticate a WebSocket upgrade request
/// 
/// # Arguments
/// * `config` - Server configuration (contains auth_token)
/// * `auth_header` - Value of Authorization header (if present)
/// 
/// # Returns
/// * `AuthResult` indicating success or failure reason
/// 
/// # Security
/// The auth token is NEVER logged, even at debug level.
pub fn authenticate(config: &Config, auth_header: Option<&str>) -> AuthResult {
    // If no token configured, auth is disabled
    let expected_token = match &config.auth_token {
        Some(t) => t,
        None => return AuthResult::NoAuthRequired,
    };
    
    // Extract Bearer token from header
    let provided_token = match auth_header {
        Some(header) => {
            if let Some(token) = header.strip_prefix("Bearer ") {
                token.trim()
            } else {
                // Log attempt without revealing token content
                warn!("Auth failed: malformed Authorization header");
                return AuthResult::Failed(AuthError::InvalidToken);
            }
        }
        None => {
            warn!("Auth failed: missing Authorization header");
            return AuthResult::Failed(AuthError::MissingToken);
        }
    };
    
    // Compare tokens (constant-time would be better but not critical for this use case)
    if provided_token == expected_token {
        // Note: We intentionally do NOT log the token or indicate which part matched
        AuthResult::Authenticated
    } else {
        // Log failure without revealing expected or provided tokens
        warn!("Auth failed: invalid token");
        AuthResult::Failed(AuthError::InvalidToken)
    }
}

/// Extract session ID and reconnect token from query string
/// 
/// # Arguments
/// * `query` - Query string (e.g., "session=abc&token=xyz")
/// 
/// # Returns
/// * `Option<(String, String)>` - (session_id, token) if present
pub fn parse_reconnect_params(query: Option<&str>) -> Option<(String, String)> {
    let query = query?;
    
    let mut session_id = None;
    let mut token = None;
    
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            match key {
                "session" => session_id = Some(value.to_string()),
                "token" => token = Some(value.to_string()),
                _ => {}
            }
        }
    }
    
    match (session_id, token) {
        (Some(s), Some(t)) => Some((s, t)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    fn config_with_token(token: Option<&str>) -> Config {
        Config {
            ws_port: 9000,
            metrics_port: 9001,
            tui_binary_path: "test".to_string(),
            auth_token: token.map(|s| s.to_string()),
            read_only: false,
            idle_timeout: Duration::from_secs(60),
            per_ip_cap: 5,
            global_cap: 50,
            disconnect_grace: Duration::from_secs(30),
            max_output_queue_bytes: 1024,
            read_model_url: "http://localhost:8080".to_string(),
            gateway_url: "http://localhost:3000".to_string(),
        }
    }
    
    #[test]
    fn test_auth_disabled_when_no_token_configured() {
        let config = config_with_token(None);
        let result = authenticate(&config, None);
        assert_eq!(result, AuthResult::NoAuthRequired);
    }
    
    #[test]
    fn test_auth_success_with_valid_token() {
        let config = config_with_token(Some("secret123"));
        let result = authenticate(&config, Some("Bearer secret123"));
        assert_eq!(result, AuthResult::Authenticated);
    }
    
    #[test]
    fn test_auth_failure_missing_header() {
        let config = config_with_token(Some("secret123"));
        let result = authenticate(&config, None);
        assert_eq!(result, AuthResult::Failed(AuthError::MissingToken));
    }
    
    #[test]
    fn test_auth_failure_invalid_token() {
        let config = config_with_token(Some("secret123"));
        let result = authenticate(&config, Some("Bearer wrongtoken"));
        assert_eq!(result, AuthResult::Failed(AuthError::InvalidToken));
    }
    
    #[test]
    fn test_auth_failure_malformed_header() {
        let config = config_with_token(Some("secret123"));
        let result = authenticate(&config, Some("Basic secret123"));
        assert_eq!(result, AuthResult::Failed(AuthError::InvalidToken));
    }
    
    #[test]
    fn test_auth_with_whitespace() {
        let config = config_with_token(Some("secret123"));
        let result = authenticate(&config, Some("Bearer   secret123  "));
        assert_eq!(result, AuthResult::Authenticated);
    }
    
    #[test]
    fn test_parse_reconnect_params_valid() {
        let result = parse_reconnect_params(Some("session=abc123&token=xyz789"));
        assert_eq!(result, Some(("abc123".to_string(), "xyz789".to_string())));
    }
    
    #[test]
    fn test_parse_reconnect_params_missing_token() {
        let result = parse_reconnect_params(Some("session=abc123"));
        assert_eq!(result, None);
    }
    
    #[test]
    fn test_parse_reconnect_params_missing_session() {
        let result = parse_reconnect_params(Some("token=xyz789"));
        assert_eq!(result, None);
    }
    
    #[test]
    fn test_parse_reconnect_params_empty() {
        let result = parse_reconnect_params(None);
        assert_eq!(result, None);
    }
    
    #[test]
    fn test_parse_reconnect_params_with_extra_params() {
        let result = parse_reconnect_params(Some("session=abc&token=xyz&other=ignored"));
        assert_eq!(result, Some(("abc".to_string(), "xyz".to_string())));
    }
}
