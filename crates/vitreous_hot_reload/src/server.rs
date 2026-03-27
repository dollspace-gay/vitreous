use crate::protocol::{ChangeKind, ClientMessage, FileChange, FileEvent, ServerMessage, DEFAULT_PORT};
use futures_util::{SinkExt, StreamExt};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Command;
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

/// Configuration for the hot reload server.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Root directory to watch for changes.
    pub watch_dir: PathBuf,
    /// Port for the WebSocket server.
    pub port: u16,
    /// Source file extensions that trigger change notifications.
    pub source_extensions: HashSet<String>,
    /// Asset file extensions that trigger asset reload.
    pub asset_extensions: HashSet<String>,
    /// Whether to trigger `cargo build` on source changes.
    pub auto_build: bool,
    /// Debounce interval in milliseconds — events for the same file within this
    /// window are coalesced into a single notification.
    pub debounce_ms: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            watch_dir: PathBuf::from("."),
            port: DEFAULT_PORT,
            source_extensions: ["rs", "toml"]
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            asset_extensions: [
                "png", "jpg", "jpeg", "svg", "gif", "webp", "ttf", "otf", "woff", "woff2",
            ]
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
            auto_build: true,
            debounce_ms: 100,
        }
    }
}

/// The hot reload development server.
///
/// Watches the project directory for file changes, classifies them (style, source,
/// asset), and broadcasts notifications to connected WebSocket clients. Optionally
/// triggers `cargo build` on source changes.
pub struct HotReloadServer {
    config: ServerConfig,
    broadcast_tx: broadcast::Sender<ServerMessage>,
}

impl HotReloadServer {
    pub fn new(config: ServerConfig) -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            config,
            broadcast_tx: tx,
        }
    }

    /// Run the server. This blocks until the process is terminated.
    pub async fn run(&self) -> Result<(), ServerError> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.config.port));
        let listener = TcpListener::bind(addr).await.map_err(ServerError::Bind)?;

        // Spawn the file watcher on a dedicated OS thread (notify is synchronous).
        let watcher_tx = self.broadcast_tx.clone();
        let source_exts = self.config.source_extensions.clone();
        let asset_exts = self.config.asset_extensions.clone();
        let watch_dir = self.config.watch_dir.clone();
        let debounce_ms = self.config.debounce_ms;

        std::thread::Builder::new()
            .name("vitreous-file-watcher".into())
            .spawn(move || {
                if let Err(e) =
                    run_file_watcher(&watch_dir, &source_exts, &asset_exts, debounce_ms, watcher_tx)
                {
                    eprintln!("[vitreous dev] File watcher error: {e}");
                }
            })
            .map_err(ServerError::SpawnWatcher)?;

        // Spawn the auto-builder task.
        if self.config.auto_build {
            let build_tx = self.broadcast_tx.clone();
            let mut build_rx = self.broadcast_tx.subscribe();
            let build_dir = self.config.watch_dir.clone();

            tokio::spawn(async move {
                auto_build_loop(&mut build_rx, &build_tx, &build_dir).await;
            });
        }

        eprintln!(
            "[vitreous dev] Hot reload server listening on ws://127.0.0.1:{}",
            self.config.port
        );

        loop {
            let (stream, peer) = listener.accept().await.map_err(ServerError::Accept)?;
            let rx = self.broadcast_tx.subscribe();
            tokio::spawn(handle_connection(stream, peer, rx));
        }
    }
}

// ── File watcher ─────────────────────────────────────────────────────────

fn run_file_watcher(
    watch_dir: &Path,
    source_exts: &HashSet<String>,
    asset_exts: &HashSet<String>,
    debounce_ms: u64,
    tx: broadcast::Sender<ServerMessage>,
) -> Result<(), notify::Error> {
    let (notify_tx, notify_rx) = std::sync::mpsc::channel();
    let mut watcher = RecommendedWatcher::new(notify_tx, Config::default())?;
    watcher.watch(watch_dir, RecursiveMode::Recursive)?;

    let debounce = Duration::from_millis(debounce_ms);
    let mut last_seen: HashMap<PathBuf, Instant> = HashMap::new();

    for result in notify_rx {
        let event = match result {
            Ok(event) => event,
            Err(e) => {
                eprintln!("[vitreous dev] Watch error: {e}");
                continue;
            }
        };

        let file_event = match event.kind {
            EventKind::Create(_) => FileEvent::Created,
            EventKind::Modify(_) => FileEvent::Modified,
            EventKind::Remove(_) => FileEvent::Removed,
            _ => continue,
        };

        let now = Instant::now();

        for path in &event.paths {
            // Debounce: skip if we saw this path recently.
            if last_seen.get(path).is_some_and(|&last| now.duration_since(last) < debounce) {
                continue;
            }
            last_seen.insert(path.clone(), now);

            if should_ignore(path) {
                continue;
            }

            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

            let kind = if asset_exts.contains(ext) {
                ChangeKind::Asset
            } else if source_exts.contains(ext) {
                classify_source_file(path)
            } else {
                continue;
            };

            let relative = path.strip_prefix(watch_dir).unwrap_or(path);
            let change = FileChange {
                path: relative.to_path_buf(),
                kind,
                event: file_event,
            };

            let _ = tx.send(ServerMessage::FileChanged(change));
        }
    }

    Ok(())
}

/// Returns `true` for paths that should be ignored (build artifacts, VCS, etc.).
pub fn should_ignore(path: &Path) -> bool {
    path.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        s == "target" || s == ".git" || s == "node_modules" || s.starts_with('.')
    })
}

