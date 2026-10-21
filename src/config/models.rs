use super::paths;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

/// Model display names. Context window sizes come from Claude Code's `context_window` input,
/// so this file only customizes how model IDs are shown.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelConfig {
    #[serde(rename = "models", default)]
    pub model_entries: Vec<ModelEntry>,
    #[serde(default)]
    pub context_modifiers: Vec<ContextModifier>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelEntry {
    pub pattern: String,
    pub display_name: String,
}

/// Appends a suffix to display names when the model ID contains `pattern`.
/// For example, `[1m]` in a model ID selects the 1M context variant, regardless of the base model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextModifier {
    pub pattern: String,
    pub display_suffix: String,
}

/// Built-in Claude model family definition (internal, not serialized).
/// Uses regex with named capture groups to auto-extract version numbers from model IDs.
///
/// Handles both naming conventions:
///   - `claude-{variant}-{major}[-{minor}]-{date}` (e.g., `claude-opus-4-6-20250901`)
///   - `claude-{major}[-{minor}]-{variant}-{date}` (e.g., `claude-4-opus-20250514`)
struct BuiltinModelFamily {
    regex: Regex,
    display_prefix: String,
}

impl BuiltinModelFamily {
    /// Create a new model family with auto-generated regex pattern.
    ///
    /// The regex matches version numbers (1-2 digits) adjacent to the keyword,
    /// followed by a boundary that signals "version numbers have ended":
    ///   - `-\d{3,}` : date suffix (e.g., `-20250514`)
    ///   - `-[a-z]`  : text qualifier (e.g., `-thinking`, `-preview`, `-latest`)
    ///   - `\[`      : context modifier (e.g., `[1m]`)
    ///   - `$`       : end of string
    ///
    /// The boundary is consumed but only named capture groups are used for version extraction.
    /// Rust's `regex` crate does not support lookahead, so the NFA engine's natural
    /// backtracking prevents date digits from being captured as minor version numbers.
    fn new(keyword: &str, display_prefix: &str) -> Self {
        let pattern = format!(
            r"(?:(?P<pre_major>\d{{1,2}})(?:-(?P<pre_minor>\d{{1,2}}))?-{kw}|{kw}-(?P<post_major>\d{{1,2}})(?:-(?P<post_minor>\d{{1,2}}))?)(?:-\d{{3,}}|-[a-z]|\[|$)",
            kw = keyword
        );
        Self {
            regex: Regex::new(&pattern).expect("built-in family regex should compile"),
            display_prefix: display_prefix.to_string(),
        }
    }

    /// Try to match a model ID (already lowercased) and extract a formatted display name.
    /// Returns `None` if the model ID doesn't match this family.
    fn match_model(&self, model_id_lower: &str) -> Option<String> {
        let caps = self.regex.captures(model_id_lower)?;

        let major = caps
            .name("post_major")
            .or_else(|| caps.name("pre_major"))
            .map(|m| m.as_str())?;

        let minor = caps
            .name("post_minor")
            .or_else(|| caps.name("pre_minor"))
            .map(|m| m.as_str());

        let version = match minor {
            Some(m) => format!("{}.{}", major, m),
            None => major.to_string(),
        };

        Some(format!("{} {}", self.display_prefix, version))
    }
}

/// Lazily-initialized built-in Claude model families.
/// Regex compilation happens only once per process, regardless of how many times
/// `get_display_name()` is called.
static BUILTIN_FAMILIES: OnceLock<Vec<BuiltinModelFamily>> = OnceLock::new();

