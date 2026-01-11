use std::path::Path;

use crate::{CoreError, ErrorCode, load_state_from_path, save_state_to_path};

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
        app.profiles
            .insert("default".to_string(), Default::default());
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

pub fn add_profile(name: &str, profile: &str) -> Result<(), CoreError> {
    let path = crate::default_state_path()?;
    add_profile_in(&path, name, profile)
}

pub fn add_profile_in(path: &Path, name: &str, profile: &str) -> Result<(), CoreError> {
    if profile.trim().is_empty() {
        return Err(CoreError::new(
            ErrorCode::InvalidState,
            "Profile name must be non-empty".to_string(),
        ));
    }
    let mut state = load_state_from_path(path)?;
    let app = state.apps.get_mut(name).ok_or_else(|| {
        CoreError::new(
            ErrorCode::AppNotFound,
            format!("App \"{name}\" is not registered"),
        )
    })?;
    app.profiles.entry(profile.to_string()).or_default();
    if app.active_profile.is_none() {
        app.active_profile = Some(profile.to_string());
    }
    save_state_to_path(path, &state)
}

pub fn remove_profile(name: &str, profile: &str) -> Result<(), CoreError> {
    let path = crate::default_state_path()?;
    remove_profile_in(&path, name, profile)
}

pub fn remove_profile_in(path: &Path, name: &str, profile: &str) -> Result<(), CoreError> {
    let mut state = load_state_from_path(path)?;
    let app = state.apps.get_mut(name).ok_or_else(|| {
        CoreError::new(
            ErrorCode::AppNotFound,
            format!("App \"{name}\" is not registered"),
        )
    })?;
    if app.profiles.shift_remove(profile).is_none() {
        return Err(CoreError::new(
            ErrorCode::ProfileNotFound,
            format!("Profile \"{profile}\" not found for app \"{name}\""),
        ));
    }
    if app.active_profile.as_deref() == Some(profile) {
        app.active_profile = app.profiles.keys().next().cloned();
    }
    save_state_to_path(path, &state)
}

pub fn set_profile_env(name: &str, profile: &str, key: &str, value: &str) -> Result<(), CoreError> {
    let path = crate::default_state_path()?;
    set_profile_env_in(&path, name, profile, key, value)
}

pub fn set_profile_env_in(
    path: &Path,
    name: &str,
    profile: &str,
    key: &str,
    value: &str,
) -> Result<(), CoreError> {
    if key.trim().is_empty() {
        return Err(CoreError::new(
            ErrorCode::InvalidState,
            "Environment key must be non-empty".to_string(),
        ));
    }
    let mut state = load_state_from_path(path)?;
    let app = state.apps.get_mut(name).ok_or_else(|| {
        CoreError::new(
            ErrorCode::AppNotFound,
            format!("App \"{name}\" is not registered"),
        )
    })?;
    let profile_env = app.profiles.get_mut(profile).ok_or_else(|| {
        CoreError::new(
            ErrorCode::ProfileNotFound,
            format!("Profile \"{profile}\" not found for app \"{name}\""),
        )
    })?;
    profile_env.insert(key.to_string(), value.to_string());
    save_state_to_path(path, &state)
}

pub fn clone_profile(name: &str, from_profile: &str, to_profile: &str) -> Result<(), CoreError> {
    let path = crate::default_state_path()?;
    clone_profile_in(&path, name, from_profile, to_profile)
}

pub fn clone_profile_in(
    path: &Path,
    name: &str,
    from_profile: &str,
    to_profile: &str,
) -> Result<(), CoreError> {
    if to_profile.trim().is_empty() {
        return Err(CoreError::new(
            ErrorCode::InvalidState,
            "Target profile name must be non-empty".to_string(),
        ));
    }
    let mut state = load_state_from_path(path)?;
    let app = state.apps.get_mut(name).ok_or_else(|| {
        CoreError::new(
            ErrorCode::AppNotFound,
            format!("App \"{name}\" is not registered"),
        )
    })?;

    if !app.profiles.contains_key(from_profile) {
        return Err(CoreError::new(
            ErrorCode::ProfileNotFound,
            format!("Source profile \"{from_profile}\" not found for app \"{name}\""),
        ));
    }

    if app.profiles.contains_key(to_profile) {
        return Err(CoreError::new(
            ErrorCode::InvalidState,
            format!("Target profile \"{to_profile}\" already exists"),
        ));
    }

    let source_env = app.profiles.get(from_profile).unwrap().clone();
    app.profiles.insert(to_profile.to_string(), source_env);

    if app.active_profile.is_none() {
        app.active_profile = Some(to_profile.to_string());
    }

    save_state_to_path(path, &state)
}

pub fn remove_profile_env(name: &str, profile: &str, key: &str) -> Result<(), CoreError> {
    let path = crate::default_state_path()?;
    remove_profile_env_in(&path, name, profile, key)
}

pub fn remove_profile_env_in(
    path: &Path,
    name: &str,
    profile: &str,
    key: &str,
) -> Result<(), CoreError> {
    let mut state = load_state_from_path(path)?;
    let app = state.apps.get_mut(name).ok_or_else(|| {
        CoreError::new(
            ErrorCode::AppNotFound,
            format!("App \"{name}\" is not registered"),
        )
    })?;
    let profile_env = app.profiles.get_mut(profile).ok_or_else(|| {
        CoreError::new(
            ErrorCode::ProfileNotFound,
            format!("Profile \"{profile}\" not found for app \"{name}\""),
        )
    })?;
    if profile_env.shift_remove(key).is_none() {
        return Err(CoreError::new(
            ErrorCode::InvalidState,
            format!("Environment key \"{key}\" not found in profile \"{profile}\""),
        ));
    }
    save_state_to_path(path, &state)
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

    #[test]
    fn add_and_remove_profile_updates_active() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("state.json");
        register_app_in(&path, "tool", "tool-bin").expect("register");

        add_profile_in(&path, "tool", "work").expect("add");
        set_active_profile_in(&path, "tool", "work").expect("active");
        remove_profile_in(&path, "tool", "work").expect("remove");

        let state = load_state_from_path(&path).expect("load");
        let app = state.apps.get("tool").expect("app");
        assert_ne!(app.active_profile.as_deref(), Some("work"));
    }

    #[test]
    fn set_and_remove_profile_env() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("state.json");
        register_app_in(&path, "tool", "tool-bin").expect("register");

        set_profile_env_in(&path, "tool", "default", "KEY", "VALUE").expect("set");
        let state = load_state_from_path(&path).expect("load");
        let app = state.apps.get("tool").expect("app");
        assert_eq!(
            app.profiles
                .get("default")
                .and_then(|env| env.get("KEY").map(String::as_str)),
            Some("VALUE")
        );

        remove_profile_env_in(&path, "tool", "default", "KEY").expect("remove");
        let state = load_state_from_path(&path).expect("load");
        let app = state.apps.get("tool").expect("app");
        assert!(
            app.profiles
                .get("default")
                .and_then(|env| env.get("KEY"))
                .is_none()
        );
    }
}
