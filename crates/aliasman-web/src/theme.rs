use std::path::{Path, PathBuf};

use aliasman_core::config::WebConfig;

/// Resolved theme colors for both light and dark modes.
pub struct ThemeColors {
    pub primary: String,
    pub primary_hover: String,
    pub accent: String,
    pub accent_hover: String,
    pub dark_primary: String,
    pub dark_primary_hover: String,
    pub dark_accent: String,
    pub dark_accent_hover: String,
}

/// Detected branding assets from the config directory.
pub struct BrandingAssets {
    pub logo: Option<BrandingFile>,
    pub header: Option<BrandingFile>,
}

pub struct BrandingFile {
    pub path: PathBuf,
    pub filename: String,
    pub mime: String,
}

/// View model passed to templates for rendering theme CSS and branding.
pub struct ThemeContext {
    pub primary: String,
    pub primary_hover: String,
    pub accent: String,
    pub accent_hover: String,
    pub dark_primary: String,
    pub dark_primary_hover: String,
    pub dark_accent: String,
    pub dark_accent_hover: String,
    pub logo_url: Option<String>,
    pub header_url: Option<String>,
}

struct Preset {
    primary: &'static str,
    primary_hover: &'static str,
    accent: &'static str,
    accent_hover: &'static str,
    dark_primary: &'static str,
    dark_primary_hover: &'static str,
    dark_accent: &'static str,
    dark_accent_hover: &'static str,
}

const BLUE: Preset = Preset {
    primary: "#2563eb",
    primary_hover: "#1d4ed8",
    accent: "#3b82f6",
    accent_hover: "#60a5fa",
    dark_primary: "#60a5fa",
    dark_primary_hover: "#93bbfd",
    dark_accent: "#93c5fd",
    dark_accent_hover: "#bfdbfe",
};

const GREEN: Preset = Preset {
    primary: "#059669",
    primary_hover: "#047857",
    accent: "#10b981",
    accent_hover: "#34d399",
    dark_primary: "#34d399",
    dark_primary_hover: "#6ee7b7",
    dark_accent: "#6ee7b7",
    dark_accent_hover: "#a7f3d0",
};

const PURPLE: Preset = Preset {
    primary: "#7c3aed",
    primary_hover: "#6d28d9",
    accent: "#8b5cf6",
    accent_hover: "#a78bfa",
    dark_primary: "#a78bfa",
    dark_primary_hover: "#c4b5fd",
    dark_accent: "#c4b5fd",
    dark_accent_hover: "#ddd6fe",
};

const ROSE: Preset = Preset {
    primary: "#e11d48",
    primary_hover: "#be123c",
    accent: "#f43f5e",
    accent_hover: "#fb7185",
    dark_primary: "#fb7185",
    dark_primary_hover: "#fda4af",
    dark_accent: "#fda4af",
    dark_accent_hover: "#fecdd3",
};

const AMBER: Preset = Preset {
    primary: "#d97706",
    primary_hover: "#b45309",
    accent: "#f59e0b",
    accent_hover: "#fbbf24",
    dark_primary: "#fbbf24",
    dark_primary_hover: "#fcd34d",
    dark_accent: "#fcd34d",
    dark_accent_hover: "#fde68a",
};

fn preset_by_name(name: &str) -> &'static Preset {
    match name {
        "green" => &GREEN,
        "purple" => &PURPLE,
        "rose" => &ROSE,
        "amber" => &AMBER,
        _ => &BLUE,
    }
}

/// Resolve a `WebConfig` into final `ThemeColors`, applying any per-color overrides.
pub fn resolve_theme(web_config: &WebConfig) -> ThemeColors {
    let preset = preset_by_name(&web_config.theme);

    let (primary, primary_hover, accent, accent_hover) = match &web_config.colors {
        Some(overrides) => (
            overrides
                .primary
                .as_deref()
                .unwrap_or(preset.primary)
                .to_string(),
            overrides
                .primary_hover
                .as_deref()
                .unwrap_or(preset.primary_hover)
                .to_string(),
            overrides
                .accent
                .as_deref()
                .unwrap_or(preset.accent)
                .to_string(),
            overrides
                .accent_hover
                .as_deref()
                .unwrap_or(preset.accent_hover)
                .to_string(),
        ),
        None => (
            preset.primary.to_string(),
            preset.primary_hover.to_string(),
            preset.accent.to_string(),
            preset.accent_hover.to_string(),
        ),
    };

    ThemeColors {
        primary,
        primary_hover,
        accent,
        accent_hover,
        dark_primary: preset.dark_primary.to_string(),
        dark_primary_hover: preset.dark_primary_hover.to_string(),
        dark_accent: preset.dark_accent.to_string(),
        dark_accent_hover: preset.dark_accent_hover.to_string(),
    }
}

/// Detect branding assets in `{config_dir}/branding/`.
pub fn detect_branding(config_dir: &Path) -> BrandingAssets {
    let branding_dir = config_dir.join("branding");

    BrandingAssets {
        logo: detect_file(&branding_dir, "logo"),
        header: detect_file(&branding_dir, "header"),
    }
}

