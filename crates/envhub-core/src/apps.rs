use std::path::Path;

use crate::{load_state_from_path, save_state_to_path, CoreError, ErrorCode};

pub fn register_app(name: &str, target: &str) -> Result<(), CoreError> {
    let path = crate::default_state_path()?;
    register_app_in(&path, name, target)
}

pub fn register_app_in(path: &Path, name: &str, target: &str) -> Result<(), CoreError> {
    if name.trim().is_empty() || target.trim().is_empty() {
        return Err(CoreError::new(
            ErrorCode::InvalidState,
            "App name and target must be non-empty".to_string(),
        ));
    }
    let mut state = load_state_from_path(path)?;
    let app = state.apps.entry(name.to_string()).or_default();
    app.target_binary = target.to_string();
    if app.active_profile.is_none() {
        app.active_profile = Some("default".to_string());
    }
    if app.profiles.is_empty() {
        app.profiles.insert("default".to_string(), Default::default());
    }
    app.installed = false;
    crate::validate_state(&mut state)?;
    save_state_to_path(path, &state)
}

pub fn set_active_profile(name: &str, profile: &str) -> Result<(), CoreError> {
    let path = crate::default_state_path()?;
    set_active_profile_in(&path, name, profile)
}

pub fn set_active_profile_in(path: &Path, name: &str, profile: &str) -> Result<(), CoreError> {
    let mut state = load_state_from_path(path)?;
    let app = state.apps.get_mut(name).ok_or_else(|| {
        CoreError::new(
            ErrorCode::AppNotFound,
            format!("App \"{name}\" is not registered"),
        )
    })?;
    if !app.profiles.contains_key(profile) {
        return Err(CoreError::new(
            ErrorCode::ProfileNotFound,
            format!("Profile \"{profile}\" not found for app \"{name}\""),
        ));
    }
    app.active_profile = Some(profile.to_string());
    save_state_to_path(path, &state)
}

pub fn list_apps() -> Result<Vec<String>, CoreError> {
    let path = crate::default_state_path()?;
    list_apps_in(&path)
}

pub fn list_apps_in(path: &Path) -> Result<Vec<String>, CoreError> {
    let state = load_state_from_path(path)?;
    Ok(state.apps.keys().cloned().collect())
}

pub fn list_profiles(name: &str) -> Result<Vec<String>, CoreError> {
    let path = crate::default_state_path()?;
    list_profiles_in(&path, name)
}

pub fn list_profiles_in(path: &Path, name: &str) -> Result<Vec<String>, CoreError> {
    let state = load_state_from_path(path)?;
    let app = state.apps.get(name).ok_or_else(|| {
        CoreError::new(
            ErrorCode::AppNotFound,
            format!("App \"{name}\" is not registered"),
        )
    })?;
    Ok(app.profiles.keys().cloned().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn register_app_creates_default_profile() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("state.json");

        register_app_in(&path, "tool", "tool-bin").expect("register");
        let state = load_state_from_path(&path).expect("load");
        let app = state.apps.get("tool").expect("app exists");
        assert_eq!(app.target_binary, "tool-bin");
        assert!(app.profiles.contains_key("default"));
        assert_eq!(app.active_profile.as_deref(), Some("default"));
    }

    #[test]
    fn set_active_profile_requires_existing_profile() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("state.json");
        register_app_in(&path, "tool", "tool-bin").expect("register");

        let err = set_active_profile_in(&path, "tool", "missing").unwrap_err();
        assert_eq!(err.code, ErrorCode::ProfileNotFound);
    }
}