impl ModelConfig {
    /// Get built-in Claude model families (compiled once, cached via OnceLock).
    fn builtin_families() -> &'static [BuiltinModelFamily] {
        BUILTIN_FAMILIES.get_or_init(|| {
            vec![
                BuiltinModelFamily::new("sonnet", "Sonnet"),
                BuiltinModelFamily::new("opus", "Opus"),
                BuiltinModelFamily::new("haiku", "Haiku"),
                BuiltinModelFamily::new("fable", "Fable"),
            ]
        })
    }

    /// Try to match a model ID against built-in Claude model families.
    fn match_builtin_family(model_id: &str) -> Option<String> {
        let model_lower = model_id.to_lowercase();
        Self::builtin_families()
            .iter()
            .find_map(|family| family.match_model(&model_lower))
    }

    /// Load `models.toml` from ccline's configuration directory, with its entries taking
    /// priority over the built-in ones. A missing file means the built-in entries only; a file
    /// that cannot be read or parsed is an error.
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = paths::models_path()?;
        let Some(content) = paths::read_optional(&path)? else {
            return Ok(Self::default());
        };
        let user: ModelConfig = toml::from_str(&content).map_err(|error| {
            let hint = if super::migration::has_obsolete_fields(&content) {
                "\nRun `ccline --config`, or Initialize Config from the `ccline` menu, to remove \
                 obsolete fields automatically."
            } else {
                ""
            };
            format!("Invalid {}: {error}{hint}", path.display())
        })?;
        Ok(user.prepend_to(Self::default()))
    }

    fn prepend_to(mut self, builtin: Self) -> Self {
        self.model_entries.extend(builtin.model_entries);
        self.context_modifiers.extend(builtin.context_modifiers);
        self
    }

    /// Resolve a model ID in a single pass through all matching layers.
    /// Returns `(display_name, modifier_suffix)`.
    ///
    /// Matching priority for display_name:
    ///   1. User/built-in model entries (simple substring match)
    ///   2. Built-in Claude model families (regex with version extraction)
    ///   3. None (caller should use upstream fallback)
    fn resolve(&self, model_id: &str) -> (Option<String>, Option<String>) {
        let model_lower = model_id.to_lowercase();

        let base_name = self
            .model_entries
            .iter()
            .find(|e| model_lower.contains(&e.pattern.to_lowercase()))
            .map(|e| e.display_name.clone())
            .or_else(|| Self::match_builtin_family(model_id));

        let modifier_suffix = self
            .context_modifiers
            .iter()
            .find(|m| model_lower.contains(&m.pattern.to_lowercase()))
            .map(|m| m.display_suffix.clone());

        let display_name = base_name.map(|name| match &modifier_suffix {
            Some(suffix) => format!("{name}{suffix}"),
            None => name,
        });

        (display_name, modifier_suffix)
    }

    /// Get display name for a model using layered matching.
    /// Composes base name with any matching context modifier suffix.
    /// Returns None if nothing matches (caller should use upstream fallback display_name).
    pub fn get_display_name(&self, model_id: &str) -> Option<String> {
        self.resolve(model_id).0
    }

    /// Get the display suffix from any matching context modifier.
    /// Used to append modifier info (e.g., " 1M") to upstream fallback display names
    /// when the model itself is not recognized by our config.
    pub fn get_display_suffix(&self, model_id: &str) -> Option<String> {
        self.resolve(model_id).1
    }

    /// Create default model configuration file with minimal template
    pub fn create_default_file<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
        let template_content = "# CCometixLine Model Configuration\n\
             # This file customizes how model IDs are displayed.\n\
             # Context window sizes and usage come from Claude Code's status line input,\n\
             # so they are not configured here.\n\
             #\n\
             # Claude models are automatically recognized (Sonnet, Opus, Haiku, Fable) with\n\
             # version extraction. You only need to add entries here for overrides or\n\
             # third-party models.\n\
             \n\
             # Model display names (simple substring matching)\n\
             # These take priority over built-in Claude model recognition\n\
             \n\
             # Example:\n\
             # [[models]]\n\
             # pattern = \"my-model\"\n\
             # display_name = \"My Model\"\n\
             \n\
             # Context modifiers append a suffix to display names. They are matched\n\
             # independently, enabling composition:\n\
             #   model \"Opus 4\" + modifier \" 1M\" = \"Opus 4 1M\"\n\
             \n\
             # Example:\n\
             # [[context_modifiers]]\n\
             # pattern = \"[1m]\"\n\
             # display_suffix = \" 1M\"\n";

        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(path, template_content)?;
        Ok(())
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            // Only third-party models need explicit entries.
            // Claude models (Sonnet, Opus, Haiku, Fable) are handled by built-in regex families.
            model_entries: [
                ("glm-4.5", "GLM-4.5"),
                ("kimi-k2-turbo", "Kimi K2 Turbo"),
                ("kimi-k2", "Kimi K2"),
                ("qwen3-coder", "Qwen Coder"),
            ]
            .into_iter()
            .map(|(pattern, display_name)| ModelEntry {
                pattern: pattern.to_string(),
                display_name: display_name.to_string(),
            })
            .collect(),
            context_modifiers: vec![ContextModifier {
                pattern: "[1m]".to_string(),
                display_suffix: " 1M".to_string(),
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_compose_entries_families_and_modifiers() {
        let models = ModelConfig::default();
        assert_eq!(
            models.get_display_name("claude-opus-5-5[1m]").as_deref(),
            Some("Opus 5.5 1M")
        );
        assert_eq!(
            models.get_display_name("claude-sonnet-5-5").as_deref(),
            Some("Sonnet 5.5")
        );
        assert_eq!(
            models.get_display_name("kimi-k2-turbo").as_deref(),
            Some("Kimi K2 Turbo")
        );
        assert_eq!(models.get_display_name("unknown[1m]"), None);
        for (id, name) in [
            ("claude-fable-5", "Fable 5"),
            ("claude-fable-5-1", "Fable 5.1"),
            ("anthropic/claude-fable-5-1[1m]", "Fable 5.1 1M"),
            ("CLAUDE-FABLE-5-1-20260901", "Fable 5.1"),
        ] {
            assert_eq!(models.get_display_name(id).as_deref(), Some(name), "{id}");
        }
        assert_eq!(
            models.get_display_suffix("unknown[1m]").as_deref(),
            Some(" 1M")
        );
        let user: ModelConfig =
            toml::from_str("[[models]]\npattern = \"opus\"\ndisplay_name = \"Big\"\n").unwrap();
        let merged = user.prepend_to(ModelConfig::default());
        assert_eq!(
            merged.get_display_name("claude-opus-5-5").as_deref(),
            Some("Big")
        );
    }

    #[test]
    fn template_parses_and_removed_capacity_fields_are_rejected() {
        let dir = crate::test_support::TestDir::new();
        let path = dir.0.join("models.toml");
        ModelConfig::create_default_file(&path).unwrap();
        let template: ModelConfig = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(template.model_entries.is_empty());
        for legacy in [
            "[[models]]\npattern = \"x\"\ndisplay_name = \"X\"\ncontext_limit = 128000\n",
            "[[context_modifiers]]\npattern = \"[1m]\"\ndisplay_suffix = \" 1M\"\ncontext_limit = 1000000\n",
        ] {
            let error = toml::from_str::<ModelConfig>(legacy).unwrap_err();
            assert!(error.to_string().contains("context_limit"), "{error}");
        }
    }
}
