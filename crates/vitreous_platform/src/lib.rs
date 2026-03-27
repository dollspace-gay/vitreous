pub mod clipboard;
pub mod dialogs;
pub mod event_loop;
pub mod gpu;
pub mod system_info;
pub mod text_engine;
pub mod window;

// Re-export primary public types for convenient access.
pub use clipboard::{ClipboardImage, PlatformClipboard};
pub use dialogs::{FileFilter, MessageLevel, MessageResult, PlatformDialogs};
pub use event_loop::DesktopRuntime;
pub use system_info::{Os, PlatformInfo, SystemTheme};
pub use text_engine::{
    CosmicTextEngine, FontDescriptor, GlyphBitmap, ShapedGlyph, ShapedText, TextMeasurement,
};
pub use window::{PlatformWindow, WindowConfig, WindowIcon, WindowTheme};
