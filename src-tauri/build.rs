use std::path::Path;

fn main() {
    tauri_build::build();

    build_launcher();
}

fn build_launcher() {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing");
    let workspace_dir = Path::new(&manifest_dir)
        .parent()
        .expect("workspace root missing");
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let launcher_name = if cfg!(windows) {
        "envhub-launcher.exe"
    } else {
        "envhub-launcher"
    };

    let target_dir = workspace_dir.join("target").join("tauri-launcher");
    let launcher_path = target_dir.join(&profile).join(launcher_name);
    let launcher_src_dir = workspace_dir.join("crates/envhub-launcher/src");
    let launcher_manifest = workspace_dir.join("crates/envhub-launcher/Cargo.toml");

    let needs_rebuild = launcher_needs_rebuild(&launcher_path, &launcher_src_dir, &launcher_manifest);
    if needs_rebuild {
        let mut cmd = Command::new("cargo");
        cmd.current_dir(workspace_dir)
            .arg("build")
            .arg("--manifest-path")
            .arg("crates/envhub-launcher/Cargo.toml")
            .arg("--target-dir")
            .arg(&target_dir);
        if profile == "release" {
            cmd.arg("--release");
        }
        let status = cmd.status().expect("failed to run cargo build");
        if !status.success() {
            panic!("envhub-launcher build failed");
        }
    }

    let resources_dir = Path::new(&manifest_dir).join("resources");
    fs::create_dir_all(&resources_dir).expect("failed to create resources dir");
    let dest = resources_dir.join(launcher_name);
    fs::copy(&launcher_path, &dest).expect("failed to copy envhub-launcher");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dest)
            .expect("failed to read launcher metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest, perms).expect("failed to set launcher permissions");
    }

    println!("cargo:rerun-if-changed=../crates/envhub-launcher/src");
    println!("cargo:rerun-if-changed=../crates/envhub-launcher/Cargo.toml");
}

fn launcher_needs_rebuild(launcher_path: &Path, src_dir: &Path, manifest: &Path) -> bool {
    if !launcher_path.exists() {
        return true;
    }

    let Ok(launcher_meta) = std::fs::metadata(launcher_path) else {
        return true;
    };
    let Ok(launcher_mtime) = launcher_meta.modified() else {
        return true;
    };

    if file_is_newer(manifest, launcher_mtime) {
        return true;
    }

    dir_has_newer_files(src_dir, launcher_mtime)
}

fn file_is_newer(path: &Path, baseline: std::time::SystemTime) -> bool {
    match std::fs::metadata(path).and_then(|m| m.modified()) {
        Ok(mtime) => mtime > baseline,
        Err(_) => true,
    }
}

fn dir_has_newer_files(dir: &Path, baseline: std::time::SystemTime) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return true;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if dir_has_newer_files(&path, baseline) {
                return true;
            }
        } else if file_is_newer(&path, baseline) {
            return true;
        }
    }
    false
}
