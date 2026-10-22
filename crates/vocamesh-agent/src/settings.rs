use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io::Write, path::PathBuf};

const SETTINGS_FILE: &str = "runtime-settings.json";
const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeSettings {
    pub schema_version: u32,
    #[serde(default)]
    pub manual_paths: BTreeMap<String, String>,
    #[serde(default)]
    pub default_tool_id: Option<String>,
}

impl RuntimeSettings {
    fn fresh() -> Self {
        Self::default()
    }
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            manual_paths: BTreeMap::new(),
            default_tool_id: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SettingsStore {
    config_dir: PathBuf,
}

impl SettingsStore {
    pub fn new(config_dir: impl Into<PathBuf>) -> Self {
        Self {
            config_dir: config_dir.into(),
        }
    }

    pub fn config_path(&self) -> PathBuf {
        self.config_dir.join(SETTINGS_FILE)
    }

    pub fn load(&self) -> Result<RuntimeSettings, SettingsError> {
        let contents = match fs::read(self.config_path()) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(RuntimeSettings::fresh())
            }
            Err(_) => return Err(SettingsError::Read),
        };
        let settings: RuntimeSettings =
            serde_json::from_slice(&contents).map_err(|_| SettingsError::Corrupt)?;
        if settings.schema_version != SCHEMA_VERSION {
            return Err(SettingsError::UnsupportedSchema);
        }
        Ok(settings)
    }

    pub fn save(&self, settings: &RuntimeSettings) -> Result<(), SettingsError> {
        fs::create_dir_all(&self.config_dir).map_err(|_| SettingsError::Write)?;
        let mut temporary =
            tempfile::NamedTempFile::new_in(&self.config_dir).map_err(|_| SettingsError::Write)?;
        serde_json::to_writer(&mut temporary, settings).map_err(|_| SettingsError::Write)?;
        temporary
            .write_all(b"\n")
            .map_err(|_| SettingsError::Write)?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|_| SettingsError::Write)?;
        temporary
            .persist(self.config_path())
            .map_err(|_| SettingsError::Write)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsError {
    Read,
    Corrupt,
    UnsupportedSchema,
    Write,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_config_loads_defaults_and_write_replaces_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(dir.path());
        assert_eq!(store.load().unwrap(), RuntimeSettings::fresh());
        let mut settings = RuntimeSettings::fresh();
        settings
            .manual_paths
            .insert("claude".into(), "/safe/claude".into());
        store.save(&settings).unwrap();
        settings
            .manual_paths
            .insert("codex".into(), "/safe/codex".into());
        store.save(&settings).unwrap();
        assert_eq!(store.load().unwrap(), settings);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn malformed_and_unknown_schema_are_recoverable_errors() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(dir.path());
        fs::write(store.config_path(), b"not json").unwrap();
        assert_eq!(store.load(), Err(SettingsError::Corrupt));
        fs::write(store.config_path(), br#"{"schemaVersion":99}"#).unwrap();
        assert_eq!(store.load(), Err(SettingsError::UnsupportedSchema));
    }

    #[test]
    fn unknown_fields_are_rejected_instead_of_retaining_unapproved_settings() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(dir.path());
        fs::write(
            store.config_path(),
            br#"{"schemaVersion":1,"manualPaths":{},"defaultToolId":null,"apiKey":"must-not-be-read"}"#,
        )
        .unwrap();
        assert_eq!(store.load(), Err(SettingsError::Corrupt));
    }

    #[test]
    fn unwriteable_settings_location_returns_a_stable_error() {
        let dir = tempfile::tempdir().unwrap();
        let not_a_directory = dir.path().join("file");
        fs::write(&not_a_directory, b"file").unwrap();
        assert_eq!(
            SettingsStore::new(not_a_directory).save(&RuntimeSettings::default()),
            Err(SettingsError::Write)
        );
    }
}
