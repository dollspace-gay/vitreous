use arboard::Clipboard;

// ═══════════════════════════════════════════════════════════════════════════
// Clipboard image type
// ═══════════════════════════════════════════════════════════════════════════

/// An RGBA image for clipboard operations.
#[derive(Debug, Clone)]
pub struct ClipboardImage {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

// ═══════════════════════════════════════════════════════════════════════════
// PlatformClipboard — system clipboard access via arboard
// ═══════════════════════════════════════════════════════════════════════════

/// System clipboard access backed by `arboard`.
///
/// Each method creates a short-lived clipboard connection. On Linux/Wayland,
/// clipboard contents are only available while the owning process is alive.
pub struct PlatformClipboard;

impl PlatformClipboard {
    /// Read text from the system clipboard.
    ///
    /// Returns `None` if the clipboard is empty or contains non-text data.
    pub fn read_text() -> Option<String> {
        let mut clipboard = Clipboard::new().ok()?;
        clipboard.get_text().ok()
    }

    /// Write text to the system clipboard.
    ///
    /// Returns `true` if the operation succeeded.
    pub fn write_text(text: &str) -> bool {
        let Ok(mut clipboard) = Clipboard::new() else {
            return false;
        };
        clipboard.set_text(text).is_ok()
    }

    /// Read an image from the system clipboard.
    ///
    /// Returns `None` if the clipboard doesn't contain image data.
    pub fn read_image() -> Option<ClipboardImage> {
        let mut clipboard = Clipboard::new().ok()?;
        let image = clipboard.get_image().ok()?;
        Some(ClipboardImage {
            rgba: image.bytes.into_owned(),
            width: image.width as u32,
            height: image.height as u32,
        })
    }

    /// Write an image to the system clipboard.
    ///
    /// Returns `true` if the operation succeeded.
    pub fn write_image(image: &ClipboardImage) -> bool {
        let Ok(mut clipboard) = Clipboard::new() else {
            return false;
        };
        let img_data = arboard::ImageData {
            width: image.width as usize,
            height: image.height as usize,
            bytes: std::borrow::Cow::Borrowed(&image.rgba),
        };
        clipboard.set_image(img_data).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_image_construction() {
        let img = ClipboardImage {
            rgba: vec![255, 0, 0, 255],
            width: 1,
            height: 1,
        };
        assert_eq!(img.width, 1);
        assert_eq!(img.height, 1);
        assert_eq!(img.rgba.len(), 4);
    }

    // Note: AC-6 (write_text + read_text roundtrip) requires a running
    // display server and is best verified in integration tests, not unit tests.
    // On headless CI, Clipboard::new() may fail.
}