/// Classify a source file change. Files whose path contains "style" or "theme"
/// are classified as [`ChangeKind::Style`]; all others as [`ChangeKind::Source`].
pub fn classify_source_file(path: &Path) -> ChangeKind {
    let path_str = path.to_string_lossy();
    if path_str.contains("style") || path_str.contains("theme") {
        ChangeKind::Style
    } else {
        ChangeKind::Source
    }
}

// ── Auto-build ───────────────────────────────────────────────────────────

async fn auto_build_loop(
    rx: &mut broadcast::Receiver<ServerMessage>,
    tx: &broadcast::Sender<ServerMessage>,
    build_dir: &Path,
) {
    loop {
        let msg = match rx.recv().await {
            Ok(m) => m,
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        };
        if let ServerMessage::FileChanged(ref change) = msg
            && change.kind == ChangeKind::Source
        {
            let _ = tx.send(ServerMessage::BuildStarted);
            match run_cargo_build(build_dir).await {
                Ok(()) => {
                    let _ = tx.send(ServerMessage::BuildComplete);
                }
                Err(errors) => {
                    let _ = tx.send(ServerMessage::BuildFailed { errors });
                }
            }
        }
    }
}

async fn run_cargo_build(dir: &Path) -> Result<(), String> {
    let output = Command::new("cargo")
        .arg("build")
        .current_dir(dir)
        .output()
        .await
        .map_err(|e| format!("failed to run cargo build: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

// ── WebSocket connection handler ─────────────────────────────────────────

async fn handle_connection(
    stream: TcpStream,
    peer: SocketAddr,
    mut rx: broadcast::Receiver<ServerMessage>,
) {
    let ws = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("[vitreous dev] WebSocket handshake failed for {peer}: {e}");
            return;
        }
    };

    let (mut ws_tx, mut ws_rx) = ws.split();

    // Read incoming client messages in the background.
    let client_task = tokio::spawn(async move {
        while let Some(frame) = ws_rx.next().await {
            let Ok(Message::Text(text)) = frame else {
                continue;
            };
            match ClientMessage::from_json(&text) {
                Ok(ClientMessage::Hello { app_name }) => {
                    eprintln!("[vitreous dev] Client connected: {app_name} ({peer})");
                }
                Ok(ClientMessage::RequestBuild) => {
                    eprintln!("[vitreous dev] Build requested by {peer}");
                }
                Err(_) => {}
            }
        }
    });

    // Forward broadcast messages to this WebSocket client.
    loop {
        match rx.recv().await {
            Ok(msg) => {
                let Ok(json) = msg.to_json() else { continue };
                if ws_tx.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                eprintln!("[vitreous dev] Client {peer} lagged by {n} messages");
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }

    client_task.abort();
    eprintln!("[vitreous dev] Client disconnected: {peer}");
}

// ── Errors ───────────────────────────────────────────────────────────────

/// Errors that can occur while running the hot reload server.
#[derive(Debug)]
pub enum ServerError {
    /// Failed to bind the TCP listener.
    Bind(std::io::Error),
    /// Failed to accept an incoming connection.
    Accept(std::io::Error),
    /// Failed to spawn the file watcher thread.
    SpawnWatcher(std::io::Error),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bind(e) => write!(f, "failed to bind WebSocket server: {e}"),
            Self::Accept(e) => write!(f, "failed to accept connection: {e}"),
            Self::SpawnWatcher(e) => write!(f, "failed to spawn file watcher thread: {e}"),
        }
    }
}

impl std::error::Error for ServerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Bind(e) | Self::Accept(e) | Self::SpawnWatcher(e) => Some(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_style_files() {
        assert_eq!(
            classify_source_file(Path::new("crates/vitreous_style/src/lib.rs")),
            ChangeKind::Style
        );
        assert_eq!(
            classify_source_file(Path::new("src/theme.rs")),
            ChangeKind::Style
        );
        assert_eq!(
            classify_source_file(Path::new("src/my_styles.rs")),
            ChangeKind::Style
        );
    }

    #[test]
    fn classify_source_files() {
        assert_eq!(
            classify_source_file(Path::new("src/main.rs")),
            ChangeKind::Source
        );
        assert_eq!(
            classify_source_file(Path::new("crates/vitreous_widgets/src/node.rs")),
            ChangeKind::Source
        );
        assert_eq!(
            classify_source_file(Path::new("Cargo.toml")),
            ChangeKind::Source
        );
    }

    #[test]
    fn ignore_target_and_hidden() {
        assert!(should_ignore(Path::new("target/debug/build/foo")));
        assert!(should_ignore(Path::new(".git/objects/ab/1234")));
        assert!(should_ignore(Path::new("node_modules/foo/index.js")));
        assert!(should_ignore(Path::new(".hidden/file.rs")));
    }

    #[test]
    fn do_not_ignore_source_paths() {
        assert!(!should_ignore(Path::new("src/main.rs")));
        assert!(!should_ignore(Path::new("crates/vitreous_style/src/lib.rs")));
        assert!(!should_ignore(Path::new("examples/counter/src/main.rs")));
    }

    #[test]
    fn default_config_has_expected_extensions() {
        let config = ServerConfig::default();
        assert!(config.source_extensions.contains("rs"));
        assert!(config.source_extensions.contains("toml"));
        assert!(config.asset_extensions.contains("png"));
        assert!(config.asset_extensions.contains("ttf"));
        assert_eq!(config.port, DEFAULT_PORT);
        assert!(config.auto_build);
    }
}
