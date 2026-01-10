use std::fs;
use std::path::{Path, PathBuf};

use crate::{default_state_path, load_state_from_path, CoreError, ErrorCode, State};

#[cfg(test)]
use crate::AppConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallMode {
    Global,
    User,
}

#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub is_windows: bool,
    pub install_dir: PathBuf,
}

pub fn detect_platform(mode: InstallMode) -> Result<PlatformInfo, CoreError> {
    if cfg!(windows) {
        let base = std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
            CoreError::new(
                ErrorCode::InstallPath,
                "LOCALAPPDATA is not set".to_string(),
            )
        })?;
        let install_dir = PathBuf::from(base).join("EnvHub").join("bin");
        return Ok(PlatformInfo {
            is_windows: true,
            install_dir,
        });
    }

    let install_dir = match mode {
        InstallMode::Global => PathBuf::from("/usr/local/bin"),
        InstallMode::User => {
            let home = dirs::home_dir().ok_or_else(|| {
                CoreError::new(
                    ErrorCode::InstallPath,
                    "Failed to resolve home directory".to_string(),
                )
            })?;
            home.join(".local").join("bin")
        }
    };

    Ok(PlatformInfo {
        is_windows: false,
        install_dir,
    })
}

pub fn install_launcher(mode: InstallMode, launcher_path: &Path) -> Result<PathBuf, CoreError> {
    let platform = detect_platform(mode)?;
    if !launcher_path.exists() {
        return Err(CoreError::new(
            ErrorCode::MissingLauncher,
            format!("Launcher not found at {}", launcher_path.display()),
        ));
    }
    fs::create_dir_all(&platform.install_dir).map_err(|err| {
        let code = if err.kind() == std::io::ErrorKind::PermissionDenied {
            ErrorCode::Permission
        } else {
            ErrorCode::InstallPath
        };
        CoreError::new(
            code,
            format!("Failed to create install directory: {err}"),
        )
    })?;

    let launcher_name = if platform.is_windows {
        "envhub-launcher.exe"
    } else {
        "envhub-launcher"
    };
    let dest = platform.install_dir.join(launcher_name);
    fs::copy(launcher_path, &dest).map_err(|err| {
        let code = if err.kind() == std::io::ErrorKind::PermissionDenied {
            ErrorCode::Permission
        } else {
            ErrorCode::Io
        };
        CoreError::new(code, format!("Failed to copy launcher: {err}"))
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dest)
            .map_err(|err| CoreError::new(ErrorCode::Io, format!("{err}")))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest, perms)
            .map_err(|err| CoreError::new(ErrorCode::Io, format!("{err}")))?;
    }

    Ok(dest)
}

pub fn install_shim(name: &str, mode: InstallMode, launcher_path: &Path) -> Result<PathBuf, CoreError> {
    let platform = detect_platform(mode)?;
    install_shim_in(name, &platform.install_dir, launcher_path)
}

pub fn install_shim_in(
    name: &str,
    install_dir: &Path,
    launcher_path: &Path,
) -> Result<PathBuf, CoreError> {
    if name.trim().is_empty() {
        return Err(CoreError::new(
            ErrorCode::InvalidState,
            "App name must be non-empty".to_string(),
        ));
    }
    if !launcher_path.exists() {
        return Err(CoreError::new(
            ErrorCode::MissingLauncher,
            format!("Launcher not found at {}", launcher_path.display()),
        ));
    }
    fs::create_dir_all(install_dir).map_err(|err| {
        let code = if err.kind() == std::io::ErrorKind::PermissionDenied {
            ErrorCode::Permission
        } else {
            ErrorCode::InstallPath
        };
        CoreError::new(
            code,
            format!("Failed to create install directory: {err}"),
        )
    })?;

    if cfg!(windows) {
        let dest = install_dir.join(format!("{name}.exe"));
        fs::copy(launcher_path, &dest).map_err(|err| {
            let code = if err.kind() == std::io::ErrorKind::PermissionDenied {
                ErrorCode::Permission
            } else {
                ErrorCode::Io
            };
            CoreError::new(code, format!("Failed to copy shim: {err}"))
        })?;
        return Ok(dest);
    }

    let dest = install_dir.join(name);
    #[cfg(unix)]
    {
        use std::os::unix::fs as unix_fs;
        if dest.exists() {
            fs::remove_file(&dest).map_err(|err| {
                CoreError::new(ErrorCode::Io, format!("Failed to replace shim: {err}"))
            })?;
        }
        unix_fs::symlink(launcher_path, &dest).map_err(|err| {
            let code = if err.kind() == std::io::ErrorKind::PermissionDenied {
                ErrorCode::Permission
            } else {
                ErrorCode::Io
            };
            CoreError::new(code, format!("Failed to create shim: {err}"))
        })?;
    }
    Ok(dest)
}

pub fn install_shim_for_state(
    state: &State,
    name: &str,
    mode: InstallMode,
    launcher_path: &Path,
) -> Result<PathBuf, CoreError> {
    let app = state.apps.get(name).ok_or_else(|| {
        CoreError::new(
            ErrorCode::AppNotFound,
            format!("App \"{name}\" is not registered"),
        )
    })?;
    let install_dir = match &app.install_path {
        Some(path) => PathBuf::from(path),
        None => detect_platform(mode)?.install_dir,
    };
    install_shim_in(name, &install_dir, launcher_path)
}

pub fn load_state_for_install() -> Result<State, CoreError> {
    let path = default_state_path()?;
    load_state_from_path(&path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn install_shim_in_creates_copy_or_symlink() {
        let dir = TempDir::new().expect("temp dir");
        let install_dir = dir.path().join("bin");
        let launcher = dir.path().join(if cfg!(windows) {
            "envhub-launcher.exe"
        } else {
            "envhub-launcher"
        });
        fs::write(&launcher, b"binary").expect("launcher");

        let shim_path = install_shim_in("tool", &install_dir, &launcher).expect("shim");
        assert!(shim_path.exists());
    }

    #[test]
    fn install_shim_for_state_uses_custom_install_path() {
        let dir = TempDir::new().expect("temp dir");
        let custom_dir = dir.path().join("custom");
        let launcher = dir.path().join("launcher");
        fs::write(&launcher, b"binary").expect("launcher");

        let mut state = State::default();
        state.apps.insert(
            "tool".to_string(),
            AppConfig {
                target_binary: "tool-bin".to_string(),
                install_path: Some(custom_dir.to_string_lossy().to_string()),
                ..AppConfig::default()
            },
        );

        let shim_path =
            install_shim_for_state(&state, "tool", InstallMode::User, &launcher).expect("shim");
        assert!(shim_path.exists());
    }
}
