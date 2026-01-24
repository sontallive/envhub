use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use envhub_core::{AppConfig, CoreError, ErrorCode};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("envhub-launcher error: {} - {}", err.code, err.message);
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode, CoreError> {
    let app_name = app_name_from_argv0()
        .ok_or_else(|| CoreError::new(ErrorCode::InvalidState, "Missing argv[0]".to_string()))?;

    // Only handle --version/--help when directly running envhub-launcher
    // For aliases (e.g., claudex), pass all args through to the target binary
    if app_name == "envhub-launcher" {
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            match args[1].as_str() {
                "--version" | "-v" => {
                    println!("envhub-launcher {}", VERSION);
                    return Ok(ExitCode::SUCCESS);
                }
                "--help" | "-h" => {
                    print_help();
                    return Ok(ExitCode::SUCCESS);
                }
                _ => {}
            }
        }

        // Prevent direct execution of envhub-launcher without flags
        eprintln!("Error: envhub-launcher should not be run directly.");
        eprintln!("This binary is meant to be symlinked/copied with your app name.");
        eprintln!();
        eprintln!("Usage:");
        eprintln!("  1. Register an app in envhub TUI");
        eprintln!("  2. Install the shim for that app");
        eprintln!("  3. Run your app by its alias name (e.g., 'iclaude', 'inode')");
        eprintln!();
        eprintln!("For more information, run: envhub-launcher --help");
        return Ok(ExitCode::from(1));
    }
    let state = envhub_core::load_state()?;

    let (target_binary, profile_env, command_args) = match state.apps.get(&app_name) {
        Some(app) => {
            let target = app.target_binary.clone();
            if target.trim().is_empty() {
                return Err(CoreError::new(
                    ErrorCode::InvalidState,
                    format!("App \"{app_name}\" is missing target_binary"),
                ));
            }
            let (env, args) = select_profile_config(app);
            (target, env, args)
        }
        None => (app_name.clone(), HashMap::new(), Vec::new()),
    };

    let resolved = resolve_target_binary(&target_binary)?;
    let mut env = merge_env(std::env::vars_os().collect(), &profile_env);

    let mut args: Vec<OsString> = command_args.into_iter().map(OsString::from).collect();
    args.extend(std::env::args_os().skip(1));
    if cfg!(windows) {
        let status = Command::new(&resolved)
            .args(args)
            .envs(env.drain())
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .map_err(|err| {
                CoreError::new(ErrorCode::Io, format!("Failed to launch target: {err}"))
            })?;
        let code = status.code().unwrap_or(1) as u8;
        return Ok(ExitCode::from(code));
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = Command::new(&resolved).args(args).envs(env.drain()).exec();
        Err(CoreError::new(
            ErrorCode::Io,
            format!("Failed to exec target: {err}"),
        ))
    }
    #[cfg(not(unix))]
    {
        Err(CoreError::new(
            ErrorCode::Io,
            "Unsupported platform".to_string(),
        ))
    }
}

fn print_help() {
    println!("envhub-launcher {}", VERSION);
    println!();
    println!("ABOUT:");
    println!("  A lightweight shim that intercepts command calls and injects environment");
    println!("  variables based on EnvHub's active profile configuration.");
    println!();
    println!("USAGE:");
    println!("  This binary should NOT be run directly. It's designed to be used as a shim:");
    println!();
    println!("  1. Register an app in EnvHub TUI (e.g., alias 'iclaude' for '/usr/local/bin/claude')");
    println!("  2. Install the shim (press 'i' in TUI)");
    println!("  3. Run your alias: iclaude code");
    println!();
    println!("  The launcher will:");
    println!("    - Read your EnvHub config to find the active profile");
    println!("    - Inject environment variables from that profile");
    println!("    - Execute the original binary with the modified environment");
    println!();
    println!("OPTIONS:");
    println!("  -h, --help       Show this help message");
    println!("  -v, --version    Show version information");
    println!();
    println!("For more information: https://github.com/sontallive/envhub");
}

fn app_name_from_argv0() -> Option<String> {
    let arg0 = std::env::args_os().next()?;
    let name = Path::new(&arg0).file_name()?.to_string_lossy().to_string();
    if cfg!(windows) {
        return Some(name.trim_end_matches(".exe").to_string());
    }
    Some(name)
}

fn select_profile_config(app: &AppConfig) -> (HashMap<String, String>, Vec<String>) {
    if app.profiles.is_empty() {
        return (HashMap::new(), Vec::new());
    }
    let profile = app
        .active_profile
        .as_ref()
        .filter(|name| app.profiles.contains_key(*name))
        .or_else(|| app.profiles.keys().next());
    match profile.and_then(|name| app.profiles.get(name)) {
        Some(profile) => (
            profile.env.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            profile.command_args.clone(),
        ),
        None => (HashMap::new(), Vec::new()),
    }
}