fn detect_file(dir: &Path, base_name: &str) -> Option<BrandingFile> {
    const EXTENSIONS: &[(&str, &str)] = &[
        ("svg", "image/svg+xml"),
        ("png", "image/png"),
        ("jpg", "image/jpeg"),
    ];

    for (ext, mime) in EXTENSIONS {
        let filename = format!("{}.{}", base_name, ext);
        let path = dir.join(&filename);
        if path.is_file() {
            return Some(BrandingFile {
                path,
                filename,
                mime: mime.to_string(),
            });
        }
    }
    None
}

impl ThemeColors {
    /// Build a `ThemeContext` view model from resolved colors and branding assets.
    pub fn to_context(&self, branding: &BrandingAssets) -> ThemeContext {
        ThemeContext {
            primary: self.primary.clone(),
            primary_hover: self.primary_hover.clone(),
            accent: self.accent.clone(),
            accent_hover: self.accent_hover.clone(),
            dark_primary: self.dark_primary.clone(),
            dark_primary_hover: self.dark_primary_hover.clone(),
            dark_accent: self.dark_accent.clone(),
            dark_accent_hover: self.dark_accent_hover.clone(),
            logo_url: branding
                .logo
                .as_ref()
                .map(|f| format!("/branding/{}", f.filename)),
            header_url: branding
                .header
                .as_ref()
                .map(|f| format!("/branding/{}", f.filename)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aliasman_core::config::ThemeColorOverrides;

    #[test]
    fn test_resolve_default_theme() {
        let config = WebConfig::default();
        let colors = resolve_theme(&config);
        assert_eq!(colors.primary, "#2563eb");
        assert_eq!(colors.dark_primary, "#60a5fa");
    }

    #[test]
    fn test_resolve_named_preset() {
        let config = WebConfig {
            theme: "purple".to_string(),
            colors: None,
        };
        let colors = resolve_theme(&config);
        assert_eq!(colors.primary, "#7c3aed");
        assert_eq!(colors.accent, "#8b5cf6");
    }

    #[test]
    fn test_resolve_with_overrides() {
        let config = WebConfig {
            theme: "blue".to_string(),
            colors: Some(ThemeColorOverrides {
                primary: Some("#111111".to_string()),
                primary_hover: None,
                accent: None,
                accent_hover: Some("#222222".to_string()),
            }),
        };
        let colors = resolve_theme(&config);
        assert_eq!(colors.primary, "#111111");
        assert_eq!(colors.primary_hover, "#1d4ed8"); // falls back to blue preset
        assert_eq!(colors.accent_hover, "#222222");
    }

    #[test]
    fn test_unknown_preset_falls_back_to_blue() {
        let config = WebConfig {
            theme: "neon".to_string(),
            colors: None,
        };
        let colors = resolve_theme(&config);
        assert_eq!(colors.primary, "#2563eb");
    }

    #[test]
    fn test_detect_branding_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let branding = detect_branding(dir.path());
        assert!(branding.logo.is_none());
        assert!(branding.header.is_none());
    }

    #[test]
    fn test_detect_branding_with_files() {
        let dir = tempfile::tempdir().unwrap();
        let branding_dir = dir.path().join("branding");
        std::fs::create_dir(&branding_dir).unwrap();
        std::fs::write(branding_dir.join("logo.png"), b"fake png").unwrap();
        std::fs::write(branding_dir.join("header.jpg"), b"fake jpg").unwrap();

        let branding = detect_branding(dir.path());
        let logo = branding.logo.unwrap();
        assert_eq!(logo.filename, "logo.png");
        assert_eq!(logo.mime, "image/png");

        let header = branding.header.unwrap();
        assert_eq!(header.filename, "header.jpg");
        assert_eq!(header.mime, "image/jpeg");
    }

    #[test]
    fn test_detect_branding_svg_priority() {
        let dir = tempfile::tempdir().unwrap();
        let branding_dir = dir.path().join("branding");
        std::fs::create_dir(&branding_dir).unwrap();
        std::fs::write(branding_dir.join("logo.svg"), b"<svg/>").unwrap();
        std::fs::write(branding_dir.join("logo.png"), b"fake png").unwrap();

        let branding = detect_branding(dir.path());
        let logo = branding.logo.unwrap();
        assert_eq!(logo.filename, "logo.svg");
        assert_eq!(logo.mime, "image/svg+xml");
    }

    #[test]
    fn test_theme_context_urls() {
        let colors = resolve_theme(&WebConfig::default());
        let branding = BrandingAssets {
            logo: Some(BrandingFile {
                path: PathBuf::from("/tmp/logo.png"),
                filename: "logo.png".to_string(),
                mime: "image/png".to_string(),
            }),
            header: None,
        };
        let ctx = colors.to_context(&branding);
        assert_eq!(ctx.logo_url.as_deref(), Some("/branding/logo.png"));
        assert!(ctx.header_url.is_none());
    }
}
