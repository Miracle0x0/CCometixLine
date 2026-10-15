//! Cleanup of `models.toml` fields that earlier versions required and that Claude Code's input
//! now provides. It runs only from explicit initialization, never while rendering.

use super::models::ModelConfig;
use super::paths::{self, Result};
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, Item};

/// Fields removed from every `[[models]]` and `[[context_modifiers]]` entry.
const OBSOLETE_FIELDS: [&str; 1] = ["context_limit"];
const SECTIONS: [&str; 2] = ["models", "context_modifiers"];

#[derive(Debug)]
pub struct ModelsMigration {
    pub path: PathBuf,
    pub backup: PathBuf,
    pub removed: usize,
}

impl ModelsMigration {
    pub fn message(&self) -> String {
        format!(
            "Removed {} obsolete {} field(s) from {}; the original is saved as {}",
            self.removed,
            OBSOLETE_FIELDS.join("/"),
            self.path.display(),
            self.backup.display()
        )
    }
}

/// Remove the obsolete fields from each entry, keeping everything else, comments included.
fn strip_obsolete(document: &mut DocumentMut) -> usize {
    let mut removed = 0;
    for section in SECTIONS {
        match document.get_mut(section) {
            // `[[models]]` entries
            Some(Item::ArrayOfTables(tables)) => {
                for table in tables.iter_mut() {
                    for field in OBSOLETE_FIELDS {
                        removed += usize::from(table.remove(field).is_some());
                    }
                }
            }
            // `models = [{ ... }]` entries
            Some(Item::Value(value)) => {
                for entry in value
                    .as_array_mut()
                    .into_iter()
                    .flat_map(|array| array.iter_mut())
                {
                    if let Some(table) = entry.as_inline_table_mut() {
                        for field in OBSOLETE_FIELDS {
                            removed += usize::from(table.remove(field).is_some());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    removed
}

/// Whether `content` contains fields that [`migrate_models_file`] would remove.
pub fn has_obsolete_fields(content: &str) -> bool {
    content
        .parse::<DocumentMut>()
        .is_ok_and(|mut document| strip_obsolete(&mut document) > 0)
}

/// Remove obsolete fields from `path` in place after saving the original next to it as
/// `models.toml.<timestamp>.bak`. Nothing is written when there is nothing to remove, or when
/// the cleaned file would still be invalid; the latter is an error naming the remaining problem.
pub fn migrate_models_file(path: &Path, timestamp: &str) -> Result<Option<ModelsMigration>> {
    let Some(content) = paths::read_optional(path)? else {
        return Ok(None);
    };
    let mut document: DocumentMut = content
        .parse()
        .map_err(|error| format!("Invalid {}: {error}", path.display()))?;
    let removed = strip_obsolete(&mut document);
    if removed == 0 {
        return Ok(None);
    }
    let migrated = document.to_string();
    toml::from_str::<ModelConfig>(&migrated).map_err(|error| {
        format!(
            "Cannot clean up {}: it is still invalid after removing obsolete fields: {error}",
            path.display()
        )
    })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("Invalid file name: {}", path.display()))?;
    let backup = path.with_file_name(format!("{file_name}.{timestamp}.bak"));
    std::fs::write(&backup, &content)
        .map_err(|error| format!("Cannot write backup {}: {error}", backup.display()))?;
    std::fs::write(path, migrated)
        .map_err(|error| format!("Cannot write {}: {error}", path.display()))?;
    Ok(Some(ModelsMigration {
        path: path.to_path_buf(),
        backup,
        removed,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestDir;
    use std::fs;

    const LEGACY: &str = r#"# My models
[[models]]
pattern = "glm-5.2"   # GLM
display_name = "GLM 5.2"
context_limit = 1000000

[[models]]
pattern = "kimi"
display_name = "Kimi"

# 1M suffix
[[context_modifiers]]
pattern = "[1m]"
display_suffix = " 1M"
context_limit = 1000000
"#;

    #[test]
    fn removes_only_obsolete_fields_and_keeps_comments_with_a_backup() {
        let dir = TestDir::new();
        let path = dir.0.join("models.toml");
        fs::write(&path, LEGACY).unwrap();
        assert!(toml::from_str::<ModelConfig>(LEGACY).is_err());
        assert!(has_obsolete_fields(LEGACY));

        let migration = migrate_models_file(&path, "20261014120000")
            .unwrap()
            .unwrap();
        assert_eq!(migration.removed, 2);
        assert_eq!(
            migration.backup,
            dir.0.join("models.toml.20261014120000.bak")
        );
        assert_eq!(fs::read_to_string(&migration.backup).unwrap(), LEGACY);
        assert!(migration
            .message()
            .contains("Removed 2 obsolete context_limit"));

        let migrated = fs::read_to_string(&path).unwrap();
        assert_eq!(
            migrated,
            LEGACY.replace("context_limit = 1000000\n", ""),
            "{migrated}"
        );
        let config: ModelConfig = toml::from_str(&migrated).unwrap();
        assert_eq!(config.model_entries.len(), 2);
        assert_eq!(config.context_modifiers[0].display_suffix, " 1M");
        assert!(!has_obsolete_fields(&migrated));

        // A clean file is left untouched and gets no new backup.
        assert!(migrate_models_file(&path, "20261014120001")
            .unwrap()
            .is_none());
        assert!(!dir.0.join("models.toml.20261014120001.bak").exists());
    }

    #[test]
    fn inline_entries_are_cleaned_too() {
        let dir = TestDir::new();
        let path = dir.0.join("models.toml");
        fs::write(
            &path,
            "models = [{ pattern = \"x\", display_name = \"X\", context_limit = 128000 }]\n",
        )
        .unwrap();
        let migration = migrate_models_file(&path, "t").unwrap().unwrap();
        assert_eq!(migration.removed, 1);
        let config: ModelConfig = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(config.model_entries[0].display_name, "X");
    }

    #[test]
    fn missing_or_still_invalid_files_are_not_written() {
        let dir = TestDir::new();
        let path = dir.0.join("models.toml");
        assert!(migrate_models_file(&path, "t").unwrap().is_none());
        assert!(!path.exists());

        let other_problem = "[[models]]\npattern = \"x\"\ncontext_limit = 1\n";
        fs::write(&path, other_problem).unwrap();
        let error = migrate_models_file(&path, "t").unwrap_err().to_string();
        assert!(error.contains("display_name"), "{error}");
        assert_eq!(fs::read_to_string(&path).unwrap(), other_problem);
        assert!(!dir.0.join("models.toml.t.bak").exists());

        fs::write(&path, "not toml = [").unwrap();
        assert!(migrate_models_file(&path, "t").is_err());
        assert!(!has_obsolete_fields("not toml = ["));
    }
}