fn merge_env(
    base: Vec<(OsString, OsString)>,
    overrides: &HashMap<String, String>,
) -> HashMap<OsString, OsString> {
    let mut env: HashMap<OsString, OsString> = base.into_iter().collect();
    for (key, value) in overrides {
        env.insert(OsString::from(key), OsString::from(value));
    }
    env
}

fn resolve_target_binary(target: &str) -> Result<PathBuf, CoreError> {
    let target_path = Path::new(target);
    let self_path = std::env::current_exe().map_err(|err| {
        CoreError::new(
            ErrorCode::Io,
            format!("Failed to resolve launcher path: {err}"),
        )
    })?;

    if target_path.is_absolute() {
        return ensure_not_self(target_path.to_path_buf(), &self_path);
    }

    if target_path.components().count() > 1 {
        if target_path.exists() {
            return ensure_not_self(target_path.to_path_buf(), &self_path);
        }
        return Err(CoreError::new(
            ErrorCode::TargetNotFound,
            format!("Target \"{target}\" not found"),
        ));
    }

    let resolved = find_executable_in_path(target, &self_path).ok_or_else(|| {
        CoreError::new(
            ErrorCode::TargetNotFound,
            format!("Target \"{target}\" not found in PATH"),
        )
    })?;
    Ok(resolved)
}

fn find_executable_in_path(target: &str, self_path: &Path) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let path_exts = if cfg!(windows) {
        std::env::var_os("PATHEXT")
            .map(|exts| {
                exts.to_string_lossy()
                    .split(';')
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_else(|| vec![".EXE".to_string()])
    } else {
        Vec::new()
    };

    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(target);
        if cfg!(windows) {
            if candidate.exists() {
                if let Ok(path) = ensure_not_self(candidate.clone(), self_path) {
                    return Some(path);
                }
            }
            for ext in &path_exts {
                let candidate = dir.join(format!("{target}{ext}"));
                if candidate.exists() {
                    if let Ok(path) = ensure_not_self(candidate.clone(), self_path) {
                        return Some(path);
                    }
                }
            }
        } else if is_executable(&candidate) {
            if let Ok(path) = ensure_not_self(candidate.clone(), self_path) {
                return Some(path);
            }
        }
    }
    None
}

fn ensure_not_self(path: PathBuf, self_path: &Path) -> Result<PathBuf, CoreError> {
    if same_executable(&path, self_path).unwrap_or(false) {
        return Err(CoreError::new(
            ErrorCode::TargetNotFound,
            "Target binary resolves to envhub-launcher".to_string(),
        ));
    }
    Ok(path)
}

fn same_executable(path: &Path, self_path: &Path) -> Option<bool> {
    let canonical_candidate = path.canonicalize().ok()?;
    let canonical_self = self_path.canonicalize().ok()?;
    if canonical_candidate == canonical_self {
        return Some(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let candidate_meta = fs_metadata(&canonical_candidate)?;
        let self_meta = fs_metadata(&canonical_self)?;
        return Some(candidate_meta.ino() == self_meta.ino());
    }
    #[cfg(not(unix))]
    {
        Some(false)
    }
}

fn fs_metadata(path: &Path) -> Option<std::fs::Metadata> {
    std::fs::metadata(path).ok()
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
        Ok(meta) => meta.permissions().mode() & 0o111 != 0,
        Err(_) => false,
    }
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_profile_env_falls_back_to_first_profile() {
        let mut app = AppConfig::default();
        app.target_binary = "tool".to_string();
        let mut profile = envhub_core::ProfileConfig::default();
        profile.env.insert("KEY".to_string(), "VALUE".to_string());
        app.profiles.insert("work".to_string(), profile);
        let (env, _args) = select_profile_config(&app);
        assert_eq!(env.get("KEY").map(String::as_str), Some("VALUE"));
    }

    #[test]
    fn merge_env_overrides_existing_values() {
        let base = vec![(OsString::from("KEY"), OsString::from("OLD"))];
        let mut overrides = HashMap::new();
        overrides.insert("KEY".to_string(), "NEW".to_string());
        let merged = merge_env(base, &overrides);
        assert_eq!(
            merged.get(std::ffi::OsStr::new("KEY")),
            Some(&OsString::from("NEW"))
        );
    }

    #[test]
    fn resolve_target_binary_skips_self() {
        let self_path = std::env::current_exe().expect("self");
        let self_dir = self_path.parent().expect("self dir").to_path_buf();
        let file_name = self_path.file_name().unwrap().to_string_lossy().to_string();

        let original_path = std::env::var_os("PATH");
        unsafe {
            std::env::set_var("PATH", &self_dir);
        }
        let found = find_executable_in_path(&file_name, &self_path);
        if let Some(path) = original_path {
            unsafe {
                std::env::set_var("PATH", path);
            }
        }
        assert!(found.is_none());
    }
}
