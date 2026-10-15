use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Update status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    /// New version available
    Ready {
        version: String,
        found_at: DateTime<Utc>,
    },
    Failed {
        error: String,
    },
}

/// Update state persistence structure
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct UpdateState {
    pub status: UpdateStatus,
    pub last_check: Option<DateTime<Utc>>,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_pid: Option<u32>,
}

impl UpdateState {
    /// Get status bar display text
    pub fn status_text(&self) -> Option<String> {
        match &self.status {
            UpdateStatus::Ready { version, .. } => Some(format!("\u{f06b0} v{}", version)),
            _ => None,
        }
    }

    /// Load update state from config directory and trigger auto-check if needed
    pub fn load(state_file: &std::path::Path) -> Self {
        let mut state = if let Ok(content) = std::fs::read_to_string(state_file) {
            serde_json::from_str::<UpdateState>(&content).unwrap_or_else(|_| UpdateState {
                current_version: env!("CARGO_PKG_VERSION").to_string(),
                ..Default::default()
            })
        } else {
            UpdateState {
                current_version: env!("CARGO_PKG_VERSION").to_string(),
                ..Default::default()
            }
        };

        if state.should_check_update() {
            let should_start_check = if let Some(pid) = state.update_pid {
                !Self::is_process_running(pid)
            } else {
                true
            };

            if should_start_check {
                state.update_pid = Some(std::process::id());
                state.last_check = Some(Utc::now());
                let _ = state.save(state_file);

                match github::check_for_updates() {
                    Ok(Some(version)) => {
                        state.status = UpdateStatus::Ready {
                            version: version.clone(),
                            found_at: Utc::now(),
                        };
                        state.latest_version = Some(version);
                    }
                    Ok(None) => {
                        state.status = UpdateStatus::Idle;
                    }
                    Err(error) => {
                        state.status = UpdateStatus::Failed {
                            error: error.to_string(),
                        };
                    }
                }

                state.update_pid = None;
                let _ = state.save(state_file);
            }
        }

        state
    }

    fn is_process_running(pid: u32) -> bool {
        #[cfg(unix)]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("ps").arg("-p").arg(pid.to_string()).output() {
                output.status.success()
            } else {
                false
            }
        }

        #[cfg(windows)]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("tasklist")
                .arg("/FI")
                .arg(&format!("PID eq {}", pid))
                .output()
            {
                String::from_utf8_lossy(&output.stdout).contains(&pid.to_string())
            } else {
                false
            }
        }

        #[cfg(not(any(unix, windows)))]
        false
    }

    pub fn save(&self, state_file: &std::path::Path) -> Result<(), std::io::Error> {
        if let Some(config_dir) = state_file.parent() {
            std::fs::create_dir_all(config_dir)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(state_file, content)?;

        Ok(())
    }

    /// Check interval: 1 hour
    fn should_check_update(&self) -> bool {
        if matches!(self.status, UpdateStatus::Checking) {
            return false;
        }

        if let Some(last_check) = self.last_check {
            Utc::now().signed_duration_since(last_check).num_hours() >= 1
        } else {
            true
        }
    }
}

/// GitHub Release version check
mod github {
    #[derive(serde::Deserialize)]
    struct Release {
        tag_name: String,
    }

    /// Check the latest release of this repository.
    pub fn check_for_updates() -> Result<Option<String>, Box<dyn std::error::Error>> {
        let url = "https://api.github.com/repos/Miracle0x0/CCometixLine/releases/latest";

        let response = ureq::get(url)
            .header("Accept", "application/vnd.github+json")
            .header(
                "User-Agent",
                concat!("CCometixLine/", env!("CARGO_PKG_VERSION")),
            )
            .call()?;

        let release: Release = response.into_body().read_json()?;
        newer_version(&release.tag_name, env!("CARGO_PKG_VERSION"))
    }

    fn newer_version(
        tag: &str,
        current: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let latest = tag
            .strip_prefix('v')
            .ok_or("Release tag must start with v")?;
        let current_ver = semver::Version::parse(current)?;
        let latest_ver = semver::Version::parse(latest)?;

        if latest_ver > current_ver {
            Ok(Some(latest.to_string()))
        } else {
            Ok(None)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{newer_version, Release};

        #[test]
        fn reads_github_release_tag() {
            let release: Release = serde_json::from_str(
                r#"{"tag_name":"v1.2.0","name":"Release 1.2.0","draft":false}"#,
            )
            .unwrap();
            assert_eq!(
                newer_version(&release.tag_name, "1.1.3").unwrap(),
                Some("1.2.0".to_string())
            );
        }

        #[test]
        fn compares_versions_semantically() {
            assert_eq!(
                newer_version("v1.1.10", "1.1.9").unwrap(),
                Some("1.1.10".to_string())
            );
            for tag in ["v1.1.3", "v1.1.2", "v1.1.3-rc.1"] {
                assert_eq!(newer_version(tag, "1.1.3").unwrap(), None);
            }
        }

        #[test]
        fn rejects_invalid_release_data() {
            assert!(serde_json::from_str::<Release>(r#"{"version":"1.2.0"}"#).is_err());
            for tag in ["1.2.0", "vnot-a-version", ""] {
                assert!(newer_version(tag, "1.1.3").is_err());
            }
        }
    }
}
