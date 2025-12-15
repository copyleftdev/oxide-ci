//! WebSocket handler for real-time event streaming.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::{extract::State, response::Response};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ClientMessage {
    Subscribe { channels: Vec<String> },
    Unsubscribe { channels: Vec<String> },
    Ping,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ServerMessage {
    Event {
        channel: String,
        payload: serde_json::Value,
    },
    Subscribed {
        channels: Vec<String>,
    },
    Unsubscribed {
        channels: Vec<String>,
    },
    Pong,
    Error {
        message: String,
    },
}

pub async fn ws_handler(ws: WebSocketUpgrade, State(_state): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    let mut subscribed_channels: Vec<String> = Vec::new();

    while let Some(msg) = socket.recv().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(_) => break,
        };

        match msg {
            Message::Text(text) => {
                let client_msg: Result<ClientMessage, _> = serde_json::from_str(&text);

                match client_msg {
                    Ok(ClientMessage::Subscribe { channels }) => {
                        subscribed_channels.extend(channels.clone());
                        let response = ServerMessage::Subscribed { channels };
                        let _ = socket
                            .send(Message::Text(
                                serde_json::to_string(&response).unwrap().into(),
                            ))
                            .await;
                    }
                    Ok(ClientMessage::Unsubscribe { channels }) => {
                        subscribed_channels.retain(|c| !channels.contains(c));
                        let response = ServerMessage::Unsubscribed { channels };
                        let _ = socket
                            .send(Message::Text(
                                serde_json::to_string(&response).unwrap().into(),
                            ))
                            .await;
                    }
                    Ok(ClientMessage::Ping) => {
                        let response = ServerMessage::Pong;
                        let _ = socket
                            .send(Message::Text(
                                serde_json::to_string(&response).unwrap().into(),
                            ))
                            .await;
                    }
                    Err(e) => {
                        let response = ServerMessage::Error {
                            message: format!("Invalid message: {}", e),
                        };
                        let _ = socket
                            .send(Message::Text(
                                serde_json::to_string(&response).unwrap().into(),
                            ))
                            .await;
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
