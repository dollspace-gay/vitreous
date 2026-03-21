// ═══════════════════════════════════════════════════════════════════════════
// vitreous — facade crate
//
// Re-exports all public APIs behind `use vitreous::*` and provides the App
// builder that bridges the reactive system, theming, and platform backends.
// ═══════════════════════════════════════════════════════════════════════════

pub use vitreous_a11y::{AccessibilityInfo, AccessibilityState, LivePoliteness, Role};
pub use vitreous_reactive::*;
pub use vitreous_style::*;
pub use vitreous_widgets::*;

// Selective re-export from vitreous_events to avoid ambiguous glob conflicts
// with vitreous_style (Corners, CursorIcon) and vitreous_widgets (Key).
pub use vitreous_events::{
    DragConfig, DropData, DropEvent, EventHandlers, KeyCode, KeyEvent, Modifiers, MouseButton,
    MouseEvent, ScrollEvent,
};
// The event `Key` type (keyboard key) is available as `vitreous::event_key::Key`
// to avoid conflict with `vitreous_widgets::Key` (list keying).
pub mod event_key {
    pub use vitreous_events::Key;
}

#[cfg(not(target_arch = "wasm32"))]
pub use vitreous_platform::*;

#[cfg(target_arch = "wasm32")]
pub use vitreous_web::*;

// ═══════════════════════════════════════════════════════════════════════════
// theme() — bridges vitreous_style::Theme with vitreous_reactive::use_context
// ═══════════════════════════════════════════════════════════════════════════

/// Access the current theme from within a widget tree.
///
/// The `App` builder provides a default theme via `provide_context`. Calling
/// this outside the widget tree (before `App::run` or `App::mount`) will panic.
pub fn theme() -> Theme {
    use_context::<Theme>()
}

// ═══════════════════════════════════════════════════════════════════════════
// App builder
// ═══════════════════════════════════════════════════════════════════════════

/// Application builder. Configure window properties and theme, then call
/// `run()` on desktop or `mount()` on WASM to start the application.
pub struct App {
    title: String,
    width: u32,
    height: u32,
    min_size: Option<(u32, u32)>,
    max_size: Option<(u32, u32)>,
    resizable: bool,
    app_theme: Theme,
    #[cfg(not(target_arch = "wasm32"))]
    icon: Option<WindowIcon>,
}

impl App {
    pub fn new() -> Self {
        Self {
            title: "Vitreous App".to_owned(),
            width: 800,
            height: 600,
            min_size: None,
            max_size: None,
            resizable: true,
            app_theme: Theme::system(),
            #[cfg(not(target_arch = "wasm32"))]
            icon: None,
        }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_owned();
        self
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn min_size(mut self, width: u32, height: u32) -> Self {
        self.min_size = Some((width, height));
        self
    }

    pub fn max_size(mut self, width: u32, height: u32) -> Self {
        self.max_size = Some((width, height));
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.app_theme = theme;
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn icon(mut self, icon: WindowIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn run(self, root: fn() -> Node) {
        let theme = self.app_theme;
        let mut config = WindowConfig::new()
            .title(&self.title)
            .size(self.width, self.height)
            .resizable(self.resizable);

        if let Some((w, h)) = self.min_size {
            config = config.min_size(w, h);
        }
        if let Some((w, h)) = self.max_size {
            config = config.max_size(w, h);
        }
        if let Some(icon) = self.icon {
            config = config.icon(icon);
        }

        DesktopRuntime::run(config, move || {
            provide_context(theme.clone());
            root()
        });
    }

    #[cfg(target_arch = "wasm32")]
    pub fn run(self, root: fn() -> Node) {
        self.mount(root, "#app");
    }

    #[cfg(target_arch = "wasm32")]
    pub fn mount(self, root: fn() -> Node, element_id: &str) {
        let theme = self.app_theme;
        let _app = WebApp::mount(element_id, move || {
            provide_context(theme.clone());
            root()
        });
        // Keep the app alive — on WASM the event loop is managed by the browser.
        std::mem::forget(_app);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_builder_defaults() {
        let app = App::new();
        assert_eq!(app.title, "Vitreous App");
        assert_eq!(app.width, 800);
        assert_eq!(app.height, 600);
        assert!(app.resizable);
    }

    #[test]
    fn app_builder_chaining() {
        let app = App::new()
            .title("Test")
            .size(400, 300)
            .min_size(200, 150)
            .max_size(1920, 1080)
            .resizable(false)
            .theme(Theme::dark());
        assert_eq!(app.title, "Test");
        assert_eq!(app.width, 400);
        assert_eq!(app.height, 300);
        assert_eq!(app.min_size, Some((200, 150)));
        assert_eq!(app.max_size, Some((1920, 1080)));
        assert!(!app.resizable);
        assert!(app.app_theme.is_dark);
    }

    #[test]
    fn theme_bridge_in_scope() {
        let scope = create_scope(|| {
            provide_context(Theme::dark());
            let t = theme();
            assert!(t.is_dark);
        });
        drop(scope);
    }
}
