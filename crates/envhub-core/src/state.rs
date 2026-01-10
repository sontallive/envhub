use std::fs;
use std::path::{Path, PathBuf};

use dirs::config_dir;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{CoreError, ErrorCode};

pub type EnvProfile = IndexMap<String, String>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct State {
    #[serde(default)]
    pub apps: IndexMap<String, AppConfig>,
    #[serde(flatten)]
    pub extra: IndexMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub installed: bool,
    #[serde(default)]
    pub target_binary: String,
    #[serde(default)]
    pub install_path: Option<String>,
    #[serde(default)]
    pub active_profile: Option<String>,
    #[serde(default)]
    pub profiles: IndexMap<String, EnvProfile>,
    #[serde(flatten)]
    pub extra: IndexMap<String, serde_json::Value>,
}

pub fn default_state_path() -> Result<PathBuf, CoreError> {
    let base = config_dir().ok_or_else(|| {
        CoreError::new(
            ErrorCode::InstallPath,
            "Failed to resolve config directory".to_string(),
        )
    })?;
    let envhub_dir = if cfg!(windows) { "EnvHub" } else { "envhub" };
    Ok(base.join(envhub_dir).join("state.json"))
}

pub fn load_state() -> Result<State, CoreError> {
    let path = default_state_path()?;
    load_state_from_path(&path)
}

pub fn load_state_from_path(path: &Path) -> Result<State, CoreError> {
    if !path.exists() {
        return Ok(State::default());
    }
    let data = fs::read_to_string(path).map_err(|err| {
        CoreError::new(
            ErrorCode::Io,
            format!("Failed to read state.json: {err}"),
        )
    })?;
    serde_json::from_str(&data).map_err(|err| {
        CoreError::new(
            ErrorCode::Json,
            format!("Failed to parse state.json: {err}"),
        )
    })
}

pub fn save_state(state: &State) -> Result<(), CoreError> {
    let path = default_state_path()?;
    save_state_to_path(&path, state)
}

pub fn save_state_to_path(path: &Path, state: &State) -> Result<(), CoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            CoreError::new(
                ErrorCode::Io,
                format!("Failed to create state.json directory: {err}"),
            )
        })?;
    }
    let data = serde_json::to_vec_pretty(state).map_err(|err| {
        CoreError::new(
            ErrorCode::Json,
            format!("Failed to serialize state.json: {err}"),
        )
    })?;
    fs::write(path, data).map_err(|err| {
        CoreError::new(
            ErrorCode::Io,
            format!("Failed to write state.json: {err}"),
        )
    })
}

pub fn validate_state(state: &mut State) -> Result<(), CoreError> {
    for (name, app) in state.apps.iter_mut() {
        if app.target_binary.trim().is_empty() {
            return Err(CoreError::new(
                ErrorCode::InvalidState,
                format!("App \"{name}\" is missing target_binary"),
            ));
        }

        if app.profiles.is_empty() {
            app.profiles.insert("default".to_string(), EnvProfile::new());
        }

        let active = app.active_profile.clone();
        let resolved = active
            .filter(|profile| app.profiles.contains_key(profile))
            .or_else(|| app.profiles.keys().next().cloned());
        app.active_profile = resolved;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn validate_state_fills_missing_profiles_and_active() {
        let mut state = State::default();
        state.apps.insert(
            "tool".to_string(),
            AppConfig {
                target_binary: "tool-bin".to_string(),
                ..AppConfig::default()
            },
        );

        validate_state(&mut state).expect("validate_state should succeed");
        let app = state.apps.get("tool").expect("app exists");
        assert!(app.profiles.contains_key("default"));
        assert_eq!(app.active_profile.as_deref(), Some("default"));
    }

    #[test]
    fn save_and_load_preserves_unknown_fields() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("state.json");
        let raw = r#"
        {
          "apps": {},
          "future": { "flag": true }
        }
        "#;
        fs::write(&path, raw).expect("write state");

        let mut state = load_state_from_path(&path).expect("load");
        validate_state(&mut state).expect("validate");
        save_state_to_path(&path, &state).expect("save");

        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("parse");
        assert!(value.get("future").is_some());
    }
}
