//! Web PTY Server - Main entry point
//!
//! WebSocket server bridging browser xterm.js to odd-dashboard TUI via PTY.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::time;
use tokio_tungstenite::{accept_hdr_async, tungstenite::Message};
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

use web_pty_server::{
    Config, SessionManager, ClientMessage, ServerMessage, error_codes,
    authenticate, AuthResult, parse_reconnect_params, parse_auth_param,
    spawn_pty, protocol,
};

/// Shared application state
struct AppState {
    config: Config,
    session_manager: Mutex<SessionManager>,
}

#[tokio::main]
async fn main() {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
    
    // Load and log configuration
    let config = Config::from_env();
    config.log_startup();
    
    let state = Arc::new(AppState {
        config: config.clone(),
        session_manager: Mutex::new(SessionManager::new(config.clone())),
    });
    
    // Spawn cleanup task
    let cleanup_state = Arc::clone(&state);
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            let stats = {
                let mut manager = cleanup_state.session_manager.lock().await;
                manager.cleanup()
            };
            if stats.removed > 0 {
                info!("Cleanup: removed {} sessions, {} active", stats.removed, stats.active);
            }
        }
    });
    
    // Start WebSocket server
    let ws_addr = SocketAddr::from(([0, 0, 0, 0], config.ws_port));
    let ws_listener = TcpListener::bind(&ws_addr).await
        .expect("Failed to bind WebSocket listener");
    info!("WebSocket server listening on {}", ws_addr);
    
    // Start metrics server
    let metrics_state = Arc::clone(&state);
    let metrics_addr = SocketAddr::from(([0, 0, 0, 0], config.metrics_port));
    tokio::spawn(async move {
        run_metrics_server(metrics_addr, metrics_state).await;
    });
    
    // Accept WebSocket connections
    loop {
        match ws_listener.accept().await {
            Ok((stream, addr)) => {
                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, addr, state).await {
                        error!("Connection error from {}: {}", addr, e);
                    }
                });
            }
            Err(e) => {
                error!("Accept error: {}", e);
            }
        }
    }
}

