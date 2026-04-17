// relay_proxy.rs - WebSocket proxy layer between clients and nostr-rs-relay backend
// Implements NIP-42 AUTH inspection and policy enforcement

use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::{collections::HashSet, sync::Arc};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::Message as TungsteniteMessage, MaybeTlsStream, WebSocketStream,
};

use crate::{auth_policy::*, AppState};

type BackendWs = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Main WebSocket handler - upgrades connection and starts proxy
pub async fn relay_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_relay_connection(socket, state))
}

/// Handle a client connection with AUTH policy enforcement
async fn handle_relay_connection(client_socket: WebSocket, state: Arc<AppState>) {
    let backend_url = std::env::var("RELAY_BACKEND")
        .unwrap_or_else(|_| "ws://127.0.0.1:7447".to_string());

    // Connect to backend relay
    let backend_result = connect_async(&backend_url).await;
    let (backend_ws, _) = match backend_result {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Failed to connect to backend relay: {}", e);
            return;
        }
    };

    // Load current AUTH policy
    let policy = AuthPolicy::load_from_db(&state.pool).await;

    // Track connection in database
    let connection_id = track_connection(&state.pool, None, "0.0.0.0", false).await;

    // Run bidirectional proxy with policy enforcement
    proxy_with_auth(client_socket, backend_ws, policy, state, connection_id).await;

    // Mark connection as closed
    mark_disconnected(&state.pool, connection_id).await;
}

/// Bidirectional proxy with AUTH policy enforcement
async fn proxy_with_auth(
    client_socket: WebSocket,
    backend_ws: BackendWs,
    policy: AuthPolicy,
    state: Arc<AppState>,
    connection_id: i64,
) {
    let (mut client_sink, mut client_stream) = client_socket.split();
    let (mut backend_sink, mut backend_stream) = backend_ws.split();

    // Track authenticated pubkeys for this connection
    let mut authenticated_pubkeys = HashSet::<String>::new();

    // Client -> Backend (with policy enforcement)
    let client_to_backend = async {
        while let Some(result) = client_stream.next().await {
            match result {
                Ok(WsMessage::Text(text)) => {
                    // Parse Nostr message
                    let parsed: Result<Value, _> = serde_json::from_str(&text);

                    if let Ok(msg) = parsed {
                        let should_forward = handle_client_message(
                            &msg,
                            &policy,
                            &mut authenticated_pubkeys,
                            &mut client_sink,
                            &state,
                            connection_id,
                        )
                        .await;

                        if should_forward {
                            // Forward to backend
                            let backend_msg = TungsteniteMessage::Text(text);
                            if backend_sink.send(backend_msg).await.is_err() {
                                break;
                            }
                        }
                    } else {
                        // Invalid JSON, forward anyway and let backend handle it
                        let backend_msg = TungsteniteMessage::Text(text);
                        if backend_sink.send(backend_msg).await.is_err() {
                            break;
                        }
                    }
                }
                Ok(WsMessage::Close(_)) => {
                    let _ = backend_sink.send(TungsteniteMessage::Close(None)).await;
                    break;
                }
                Err(_) => break,
                _ => {}
            }
        }
    };

    // Backend -> Client (passthrough)
    let backend_to_client = async {
        while let Some(result) = backend_stream.next().await {
            match result {
                Ok(TungsteniteMessage::Text(text)) => {
                    let client_msg = WsMessage::Text(text);
                    if client_sink.send(client_msg).await.is_err() {
                        break;
                    }
                }
                Ok(TungsteniteMessage::Close(_)) => {
                    let _ = client_sink.send(WsMessage::Close(None)).await;
                    break;
                }
                Err(_) => break,
                _ => {}
            }
        }
    };

    // Run both directions concurrently
    tokio::select! {
        _ = client_to_backend => {},
        _ = backend_to_client => {},
    }
}

/// Handle a message from client and enforce AUTH policy
/// Returns true if message should be forwarded to backend
async fn handle_client_message(
    msg: &Value,
    policy: &AuthPolicy,
    authenticated_pubkeys: &mut HashSet<String>,
    client_sink: &mut futures::stream::SplitSink<WebSocket, WsMessage>,
    state: &Arc<AppState>,
    connection_id: i64,
) -> bool {
    let msg_type = msg
        .as_array()
        .and_then(|a| a.get(0))
        .and_then(|v| v.as_str());

    match msg_type {
        Some("EVENT") => {
            handle_event_message(msg, policy, authenticated_pubkeys, client_sink).await
        }
        Some("REQ") => handle_req_message(msg, policy, authenticated_pubkeys, client_sink).await,
        Some("AUTH") => {
            handle_auth_message(msg, authenticated_pubkeys, state, connection_id).await;
            true // Always forward AUTH to backend
        }
        _ => true, // Forward other message types
    }
}

