// Theme presets for TUI configuration

use crate::config::{paths, Config, StyleConfig, StyleMode};

// Import all theme modules
use super::{
    theme_cometix, theme_default, theme_gruvbox, theme_minimal, theme_nord, theme_powerline_dark,
    theme_powerline_light, theme_powerline_rose_pine, theme_powerline_tokyo_night,
};

pub struct ThemePresets;

impl ThemePresets {
    pub const BUILTIN_THEMES: [&'static str; 9] = [
        "cometix",
        "default",
        "minimal",
        "gruvbox",
        "nord",
        "powerline-dark",
        "powerline-light",
        "powerline-rose-pine",
        "powerline-tokyo-night",
    ];

    /// Agents is opt-in in every theme and uses the theme's directory palette.
    pub fn agents_segment(theme_name: &str) -> crate::config::SegmentConfig {
        let mut segment = match theme_name {
            "cometix" => theme_cometix::directory_segment(),
            "gruvbox" => theme_gruvbox::directory_segment(),
            "minimal" => theme_minimal::directory_segment(),
            "nord" => theme_nord::directory_segment(),
            "powerline-dark" => theme_powerline_dark::directory_segment(),
            "powerline-light" => theme_powerline_light::directory_segment(),
            "powerline-rose-pine" => theme_powerline_rose_pine::directory_segment(),
            "powerline-tokyo-night" => theme_powerline_tokyo_night::directory_segment(),
            _ => theme_default::directory_segment(),
        };
        segment.id = crate::config::SegmentId::Agents;
        segment.enabled = false;
        segment.icon.plain = "&".into();
        segment.icon.nerd_font = "\u{f0c0}".into();
        segment.options.clear();
        segment.options.insert(
            "summary_ttl_seconds".into(),
            crate::agents::summary::DEFAULT_TTL_SECONDS.into(),
        );
        segment
    }

    /// Default Effort appearance, independent of the configured Model segment.
    pub fn effort_segment(theme_name: &str) -> crate::config::SegmentConfig {
        match theme_name {
            "cometix" => theme_cometix::effort_segment(),
            "gruvbox" => theme_gruvbox::effort_segment(),
            "minimal" => theme_minimal::effort_segment(),
            "nord" => theme_nord::effort_segment(),
            "powerline-dark" => theme_powerline_dark::effort_segment(),
            "powerline-light" => theme_powerline_light::effort_segment(),
            "powerline-rose-pine" => theme_powerline_rose_pine::effort_segment(),
            "powerline-tokyo-night" => theme_powerline_tokyo_night::effort_segment(),
            _ => theme_default::effort_segment(),
        }
    }

    /// A theme file in the themes directory takes priority over the built-in preset of the same
    /// name. An unreadable or invalid file is an error, as is a name with neither.
    pub fn get_theme(theme_name: &str) -> Result<Config, Box<dyn std::error::Error>> {
        if let Some(config) = Self::load_theme_from_file(theme_name)? {
            return Ok(config);
        }
        Self::builtin(theme_name).ok_or_else(|| format!("Unknown theme: {theme_name}").into())
    }

    pub fn builtin(theme_name: &str) -> Option<Config> {
        Some(match theme_name {
            "cometix" => Self::get_cometix(),
            "default" => Self::get_default(),
            "gruvbox" => Self::get_gruvbox(),
            "minimal" => Self::get_minimal(),
            "nord" => Self::get_nord(),
            "powerline-dark" => Self::get_powerline_dark(),
            "powerline-light" => Self::get_powerline_light(),
            "powerline-rose-pine" => Self::get_powerline_rose_pine(),
            "powerline-tokyo-night" => Self::get_powerline_tokyo_night(),
            _ => return None,
        })
    }

    /// Load a theme file; `None` when it does not exist.
    fn load_theme_from_file(
        theme_name: &str,
    ) -> Result<Option<Config>, Box<dyn std::error::Error>> {
        let theme_path = paths::themes_dir()?.join(format!("{}.toml", theme_name));
        let Some(content) = paths::read_optional(&theme_path)? else {
            return Ok(None);
        };
        let mut config: Config = toml::from_str(&content)
            .map_err(|error| format!("Invalid {}: {error}", theme_path.display()))?;

        // Ensure the theme field matches the requested theme
        config.theme = theme_name.to_string();

        Ok(Some(config))
    }

    /// Save current config as a new theme
    pub fn save_theme(theme_name: &str, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
        let themes_dir = paths::themes_dir()?;
        let theme_path = themes_dir.join(format!("{}.toml", theme_name));

        // Create themes directory if it doesn't exist
        std::fs::create_dir_all(&themes_dir)?;

        // Create a copy of config with the correct theme name
        let mut theme_config = config.clone();
        theme_config.theme = theme_name.to_string();

        let content = toml::to_string_pretty(&theme_config)?;
        std::fs::write(&theme_path, content)?;

        Ok(())
    }

