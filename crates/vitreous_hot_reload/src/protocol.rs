use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Default port for the hot reload WebSocket server.
pub const DEFAULT_PORT: u16 = 3742;

/// Message sent from the hot reload server to connected clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerMessage {
    /// A source file was created, modified, or removed.
    FileChanged(FileChange),
    /// A rebuild was triggered due to a source change.
    BuildStarted,
    /// Rebuild completed successfully — client should reload.
    BuildComplete,
    /// Rebuild failed with compiler output.
    BuildFailed { errors: String },
    /// Server is shutting down.
    Shutdown,
}

/// Message sent from the runtime client to the server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClientMessage {
    /// Client connected and identifies itself.
    Hello { app_name: String },
    /// Client requests a rebuild.
    RequestBuild,
}

/// Describes a single file change detected by the watcher.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileChange {
    /// Path to the changed file, relative to the project root.
    pub path: PathBuf,
    /// What kind of content changed.
    pub kind: ChangeKind,
    /// Whether the file was created, modified, or removed.
    pub event: FileEvent,
}

/// Classification of what kind of content changed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChangeKind {
    /// A style or theme source file — may support hot patching without recompilation.
    Style,
    /// A source file with logic — requires recompilation.
    Source,
    /// An asset file (image, font, etc.).
    Asset,
}

/// Whether the file was created, modified, or removed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileEvent {
    Created,
    Modified,
    Removed,
}

impl ServerMessage {
    /// Serialize to a JSON string for WebSocket transmission.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl ClientMessage {
    /// Serialize to a JSON string for WebSocket transmission.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_message_roundtrip_file_changed() {
        let msg = ServerMessage::FileChanged(FileChange {
            path: PathBuf::from("src/main.rs"),
            kind: ChangeKind::Source,
            event: FileEvent::Modified,
        });
        let json = msg.to_json().unwrap();
        assert_eq!(ServerMessage::from_json(&json).unwrap(), msg);
    }

    #[test]
    fn server_message_roundtrip_build_lifecycle() {
        for msg in [
            ServerMessage::BuildStarted,
            ServerMessage::BuildComplete,
            ServerMessage::BuildFailed {
                errors: "error[E0308]: mismatched types".into(),
            },
            ServerMessage::Shutdown,
        ] {
            let json = msg.to_json().unwrap();
            assert_eq!(ServerMessage::from_json(&json).unwrap(), msg);
        }
    }

    #[test]
    fn client_message_roundtrip() {
        for msg in [
            ClientMessage::Hello {
                app_name: "my-app".into(),
            },
            ClientMessage::RequestBuild,
        ] {
            let json = msg.to_json().unwrap();
            assert_eq!(ClientMessage::from_json(&json).unwrap(), msg);
        }
    }

    #[test]
    fn file_change_all_variants() {
        for kind in [ChangeKind::Style, ChangeKind::Source, ChangeKind::Asset] {
            for event in [FileEvent::Created, FileEvent::Modified, FileEvent::Removed] {
                let msg = ServerMessage::FileChanged(FileChange {
                    path: PathBuf::from("test.rs"),
                    kind,
                    event,
                });
                let json = msg.to_json().unwrap();
                assert_eq!(ServerMessage::from_json(&json).unwrap(), msg);
            }
        }
    }

    #[test]
    fn invalid_json_returns_error() {
        assert!(ServerMessage::from_json("not json").is_err());
        assert!(ClientMessage::from_json("{\"Unknown\":null}").is_err());
    }
}
