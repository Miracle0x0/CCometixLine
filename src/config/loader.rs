use super::migration::migrate_models_file;
use super::models::ModelConfig;
use super::paths;
use super::types::Config;
use crate::ui::themes::ThemePresets;
use std::fs;
use std::path::PathBuf;

/// Result of config initialization
#[derive(Debug)]
pub enum InitResult {
    /// Config was created at the given path
    Created(PathBuf),
    /// Config already existed at the given path
    AlreadyExists(PathBuf),
}

pub struct ConfigLoader;

impl ConfigLoader {
    /// Create the built-in theme files and the `models.toml` template where they are missing, and
    /// remove obsolete fields from an existing `models.toml`, returning a note about that cleanup.
    /// Only explicit initialization calls this; rendering never writes configuration files.
    pub fn init_files() -> Result<Option<String>, Box<dyn std::error::Error>> {
        let themes_dir = paths::themes_dir()?;
        fs::create_dir_all(&themes_dir)?;
        for theme_name in ThemePresets::BUILTIN_THEMES {
            let theme_path = themes_dir.join(format!("{theme_name}.toml"));
            if !theme_path.exists() {
                let theme_config = ThemePresets::builtin(theme_name)
                    .ok_or_else(|| format!("Missing built-in theme {theme_name}"))?;
                fs::write(&theme_path, toml::to_string_pretty(&theme_config)?)?;
            }
        }
        let models_path = paths::models_path()?;
        if !models_path.exists() {
            ModelConfig::create_default_file(&models_path)?;
            return Ok(None);
        }
        let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
        Ok(migrate_models_file(&models_path, &timestamp)?.map(|migration| migration.message()))
    }
}

impl Config {
    /// Load the configuration; a missing file means the default theme, any other failure is an error.
    pub fn load() -> Result<Config, Box<dyn std::error::Error>> {
        let config_path = paths::config_path()?;
        match paths::read_optional(&config_path)? {
            Some(content) => toml::from_str(&content)
                .map_err(|error| format!("Invalid {}: {error}", config_path.display()).into()),
            None => Ok(Config::default()),
        }
    }

    /// Save configuration to default location
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = paths::config_path()?;
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(config_path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Initialize config directory and create default config, with any note from
    /// [`ConfigLoader::init_files`].
    pub fn init() -> Result<(InitResult, Option<String>), Box<dyn std::error::Error>> {
        let config_path = paths::config_path()?;
        let note = ConfigLoader::init_files()?;
        let result = if config_path.exists() {
            InitResult::AlreadyExists(config_path)
        } else {
            Config::default().save()?;
            InitResult::Created(config_path)
        };
        Ok((result, note))
    }

    /// Validate configuration
    pub fn check(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Basic validation
        if self.segments.is_empty() {
            return Err("No segments configured".into());
        }

        // Validate segment IDs are unique
        let mut seen_ids = std::collections::HashSet::new();
        for segment in &self.segments {
            if !seen_ids.insert(segment.id) {
                return Err(format!("Duplicate segment ID: {:?}", segment.id).into());
            }
        }

        Ok(())
    }

    /// Print configuration as TOML
    pub fn print(&self) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        println!("{}", content);
        Ok(())
    }
}