/// Handle a WebSocket connection
async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    state: Arc<AppState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client_ip = addr.ip();
    
    // Variables to capture from handshake
    let mut auth_header: Option<String> = None;
    let mut query_string: Option<String> = None;
    
    // Perform WebSocket handshake with header callback
    let callback = |req: &Request, response: Response| {
        // Extract Authorization header (R5: never log the value)
        auth_header = req.headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        
        // Extract query string for reconnect
        query_string = req.uri().query().map(|s| s.to_string());
        
        Ok(response)
    };
    
    let ws_stream = accept_hdr_async(stream, callback).await?;
    
    // Extract auth token from query string (browser WebSocket compat)
    let auth_query = parse_auth_param(query_string.as_deref());
    
    // Authenticate (R5) - checks header first, then query string
    let auth_result = authenticate(&state.config, auth_header.as_deref(), auth_query.as_deref());
    if let AuthResult::Failed(err) = auth_result {
        // Send error and close
        let (mut write, _read) = ws_stream.split();
        let msg = match err {
            web_pty_server::AuthError::MissingToken => {
                ServerMessage::error("Authorization required", error_codes::AUTH_REQUIRED)
            }
            web_pty_server::AuthError::InvalidToken => {
                ServerMessage::error("Invalid token", error_codes::AUTH_FAILED)
            }
        };
        let json = serde_json::to_string(&msg)?;
        write.send(Message::Text(json)).await?;
        write.close().await?;
        return Ok(());
    }
    
    // Check for reconnect params (R2)
    let reconnect = parse_reconnect_params(query_string.as_deref());
    let is_reconnect = reconnect.is_some();
    
    let (session_id, reconnect_token) = if let Some((sid_str, token)) = reconnect {
        // Attempt reconnect
        let session_id = Uuid::parse_str(&sid_str)?;
        
        let result = {
            let mut manager = state.session_manager.lock().await;
            manager.reconnect_session(session_id, &token, client_ip)
        };
        
        match result {
            Ok(new_token) => {
                info!("Client {} reconnected to session {}", client_ip, session_id);
                (session_id, new_token)
            }
            Err(e) => {
                let (mut write, _) = ws_stream.split();
                let msg = ServerMessage::error(e.to_string(), error_codes::INVALID_TOKEN);
                let json = serde_json::to_string(&msg)?;
                write.send(Message::Text(json)).await?;
                write.close().await?;
                return Ok(());
            }
        }
    } else {
        // Create new session
        let result = {
            let mut manager = state.session_manager.lock().await;
            manager.create_session(client_ip)
        };
        
        match result {
            Ok((id, token)) => (id, token),
            Err(e) => {
                let (mut write, _) = ws_stream.split();
                let code = match e {
                    web_pty_server::SessionError::GlobalCapReached => error_codes::GLOBAL_CAP,
                    web_pty_server::SessionError::PerIpCapReached => error_codes::PER_IP_CAP,
                    _ => error_codes::INTERNAL_ERROR,
                };
                let msg = ServerMessage::error(e.to_string(), code);
                let json = serde_json::to_string(&msg)?;
                write.send(Message::Text(json)).await?;
                write.close().await?;
                return Ok(());
            }
        }
    };
    
    // Split WebSocket
    let (mut ws_write, mut ws_read) = ws_stream.split();
    
    // Send session info
    let session_msg = if is_reconnect {
        ServerMessage::Reconnected {
            session_id: session_id.to_string(),
            reconnect_token: reconnect_token.clone(),
        }
    } else {
        ServerMessage::Session {
            session_id: session_id.to_string(),
            reconnect_token: reconnect_token.clone(),
        }
    };
    let json = serde_json::to_string(&session_msg)?;
    ws_write.send(Message::Text(json)).await?;
    
    // Spawn PTY for this connection
    // NOTE: Currently spawns fresh PTY on each connection (including reconnects).
    // Session tokens provide continuity for authentication, but PTY state is not preserved.
    // TODO: Implement PTY output buffering to preserve state across reconnects.
    let pty_result = spawn_pty(&state.config, 80, 24);
    let pty_handle = match pty_result {
        Ok(h) => h,
        Err(e) => {
            let msg = ServerMessage::error(e.to_string(), error_codes::PTY_SPAWN_FAILED);
            let json = serde_json::to_string(&msg)?;
            ws_write.send(Message::Text(json)).await?;
            ws_write.close().await?;
            return Ok(());
        }
    };
    
    let input_tx = pty_handle.input_tx;
    let mut output_rx = pty_handle.output_rx;
    
    // Ping/pong keepalive (R6)
    let ping_interval = Duration::from_secs(30);
    let mut last_pong = std::time::Instant::now();
    let mut ping_ticker = time::interval(ping_interval);
    
    // Output coalescing timer (R9)
    let mut coalesce_buffer: Vec<u8> = Vec::new();
    let coalesce_interval = Duration::from_millis(16);
    let mut coalesce_ticker = time::interval(coalesce_interval);
    
    loop {
        tokio::select! {
            // Handle incoming WebSocket messages
            msg = ws_read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Update last seen
                        {
                            let mut manager = state.session_manager.lock().await;
                            if let Some(session) = manager.get_session(session_id) {
                                session.last_seen = std::time::Instant::now();
                            }
                        }
                        
                        // Parse client message
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(ClientMessage::Input { data }) => {
                                // Check read-only mode (R4)
                                if state.config.read_only {
                                    if let Some(input_class) = protocol::classify_input(&data) {
                                        // Rate-limit notice
                                        let should_notify = {
                                            let mut manager = state.session_manager.lock().await;
                                            if let Some(session) = manager.get_session(session_id) {
                                                session.should_show_notice(input_class)
                                            } else {
                                                false
                                            }
                                        };
                                        
                                        if should_notify {
                                            let action = match input_class {
                                                protocol::input_class::NEW_TASK => "Task creation",
                                                protocol::input_class::LAUNCH => "Cluster launch",
                                                protocol::input_class::INSTALL => "Installation",
                                                _ => "This action",
                                            };
                                            warn!("Blocked mutating input '{}' in read-only mode", input_class);
                                            let msg = ServerMessage::read_only_notice(action);
                                            let json = serde_json::to_string(&msg)?;
                                            ws_write.send(Message::Text(json)).await?;
                                        }
                                        continue; // Don't send to PTY
                                    }
                                }
                                
                                // Send to PTY
                                if input_tx.send(data.into_bytes()).await.is_err() {
                                    error!("PTY input channel closed");
                                    break;
                                }
                            }
                            Ok(ClientMessage::Resize { cols, rows }) => {
                                // R8: Resize handling is prioritized
                                debug!("Resize to {}x{}", cols, rows);
                                // Note: Resize requires PTY ioctl which we'd need to add
                                // For now, log it
                            }
                            Ok(ClientMessage::Ping) => {
                                let msg = ServerMessage::Pong;
                                let json = serde_json::to_string(&msg)?;
                                ws_write.send(Message::Text(json)).await?;
                            }
                            Err(e) => {
                                warn!("Invalid client message: {}", e);
                            }
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        last_pong = std::time::Instant::now();
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("Client {} closed connection", client_ip);
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        debug!("WebSocket stream ended");
                        break;
                    }
                    _ => {}
                }
            }
            
            // Handle PTY output
            output = output_rx.recv() => {
                match output {
                    Some(data) => {
                        // Coalesce output (R9)
                        coalesce_buffer.extend(data);
                    }
                    None => {
                        info!("PTY closed");
                        break;
                    }
                }
            }
            
            // Flush coalesced output (R9)
            _ = coalesce_ticker.tick() => {
                if !coalesce_buffer.is_empty() {
                    let data = std::mem::take(&mut coalesce_buffer);
                    // Convert to string (assuming UTF-8, but handle invalid gracefully)
                    let text = String::from_utf8_lossy(&data).to_string();
                    let msg = ServerMessage::Output { data: text };
                    let json = serde_json::to_string(&msg)?;
                    if let Err(e) = ws_write.send(Message::Text(json)).await {
                        error!("Failed to send output: {}", e);
                        break;
                    }
                }
            }
            
            // Send ping for keepalive (R6)
            _ = ping_ticker.tick() => {
                if last_pong.elapsed() > Duration::from_secs(60) {
                    warn!("No pong received, closing connection");
                    break;
                }
                if let Err(e) = ws_write.send(Message::Ping(vec![])).await {
                    error!("Failed to send ping: {}", e);
                    break;
                }
            }
        }
    }
    
    // Mark session as disconnected
    {
        let mut manager = state.session_manager.lock().await;
        manager.disconnect_session(session_id);
    }
    
    info!("Session {} disconnected", session_id);
    Ok(())
}