    /// List all available themes (built-in + custom)
    pub fn list_available_themes() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut themes: Vec<String> = Self::BUILTIN_THEMES.map(String::from).to_vec();
        let themes_dir = paths::themes_dir()?;
        let entries = match std::fs::read_dir(&themes_dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(themes),
            Err(error) => {
                return Err(format!("Cannot read {}: {error}", themes_dir.display()).into())
            }
        };
        for entry in entries {
            let entry = entry?;
            if let Some(theme_name) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_suffix(".toml"))
            {
                if !themes.iter().any(|theme| theme == theme_name) {
                    themes.push(theme_name.to_string());
                }
            }
        }
        Ok(themes)
    }

    pub fn get_available_themes() -> Vec<(&'static str, &'static str)> {
        vec![
            ("cometix", "Cometix theme"),
            ("default", "Default theme with emoji icons"),
            ("minimal", "Minimal theme with reduced colors"),
            ("gruvbox", "Gruvbox color scheme"),
            ("nord", "Nord color scheme"),
            ("powerline-dark", "Dark powerline theme"),
            ("powerline-light", "Light powerline theme"),
            ("powerline-rose-pine", "Rose Pine powerline theme"),
            ("powerline-tokyo-night", "Tokyo Night powerline theme"),
        ]
    }

    pub fn get_cometix() -> Config {
        Config {
            style: StyleConfig {
                mode: StyleMode::NerdFont,
                separator: " | ".to_string(),
            },
            segments: vec![
                theme_cometix::model_segment(),
                theme_cometix::effort_segment(),
                theme_cometix::directory_segment(),
                theme_cometix::git_segment(),
                theme_cometix::context_window_segment(),
                theme_cometix::usage_segment(),
                theme_cometix::cost_segment(),
                theme_cometix::session_segment(),
                theme_cometix::output_style_segment(),
                Self::agents_segment("cometix"),
            ],
            theme: "cometix".to_string(),
        }
    }

    pub fn get_default() -> Config {
        Config {
            style: StyleConfig {
                mode: StyleMode::Plain,
                separator: " | ".to_string(),
            },
            segments: vec![
                theme_default::model_segment(),
                theme_default::effort_segment(),
                theme_default::directory_segment(),
                theme_default::git_segment(),
                theme_default::context_window_segment(),
                theme_default::usage_segment(),
                theme_default::cost_segment(),
                theme_default::session_segment(),
                theme_default::output_style_segment(),
                Self::agents_segment("default"),
            ],
            theme: "default".to_string(),
        }
    }

    pub fn get_minimal() -> Config {
        Config {
            style: StyleConfig {
                mode: StyleMode::Plain,
                separator: " │ ".to_string(),
            },
            segments: vec![
                theme_minimal::model_segment(),
                theme_minimal::effort_segment(),
                theme_minimal::directory_segment(),
                theme_minimal::git_segment(),
                theme_minimal::context_window_segment(),
                theme_minimal::usage_segment(),
                theme_minimal::cost_segment(),
                theme_minimal::session_segment(),
                theme_minimal::output_style_segment(),
                Self::agents_segment("minimal"),
            ],
            theme: "minimal".to_string(),
        }
    }

    pub fn get_gruvbox() -> Config {
        Config {
            style: StyleConfig {
                mode: StyleMode::NerdFont,
                separator: " | ".to_string(),
            },
            segments: vec![
                theme_gruvbox::model_segment(),
                theme_gruvbox::effort_segment(),
                theme_gruvbox::directory_segment(),
                theme_gruvbox::git_segment(),
                theme_gruvbox::context_window_segment(),
                theme_gruvbox::usage_segment(),
                theme_gruvbox::cost_segment(),
                theme_gruvbox::session_segment(),
                theme_gruvbox::output_style_segment(),
                Self::agents_segment("gruvbox"),
            ],
            theme: "gruvbox".to_string(),
        }
    }

    pub fn get_nord() -> Config {
        Config {
            style: StyleConfig {
                mode: StyleMode::NerdFont,
                separator: "".to_string(),
            },
            segments: vec![
                theme_nord::model_segment(),
                theme_nord::effort_segment(),
                theme_nord::directory_segment(),
                theme_nord::git_segment(),
                theme_nord::context_window_segment(),
                theme_nord::usage_segment(),
                theme_nord::cost_segment(),
                theme_nord::session_segment(),
                theme_nord::output_style_segment(),
                Self::agents_segment("nord"),
            ],
            theme: "nord".to_string(),
        }
    }

    pub fn get_powerline_dark() -> Config {
        Config {
            style: StyleConfig {
                mode: StyleMode::NerdFont,
                separator: "".to_string(),
            },
            segments: vec![
                theme_powerline_dark::model_segment(),
                theme_powerline_dark::effort_segment(),
                theme_powerline_dark::directory_segment(),
                theme_powerline_dark::git_segment(),
                theme_powerline_dark::context_window_segment(),
                theme_powerline_dark::usage_segment(),
                theme_powerline_dark::cost_segment(),
                theme_powerline_dark::session_segment(),
                theme_powerline_dark::output_style_segment(),
                Self::agents_segment("powerline-dark"),
            ],
            theme: "powerline-dark".to_string(),
        }
    }

    pub fn get_powerline_light() -> Config {
        Config {
            style: StyleConfig {
                mode: StyleMode::NerdFont,
                separator: "".to_string(),
            },
            segments: vec![
                theme_powerline_light::model_segment(),
                theme_powerline_light::effort_segment(),
                theme_powerline_light::directory_segment(),
                theme_powerline_light::git_segment(),
                theme_powerline_light::context_window_segment(),
                theme_powerline_light::usage_segment(),
                theme_powerline_light::cost_segment(),
                theme_powerline_light::session_segment(),
                theme_powerline_light::output_style_segment(),
                Self::agents_segment("powerline-light"),
            ],
            theme: "powerline-light".to_string(),
        }
    }

    pub fn get_powerline_rose_pine() -> Config {
        Config {
            style: StyleConfig {
                mode: StyleMode::NerdFont,
                separator: "".to_string(),
            },
            segments: vec![
                theme_powerline_rose_pine::model_segment(),
                theme_powerline_rose_pine::effort_segment(),
                theme_powerline_rose_pine::directory_segment(),
                theme_powerline_rose_pine::git_segment(),
                theme_powerline_rose_pine::context_window_segment(),
                theme_powerline_rose_pine::usage_segment(),
                theme_powerline_rose_pine::cost_segment(),
                theme_powerline_rose_pine::session_segment(),
                theme_powerline_rose_pine::output_style_segment(),
                Self::agents_segment("powerline-rose-pine"),
            ],
            theme: "powerline-rose-pine".to_string(),
        }
    }

    pub fn get_powerline_tokyo_night() -> Config {
        Config {
            style: StyleConfig {
                mode: StyleMode::NerdFont,
                separator: "".to_string(),
            },
            segments: vec![
                theme_powerline_tokyo_night::model_segment(),
                theme_powerline_tokyo_night::effort_segment(),
                theme_powerline_tokyo_night::directory_segment(),
                theme_powerline_tokyo_night::git_segment(),
                theme_powerline_tokyo_night::context_window_segment(),
                theme_powerline_tokyo_night::usage_segment(),
                theme_powerline_tokyo_night::cost_segment(),
                theme_powerline_tokyo_night::session_segment(),
                theme_powerline_tokyo_night::output_style_segment(),
                Self::agents_segment("powerline-tokyo-night"),
            ],
            theme: "powerline-tokyo-night".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SegmentId;

    #[test]
    fn every_builtin_theme_has_an_independent_effort_segment_after_model() {
        for config in [
            ThemePresets::get_cometix(),
            ThemePresets::get_default(),
            ThemePresets::get_minimal(),
            ThemePresets::get_gruvbox(),
            ThemePresets::get_nord(),
            ThemePresets::get_powerline_dark(),
            ThemePresets::get_powerline_light(),
            ThemePresets::get_powerline_rose_pine(),
            ThemePresets::get_powerline_tokyo_night(),
        ] {
            config.check().unwrap();
            let agents: Vec<_> = config
                .segments
                .iter()
                .filter(|s| s.id == SegmentId::Agents)
                .collect();
            assert_eq!(agents.len(), 1, "{}", config.theme);
            assert!(!agents[0].enabled, "{}", config.theme);
            let mut restored: Config =
                toml::from_str(&toml::to_string_pretty(&config).unwrap()).unwrap();
            let model = restored
                .segments
                .iter()
                .position(|s| s.id == SegmentId::Model)
                .unwrap();
            let model_config = restored.segments[model].clone();
            let effort = &mut restored.segments[model + 1];
            assert_eq!(effort.id, SegmentId::Effort, "{}", config.theme);
            assert!(effort.enabled);
            assert_eq!(effort.icon.plain, "⚡");
            assert_eq!(effort.icon.nerd_font, "\u{f0e7}");
            assert_ne!(effort.icon.plain, model_config.icon.plain);
            assert_ne!(effort.icon.nerd_font, model_config.icon.nerd_font);
            assert_ne!(effort.colors.icon, model_config.colors.icon);
            assert_ne!(effort.colors.text, model_config.colors.text);
            if model_config.colors.background.is_some() {
                assert_ne!(effort.colors.background, model_config.colors.background);
            }
            assert_eq!(
                serde_json::to_value(&*effort).unwrap(),
                serde_json::to_value(ThemePresets::effort_segment(&config.theme)).unwrap()
            );
            effort.enabled = false;
            assert!(restored.segments[model].enabled);
        }
    }
}
