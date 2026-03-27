use crate::protocol::{ClientMessage, ServerMessage, DEFAULT_PORT};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

/// A handle to the hot reload client running in a background thread.
///
/// The client connects to the development server over WebSocket and forwards
/// incoming [`ServerMessage`]s through a non-blocking channel. The UI thread
/// polls for messages each frame via [`try_recv`](Self::try_recv) or
/// [`drain`](Self::drain) without blocking the event loop.
pub struct HotReloadClient {
    messages: mpsc::Receiver<ServerMessage>,
    connected: Arc<AtomicBool>,
    _shutdown: Option<tokio::sync::oneshot::Sender<()>>,
}

impl HotReloadClient {
    /// Connect to a hot reload server at the given WebSocket address
    /// (e.g. `"ws://127.0.0.1:3742"`).
    ///
    /// Returns immediately. The connection is established in a background thread.
    pub fn connect(addr: &str, app_name: &str) -> Result<Self, ClientError> {
        let (msg_tx, msg_rx) = mpsc::channel();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let connected = Arc::new(AtomicBool::new(false));
        let connected_clone = connected.clone();
        let addr = addr.to_owned();
        let app_name = app_name.to_owned();

        std::thread::Builder::new()
            .name("vitreous-hot-reload".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to create tokio runtime for hot reload client");

                rt.block_on(async {
                    match client_loop(&addr, &app_name, msg_tx, shutdown_rx, &connected_clone).await
                    {
                        Ok(()) => {}
                        Err(ClientError::ConnectionFailed(e)) => {
                            eprintln!("[vitreous hot-reload] Could not connect to dev server: {e}");
                        }
                        Err(e) => {
                            eprintln!("[vitreous hot-reload] Client error: {e}");
                        }
                    }
                    connected_clone.store(false, Ordering::Release);
                });
            })
            .map_err(|e| ClientError::SpawnFailed(e.to_string()))?;

        Ok(Self {
            messages: msg_rx,
            connected,
            _shutdown: Some(shutdown_tx),
        })
    }

    /// Connect to the default local server at `ws://127.0.0.1:{DEFAULT_PORT}`.
    pub fn connect_default(app_name: &str) -> Result<Self, ClientError> {
        Self::connect(&format!("ws://127.0.0.1:{DEFAULT_PORT}"), app_name)
    }

    /// Try to receive the next message without blocking.
    pub fn try_recv(&self) -> Option<ServerMessage> {
        self.messages.try_recv().ok()
    }

    /// Drain all pending messages into a `Vec`.
    pub fn drain(&self) -> Vec<ServerMessage> {
        let mut msgs = Vec::new();
        while let Ok(msg) = self.messages.try_recv() {
            msgs.push(msg);
        }
        msgs
    }

    /// Whether the WebSocket connection is currently alive.
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }
}

impl Drop for HotReloadClient {
    fn drop(&mut self) {
        if let Some(tx) = self._shutdown.take() {
            let _ = tx.send(());
        }
    }
}

// ── Background event loop ────────────────────────────────────────────────

async fn client_loop(
    addr: &str,
    app_name: &str,
    tx: mpsc::Sender<ServerMessage>,
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
    connected: &AtomicBool,
) -> Result<(), ClientError> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    let (ws, _) = tokio_tungstenite::connect_async(addr)
        .await
        .map_err(|e| ClientError::ConnectionFailed(e.to_string()))?;

    connected.store(true, Ordering::Release);
    let (mut ws_tx, mut ws_rx) = ws.split();

    // Send hello.
    let hello = ClientMessage::Hello {
        app_name: app_name.to_owned(),
    };
    if let Ok(json) = hello.to_json() {
        let _ = ws_tx.send(Message::Text(json.into())).await;
    }

    // Receive messages until shutdown or disconnect.
    loop {
        tokio::select! {
            msg = ws_rx.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let Ok(server_msg) = ServerMessage::from_json(&text) else { continue };
                        if tx.send(server_msg).is_err() {
                            break; // Receiver dropped.
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
            _ = &mut shutdown => {
                let _ = ws_tx.send(Message::Close(None)).await;
                break;
            }
        }
    }

    Ok(())
}

// ── Errors ───────────────────────────────────────────────────────────────

/// Errors that can occur in the hot reload client.
#[derive(Debug)]
pub enum ClientError {
    /// Failed to spawn the background thread.
    SpawnFailed(String),
    /// Failed to establish a WebSocket connection.
    ConnectionFailed(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SpawnFailed(e) => write!(f, "failed to spawn hot reload thread: {e}"),
            Self::ConnectionFailed(e) => write!(f, "failed to connect to hot reload server: {e}"),
        }
    }
}

impl std::error::Error for ClientError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_default_not_connected() {
        // Connecting to a non-existent server should not panic; the background
        // thread logs the error and sets connected = false.
        let client = HotReloadClient::connect("ws://127.0.0.1:1", "test-app").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(!client.is_connected());
        assert!(client.try_recv().is_none());
    }

    #[test]
    fn drain_returns_empty_when_no_messages() {
        let client = HotReloadClient::connect("ws://127.0.0.1:1", "test-app").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(client.drain().is_empty());
    }
}
