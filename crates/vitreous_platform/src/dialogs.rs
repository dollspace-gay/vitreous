use std::path::PathBuf;

use rfd::{
    FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel as RfdMessageLevel,
};

// ═══════════════════════════════════════════════════════════════════════════
// Dialog types
// ═══════════════════════════════════════════════════════════════════════════

/// Filter for file dialogs (e.g. "Images", &["png", "jpg"]).
#[derive(Debug, Clone)]
pub struct FileFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

impl FileFilter {
    pub fn new(name: impl Into<String>, extensions: &[&str]) -> Self {
        Self {
            name: name.into(),
            extensions: extensions.iter().map(|s| (*s).to_string()).collect(),
        }
    }
}

/// Severity level for message box dialogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageLevel {
    Info,
    Warning,
    Error,
}

/// Result of a message box with OK/Cancel buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageResult {
    Ok,
    Cancel,
}

// ═══════════════════════════════════════════════════════════════════════════
// PlatformDialogs — native file/message dialogs via rfd
// ═══════════════════════════════════════════════════════════════════════════

/// Native file and message dialogs backed by `rfd`.
///
/// All methods are blocking — they show a native dialog and wait for the
/// user to respond. Call from the main thread only.
pub struct PlatformDialogs;

impl PlatformDialogs {
    /// Show an "open file" dialog. Returns `None` if the user cancelled.
    pub fn open_file(
        title: &str,
        default_dir: Option<&PathBuf>,
        filters: &[FileFilter],
    ) -> Option<PathBuf> {
        let mut dialog = FileDialog::new().set_title(title);
        if let Some(dir) = default_dir {
            dialog = dialog.set_directory(dir);
        }
        for filter in filters {
            let ext_refs: Vec<&str> = filter.extensions.iter().map(String::as_str).collect();
            dialog = dialog.add_filter(&filter.name, &ext_refs);
        }
        dialog.pick_file()
    }

    /// Show an "open files" dialog for multiple selection.
    /// Returns an empty vec if the user cancelled.
    pub fn open_files(
        title: &str,
        default_dir: Option<&PathBuf>,
        filters: &[FileFilter],
    ) -> Vec<PathBuf> {
        let mut dialog = FileDialog::new().set_title(title);
        if let Some(dir) = default_dir {
            dialog = dialog.set_directory(dir);
        }
        for filter in filters {
            let ext_refs: Vec<&str> = filter.extensions.iter().map(String::as_str).collect();
            dialog = dialog.add_filter(&filter.name, &ext_refs);
        }
        dialog.pick_files().unwrap_or_default()
    }

    /// Show a "save file" dialog. Returns `None` if the user cancelled.
    pub fn save_file(
        title: &str,
        default_dir: Option<&PathBuf>,
        default_name: Option<&str>,
        filters: &[FileFilter],
    ) -> Option<PathBuf> {
        let mut dialog = FileDialog::new().set_title(title);
        if let Some(dir) = default_dir {
            dialog = dialog.set_directory(dir);
        }
        if let Some(name) = default_name {
            dialog = dialog.set_file_name(name);
        }
        for filter in filters {
            let ext_refs: Vec<&str> = filter.extensions.iter().map(String::as_str).collect();
            dialog = dialog.add_filter(&filter.name, &ext_refs);
        }
        dialog.save_file()
    }

    /// Show an "open directory" dialog. Returns `None` if the user cancelled.
    pub fn open_directory(title: &str, default_dir: Option<&PathBuf>) -> Option<PathBuf> {
        let mut dialog = FileDialog::new().set_title(title);
        if let Some(dir) = default_dir {
            dialog = dialog.set_directory(dir);
        }
        dialog.pick_folder()
    }

    /// Show a message box with OK/Cancel buttons.
    pub fn message_box(title: &str, message: &str, level: self::MessageLevel) -> MessageResult {
        let rfd_level = match level {
            self::MessageLevel::Info => RfdMessageLevel::Info,
            self::MessageLevel::Warning => RfdMessageLevel::Warning,
            self::MessageLevel::Error => RfdMessageLevel::Error,
        };

        let result = MessageDialog::new()
            .set_title(title)
            .set_description(message)
            .set_level(rfd_level)
            .set_buttons(MessageButtons::OkCancel)
            .show();

        match result {
            MessageDialogResult::Ok => MessageResult::Ok,
            _ => MessageResult::Cancel,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_filter_construction() {
        let filter = FileFilter::new("Images", &["png", "jpg", "gif"]);
        assert_eq!(filter.name, "Images");
        assert_eq!(filter.extensions, vec!["png", "jpg", "gif"]);
    }

    #[test]
    fn file_filter_empty_extensions() {
        let filter = FileFilter::new("All", &[]);
        assert!(filter.extensions.is_empty());
    }

    #[test]
    fn message_level_equality() {
        assert_eq!(MessageLevel::Info, MessageLevel::Info);
        assert_ne!(MessageLevel::Info, MessageLevel::Error);
    }

    #[test]
    fn message_result_equality() {
        assert_eq!(MessageResult::Ok, MessageResult::Ok);
        assert_ne!(MessageResult::Ok, MessageResult::Cancel);
    }
}
