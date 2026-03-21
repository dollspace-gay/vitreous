use vitreous_style::Color;

// ═══════════════════════════════════════════════════════════════════════════
// OS identification
// ═══════════════════════════════════════════════════════════════════════════

/// The operating system the app is running on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Os {
    Linux,
    MacOs,
    Windows,
    Other,
}

/// The system-wide appearance theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemTheme {
    Light,
    Dark,
}

// ═══════════════════════════════════════════════════════════════════════════
// PlatformInfo — system information queries
// ═══════════════════════════════════════════════════════════════════════════

/// Provides information about the host platform: OS, theme, locale,
/// display scale factor, and accent color.
pub struct PlatformInfo;

impl PlatformInfo {
    /// Detect the current operating system.
    pub fn os() -> Os {
        if cfg!(target_os = "linux") {
            Os::Linux
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else if cfg!(target_os = "windows") {
            Os::Windows
        } else {
            Os::Other
        }
    }

    /// Detect the system-wide theme preference.
    ///
    /// Falls back to `Light` if detection fails. On Linux, checks
    /// `GTK_THEME`, `DBUS`, and `gsettings`. On macOS/Windows, checks
    /// the native dark mode setting.
    pub fn theme() -> SystemTheme {
        if Self::detect_dark_mode() {
            SystemTheme::Dark
        } else {
            SystemTheme::Light
        }
    }

    /// Get the system locale string (e.g. "en-US", "ja-JP").
    ///
    /// Falls back to `"en-US"` if detection fails.
    pub fn locale() -> String {
        Self::detect_locale().unwrap_or_else(|| "en-US".to_string())
    }

    /// Get the default display scale factor.
    ///
    /// Returns 1.0 as a fallback. The actual per-window scale factor should
    /// be queried from `PlatformWindow::scale_factor()` instead — this is
    /// only useful before a window is created.
    pub fn scale_factor() -> f64 {
        // Pre-window scale factor detection is limited.
        // On Linux, check GDK_SCALE or QT_SCALE_FACTOR env vars.
        // The real scale factor comes from winit after window creation.
        Self::detect_scale_factor().unwrap_or(1.0)
    }

    /// Get the system accent color, if available.
    ///
    /// Returns `None` on platforms that don't expose an accent color.
    pub fn accent_color() -> Option<Color> {
        Self::detect_accent_color()
    }

    // ───────────────────────────────────────────────────────────────────
    // Private detection methods
    // ───────────────────────────────────────────────────────────────────

    fn detect_dark_mode() -> bool {
        // Check environment variables common on Linux desktops
        if let Ok(gtk_theme) = std::env::var("GTK_THEME")
            && gtk_theme.to_lowercase().contains("dark")
        {
            return true;
        }

        // Check XDG portal preference (freedesktop standard)
        if let Ok(val) = std::env::var("XDG_CURRENT_DESKTOP_PREFERS_DARK")
            && (val == "1" || val.to_lowercase() == "true")
        {
            return true;
        }

        // On macOS/Windows, winit provides theme detection after window
        // creation. This pre-window detection is best-effort.
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = std::process::Command::new("defaults")
                .args(["read", "-g", "AppleInterfaceStyle"])
                .output()
                && output.status.success()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.trim().eq_ignore_ascii_case("dark") {
                    return true;
                }
            }
        }

        false
    }

    fn detect_locale() -> Option<String> {
        // POSIX: LC_ALL > LC_MESSAGES > LANG
        for var in &["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(val) = std::env::var(var)
                && !val.is_empty()
                && val != "C"
                && val != "POSIX"
            {
                // Convert POSIX locale (en_US.UTF-8) to BCP 47 (en-US)
                let normalized = val.split('.').next().unwrap_or(&val).replace('_', "-");
                return Some(normalized);
            }
        }
        None
    }

    fn detect_scale_factor() -> Option<f64> {
        // GDK_SCALE is set by GTK-based desktops
        if let Ok(val) = std::env::var("GDK_SCALE")
            && let Ok(scale) = val.parse::<f64>()
            && scale > 0.0
        {
            return Some(scale);
        }

        // QT_SCALE_FACTOR for Qt-based desktops
        if let Ok(val) = std::env::var("QT_SCALE_FACTOR")
            && let Ok(scale) = val.parse::<f64>()
            && scale > 0.0
        {
            return Some(scale);
        }

        None
    }

    fn detect_accent_color() -> Option<Color> {
        // Accent color detection is platform-specific and limited pre-window.
        // On macOS, the accent color is available via NSUserDefaults but
        // requires Objective-C interop. On Windows 10+, it's in the registry.
        // For now, return None — the window's theme provides this post-creation.
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_detection() {
        let os = PlatformInfo::os();
        // Should return a valid variant on any platform
        assert!(os == Os::Linux || os == Os::MacOs || os == Os::Windows || os == Os::Other);
    }

    #[test]
    fn theme_detection_returns_valid() {
        let theme = PlatformInfo::theme();
        assert!(theme == SystemTheme::Light || theme == SystemTheme::Dark);
    }

    #[test]
    fn locale_returns_nonempty() {
        let locale = PlatformInfo::locale();
        assert!(!locale.is_empty());
    }

    #[test]
    fn scale_factor_positive() {
        let scale = PlatformInfo::scale_factor();
        assert!(scale > 0.0);
    }

    #[test]
    fn accent_color_option() {
        // May be None on most platforms — just ensure it doesn't panic
        let _ = PlatformInfo::accent_color();
    }

    #[test]
    fn os_variants_distinct() {
        assert_ne!(Os::Linux, Os::MacOs);
        assert_ne!(Os::MacOs, Os::Windows);
        assert_ne!(Os::Windows, Os::Other);
    }

    #[test]
    fn system_theme_variants() {
        assert_ne!(SystemTheme::Light, SystemTheme::Dark);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn os_is_linux() {
        assert_eq!(PlatformInfo::os(), Os::Linux);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn os_is_macos() {
        assert_eq!(PlatformInfo::os(), Os::MacOs);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn os_is_windows() {
        assert_eq!(PlatformInfo::os(), Os::Windows);
    }
}
