use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::{
    CursorIcon as WinitCursorIcon, Fullscreen, Theme as WinitTheme, Window, WindowAttributes,
};

use vitreous_style::CursorIcon;

// winit 0.30 re-exports cursor_icon::CursorIcon as winit::window::CursorIcon.
// This converts to winit::window::Cursor via From impl.

// ═══════════════════════════════════════════════════════════════════════════
// WindowConfig — declarative window configuration
// ═══════════════════════════════════════════════════════════════════════════

/// Configuration for creating a platform window.
///
/// All fields have sensible defaults. Use the builder methods to customise.
#[derive(Clone, Debug)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub resizable: bool,
    pub decorations: bool,
    pub transparent: bool,
    pub always_on_top: bool,
    pub icon: Option<WindowIcon>,
    pub theme: Option<WindowTheme>,
}

/// Window icon data (RGBA pixels).
#[derive(Clone, Debug)]
pub struct WindowIcon {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Preferred window theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowTheme {
    Light,
    Dark,
    System,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: String::from("Vitreous App"),
            width: 800,
            height: 600,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            resizable: true,
            decorations: true,
            transparent: false,
            always_on_top: false,
            icon: None,
            theme: None,
        }
    }
}

impl WindowConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn min_size(mut self, width: u32, height: u32) -> Self {
        self.min_width = Some(width);
        self.min_height = Some(height);
        self
    }

    pub fn max_size(mut self, width: u32, height: u32) -> Self {
        self.max_width = Some(width);
        self.max_height = Some(height);
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn decorations(mut self, decorations: bool) -> Self {
        self.decorations = decorations;
        self
    }

    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    pub fn always_on_top(mut self, always_on_top: bool) -> Self {
        self.always_on_top = always_on_top;
        self
    }

    pub fn icon(mut self, icon: WindowIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn theme(mut self, theme: WindowTheme) -> Self {
        self.theme = Some(theme);
        self
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PlatformWindow — wraps a winit Window
// ═══════════════════════════════════════════════════════════════════════════

/// A platform window wrapping a winit `Window`.
///
/// Created via [`PlatformWindow::create`] during the event loop's `resumed`
/// callback. Provides all window manipulation methods needed by the runtime.
pub struct PlatformWindow {
    window: Window,
}

impl PlatformWindow {
    /// Create a new platform window from the given config within an active
    /// event loop.
    pub fn create(event_loop: &ActiveEventLoop, config: &WindowConfig) -> Self {
        let mut attrs = WindowAttributes::default()
            .with_title(&config.title)
            .with_inner_size(LogicalSize::new(config.width, config.height))
            .with_resizable(config.resizable)
            .with_decorations(config.decorations)
            .with_transparent(config.transparent);

        if let (Some(min_w), Some(min_h)) = (config.min_width, config.min_height) {
            attrs = attrs.with_min_inner_size(LogicalSize::new(min_w, min_h));
        }

        if let (Some(max_w), Some(max_h)) = (config.max_width, config.max_height) {
            attrs = attrs.with_max_inner_size(LogicalSize::new(max_w, max_h));
        }

        if config.always_on_top {
            attrs = attrs.with_window_level(winit::window::WindowLevel::AlwaysOnTop);
        }

        if let Some(ref theme_pref) = config.theme {
            match theme_pref {
                WindowTheme::Light => {
                    attrs = attrs.with_theme(Some(WinitTheme::Light));
                }
                WindowTheme::Dark => {
                    attrs = attrs.with_theme(Some(WinitTheme::Dark));
                }
                WindowTheme::System => {
                    attrs = attrs.with_theme(None);
                }
            }
        }

        let window = event_loop
            .create_window(attrs)
            .expect("failed to create window");

        Self { window }
    }

    /// Request a redraw for the next frame.
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Set the window title.
    pub fn set_title(&self, title: &str) {
        self.window.set_title(title);
    }

    /// Set the window inner size in logical pixels.
    pub fn set_size(&self, width: u32, height: u32) {
        let _ = self
            .window
            .request_inner_size(LogicalSize::new(width, height));
    }

    /// Get the current inner size in logical pixels.
    pub fn inner_size(&self) -> (u32, u32) {
        let physical: PhysicalSize<u32> = self.window.inner_size();
        let scale = self.window.scale_factor();
        let logical = physical.to_logical::<u32>(scale);
        (logical.width, logical.height)
    }

    /// Get the current inner size in physical pixels.
    pub fn inner_size_physical(&self) -> (u32, u32) {
        let physical: PhysicalSize<u32> = self.window.inner_size();
        (physical.width, physical.height)
    }

    /// Get the display scale factor.
    pub fn scale_factor(&self) -> f64 {
        self.window.scale_factor()
    }

    /// Set the cursor icon.
    pub fn set_cursor(&self, cursor: CursorIcon) {
        let winit_cursor = map_cursor(cursor);
        self.window.set_cursor(winit_cursor);
    }

    /// Set fullscreen mode. Pass `true` for borderless fullscreen, `false` to exit.
    pub fn set_fullscreen(&self, fullscreen: bool) {
        if fullscreen {
            self.window
                .set_fullscreen(Some(Fullscreen::Borderless(None)));
        } else {
            self.window.set_fullscreen(None);
        }
    }

    /// Request the window to close.
    pub fn close(&self) {
        self.window.set_visible(false);
    }

    /// Returns whether the window currently has input focus.
    pub fn is_focused(&self) -> bool {
        self.window.has_focus()
    }

    /// Returns the current window theme (Light or Dark).
    pub fn theme(&self) -> WindowTheme {
        match self.window.theme() {
            Some(WinitTheme::Light) => WindowTheme::Light,
            Some(WinitTheme::Dark) => WindowTheme::Dark,
            None => WindowTheme::Light,
        }
    }

    /// Get a reference to the underlying winit window.
    pub fn winit_window(&self) -> &Window {
        &self.window
    }

    /// Set the window position in logical pixels.
    pub fn set_position(&self, x: i32, y: i32) {
        self.window.set_outer_position(LogicalPosition::new(x, y));
    }

    /// Set the window visibility.
    pub fn set_visible(&self, visible: bool) {
        self.window.set_visible(visible);
    }
}

/// Map vitreous `CursorIcon` to winit `CursorIcon`.
fn map_cursor(cursor: CursorIcon) -> WinitCursorIcon {
    match cursor {
        CursorIcon::Default => WinitCursorIcon::Default,
        CursorIcon::Pointer => WinitCursorIcon::Pointer,
        CursorIcon::Text => WinitCursorIcon::Text,
        CursorIcon::Crosshair => WinitCursorIcon::Crosshair,
        CursorIcon::Move => WinitCursorIcon::Move,
        CursorIcon::NotAllowed => WinitCursorIcon::NotAllowed,
        CursorIcon::Grab => WinitCursorIcon::Grab,
        CursorIcon::Grabbing => WinitCursorIcon::Grabbing,
        CursorIcon::ColResize => WinitCursorIcon::ColResize,
        CursorIcon::RowResize => WinitCursorIcon::RowResize,
        CursorIcon::NResize => WinitCursorIcon::NResize,
        CursorIcon::EResize => WinitCursorIcon::EResize,
        CursorIcon::SResize => WinitCursorIcon::SResize,
        CursorIcon::WResize => WinitCursorIcon::WResize,
        CursorIcon::NeResize => WinitCursorIcon::NeResize,
        CursorIcon::NwResize => WinitCursorIcon::NwResize,
        CursorIcon::SeResize => WinitCursorIcon::SeResize,
        CursorIcon::SwResize => WinitCursorIcon::SwResize,
        CursorIcon::EwResize => WinitCursorIcon::EwResize,
        CursorIcon::NsResize => WinitCursorIcon::NsResize,
        CursorIcon::NeswResize => WinitCursorIcon::NeswResize,
        CursorIcon::NwseResize => WinitCursorIcon::NwseResize,
        CursorIcon::Wait => WinitCursorIcon::Wait,
        CursorIcon::Progress => WinitCursorIcon::Progress,
        CursorIcon::Help => WinitCursorIcon::Help,
        CursorIcon::ZoomIn => WinitCursorIcon::ZoomIn,
        CursorIcon::ZoomOut => WinitCursorIcon::ZoomOut,
        CursorIcon::None => WinitCursorIcon::Default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_config_defaults() {
        let config = WindowConfig::default();
        assert_eq!(config.title, "Vitreous App");
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 600);
        assert!(config.resizable);
        assert!(config.decorations);
        assert!(!config.transparent);
        assert!(!config.always_on_top);
        assert!(config.icon.is_none());
        assert!(config.theme.is_none());
        assert!(config.min_width.is_none());
        assert!(config.max_width.is_none());
    }

    #[test]
    fn window_config_builder() {
        let config = WindowConfig::new()
            .title("Test App")
            .size(1024, 768)
            .min_size(320, 240)
            .max_size(1920, 1080)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .theme(WindowTheme::Dark);

        assert_eq!(config.title, "Test App");
        assert_eq!(config.width, 1024);
        assert_eq!(config.height, 768);
        assert_eq!(config.min_width, Some(320));
        assert_eq!(config.min_height, Some(240));
        assert_eq!(config.max_width, Some(1920));
        assert_eq!(config.max_height, Some(1080));
        assert!(!config.resizable);
        assert!(!config.decorations);
        assert!(config.transparent);
        assert!(config.always_on_top);
        assert_eq!(config.theme, Some(WindowTheme::Dark));
    }

    #[test]
    fn cursor_mapping_roundtrip() {
        // Verify all cursor variants map without panic
        let cursors = [
            CursorIcon::Default,
            CursorIcon::Pointer,
            CursorIcon::Text,
            CursorIcon::Crosshair,
            CursorIcon::Move,
            CursorIcon::NotAllowed,
            CursorIcon::Grab,
            CursorIcon::Grabbing,
            CursorIcon::ColResize,
            CursorIcon::RowResize,
            CursorIcon::NResize,
            CursorIcon::EResize,
            CursorIcon::SResize,
            CursorIcon::WResize,
            CursorIcon::NeResize,
            CursorIcon::NwResize,
            CursorIcon::SeResize,
            CursorIcon::SwResize,
            CursorIcon::EwResize,
            CursorIcon::NsResize,
            CursorIcon::NeswResize,
            CursorIcon::NwseResize,
            CursorIcon::Wait,
            CursorIcon::Progress,
            CursorIcon::Help,
            CursorIcon::ZoomIn,
            CursorIcon::ZoomOut,
            CursorIcon::None,
        ];
        for cursor in cursors {
            let _ = map_cursor(cursor);
        }
    }
}