/// Simple metrics/health server
async fn run_metrics_server(addr: SocketAddr, state: Arc<AppState>) {
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::{Request, Response, StatusCode};
    use hyper::body::Incoming;
    use http_body_util::Full;
    use hyper::body::Bytes;
    use hyper_util::rt::TokioIo;
    
    let listener = TcpListener::bind(&addr).await
        .expect("Failed to bind metrics listener");
    info!("Metrics server listening on {}", addr);
    
    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let io = TokioIo::new(stream);
        let state = Arc::clone(&state);
        
        tokio::spawn(async move {
            let service = service_fn(move |req: Request<Incoming>| {
                let state = Arc::clone(&state);
                async move {
                    let path = req.uri().path();
                    match path {
                        "/healthz" => {
                            Ok::<_, std::convert::Infallible>(
                                Response::new(Full::new(Bytes::from("ok")))
                            )
                        }
                        "/readyz" => {
                            Ok(Response::new(Full::new(Bytes::from("ok"))))
                        }
                        "/metrics" => {
                            let metrics = {
                                let manager = state.session_manager.lock().await;
                                manager.get_metrics()
                            };
                            let body = format!(
                                "# HELP pty_sessions_active Active PTY sessions\n\
                                 # TYPE pty_sessions_active gauge\n\
                                 pty_sessions_active {}\n\
                                 # HELP pty_output_queue_bytes_total Total bytes in output queues\n\
                                 # TYPE pty_output_queue_bytes_total gauge\n\
                                 pty_output_queue_bytes_total {}\n\
                                 # HELP pty_output_drops_total Total output drops (backpressure)\n\
                                 # TYPE pty_output_drops_total counter\n\
                                 pty_output_drops_total {}\n",
                                metrics.active_sessions,
                                metrics.total_output_queue_bytes,
                                metrics.total_output_drops
                            );
                            Ok(Response::new(Full::new(Bytes::from(body))))
                        }
                        _ => {
                            let mut resp = Response::new(Full::new(Bytes::from("not found")));
                            *resp.status_mut() = StatusCode::NOT_FOUND;
                            Ok(resp)
                        }
                    }
                }
            });
            
            if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                error!("Metrics server error: {}", e);
            }
        });
    }
}