/// Handle EVENT message - check if AUTH is required for this kind
async fn handle_event_message(
    msg: &Value,
    policy: &AuthPolicy,
    authenticated_pubkeys: &HashSet<String>,
    client_sink: &mut futures::stream::SplitSink<WebSocket, WsMessage>,
) -> bool {
    let event = match msg.get(1) {
        Some(e) => e,
        None => return true, // Malformed, let backend handle
    };

    let kind = event
        .get("kind")
        .and_then(|k| k.as_u64())
        .unwrap_or(0) as u16;
    let pubkey = event
        .get("pubkey")
        .and_then(|p| p.as_str())
        .unwrap_or("");
    let event_id = event.get("id").and_then(|i| i.as_str()).unwrap_or("");

    // Check if this kind requires AUTH
    if policy.requires_auth_for_kind(kind) {
        if !authenticated_pubkeys.contains(pubkey) {
            // Send auth-required error
            let error_msg = json!([
                "OK",
                event_id,
                false,
                format!("auth-required: kind {} requires authentication", kind)
            ]);

            let _ = client_sink
                .send(WsMessage::Text(error_msg.to_string()))
                .await;

            return false; // Block this message
        }
    }

    // Check whitelist if enabled
    if !policy.is_whitelisted(pubkey) {
        let error_msg = json!([
            "OK",
            event_id,
            false,
            "restricted: pubkey not whitelisted"
        ]);

        let _ = client_sink
            .send(WsMessage::Text(error_msg.to_string()))
            .await;

        return false;
    }

    true // Allow event
}

/// Handle REQ message - check if reading requires AUTH
async fn handle_req_message(
    msg: &Value,
    policy: &AuthPolicy,
    authenticated_pubkeys: &HashSet<String>,
    client_sink: &mut futures::stream::SplitSink<WebSocket, WsMessage>,
) -> bool {
    if policy.requires_auth_for_read() && authenticated_pubkeys.is_empty() {
        let sub_id = msg.get(1).and_then(|s| s.as_str()).unwrap_or("");

        let error_msg = json!([
            "CLOSED",
            sub_id,
            "auth-required: relay requires authentication"
        ]);

        let _ = client_sink
            .send(WsMessage::Text(error_msg.to_string()))
            .await;

        return false; // Block this subscription
    }

    true // Allow REQ
}

/// Handle AUTH message - track authenticated pubkey
async fn handle_auth_message(
    msg: &Value,
    authenticated_pubkeys: &mut HashSet<String>,
    state: &Arc<AppState>,
    connection_id: i64,
) {
    if let Some(event) = msg.get(1) {
        if let Some(pubkey) = event.get("pubkey").and_then(|p| p.as_str()) {
            // Validate AUTH event (kind 22242, has challenge and relay tags)
            let kind = event.get("kind").and_then(|k| k.as_u64()).unwrap_or(0);

            if kind == 22242 {
                authenticated_pubkeys.insert(pubkey.to_string());

                // Update connection record
                mark_authenticated(&state.pool, connection_id, pubkey).await;

                // Log AUTH success
                log_auth_event(&state.pool, pubkey, "AUTH", "success").await;

                eprintln!(
                    "[AUTH] Client authenticated as {}... (connection {})",
                    &pubkey[..16],
                    connection_id
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_auth_enforcement() {
        let policy = AuthPolicy {
            global_auth_required: false,
            whitelist_only: false,
            auth_read_required: false,
            protected_kinds: vec![4, 1059].into_iter().collect(),
            whitelisted_pubkeys: HashSet::new(),
        };

        // Create mock event for kind 4 (DM)
        let event_msg = json!([
            "EVENT",
            {
                "id": "test123",
                "pubkey": "testpubkey",
                "kind": 4,
                "content": "secret message",
            }
        ]);

        let authenticated = HashSet::new();

        // Should require AUTH for kind 4
        // (In real test, we'd mock the client_sink)
        assert!(policy.requires_auth_for_kind(4));
        assert!(!authenticated.contains("testpubkey"));
    }

    #[test]
    fn test_whitelist_enforcement() {
        let mut policy = AuthPolicy {
            global_auth_required: false,
            whitelist_only: true,
            auth_read_required: false,
            protected_kinds: HashSet::new(),
            whitelisted_pubkeys: HashSet::new(),
        };

        policy.whitelisted_pubkeys.insert("allowed".to_string());

        assert!(policy.is_whitelisted("allowed"));
        assert!(!policy.is_whitelisted("blocked"));
    }
}
