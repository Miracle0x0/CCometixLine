//! Every file ccline reads or writes lives under Claude Code's configuration directory:
//! `CLAUDE_CONFIG_DIR` when set, otherwise `~/.claude`
//! (https://code.claude.com/docs/en/env-vars).

use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const CONFIG_DIR_ENV: &str = "CLAUDE_CONFIG_DIR";

/// Claude Code's configuration directory for this process.
pub fn claude_dir() -> Result<PathBuf> {
    resolve_claude_dir(std::env::var_os(CONFIG_DIR_ENV), dirs::home_dir())
}

fn resolve_claude_dir(
    configured: Option<std::ffi::OsString>,
    home: Option<PathBuf>,
) -> Result<PathBuf> {
    match configured {
        Some(dir) => {
            let dir = PathBuf::from(dir);
            if dir.as_os_str().is_empty() {
                return Err(format!("{CONFIG_DIR_ENV} is set but empty").into());
            }
            if !dir.is_absolute() {
                return Err(format!(
                    "{CONFIG_DIR_ENV} must be an absolute path, not {}",
                    dir.display()
                )
                .into());
            }
            Ok(dir)
        }
        None => Ok(home
            .ok_or("Cannot locate the home directory for ~/.claude")?
            .join(".claude")),
    }
}

/// The host settings file ccline edits for the subagent panel.
pub fn settings_path(claude_dir: &Path) -> PathBuf {
    claude_dir.join("settings.json")
}

/// ccline's own files: config, themes, model names, update state and installation manifest.
pub fn ccline_dir(claude_dir: &Path) -> PathBuf {
    claude_dir.join("ccline")
}

pub fn config_path() -> Result<PathBuf> {
    Ok(ccline_dir(&claude_dir()?).join("config.toml"))
}

pub fn themes_dir() -> Result<PathBuf> {
    Ok(ccline_dir(&claude_dir()?).join("themes"))
}

pub fn models_path() -> Result<PathBuf> {
    Ok(ccline_dir(&claude_dir()?).join("models.toml"))
}

pub fn update_state_path() -> Result<PathBuf> {
    Ok(ccline_dir(&claude_dir()?).join(".update_state.json"))
}

/// Read a file that may legitimately be absent: `None` when it does not exist, an error naming
/// the path for any other failure.
pub fn read_optional(path: &Path) -> Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Cannot read {}: {error}", path.display()).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_directory_wins_and_must_be_absolute() {
        let home = Some(PathBuf::from("/home/user"));
        let custom = std::env::temp_dir().join("claude work");
        assert_eq!(
            resolve_claude_dir(Some(custom.clone().into()), home.clone()).unwrap(),
            custom
        );
        assert_eq!(
            resolve_claude_dir(None, home.clone()).unwrap(),
            PathBuf::from("/home/user/.claude")
        );
        for invalid in ["", "relative/dir", "~/.claude-work"] {
            assert!(resolve_claude_dir(Some(invalid.into()), home.clone()).is_err());
        }
        assert!(resolve_claude_dir(None, None).is_err());
    }
}
