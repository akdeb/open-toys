use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::paths::{
    get_elato_dir, get_python_runtime_root, get_runtime_python, get_venv_path, get_venv_pip,
    get_venv_python,
};
use crate::python_setup;

const PYTHON_RUNTIME_URL: &str = "https://github.com/astral-sh/python-build-standalone/releases/download/20251217/cpython-3.11.14%2B20251217-aarch64-apple-darwin-install_only.tar.gz";

fn emit_setup_progress(app: &AppHandle, message: &str) {
    app.emit("setup-progress", message).ok();
}

fn ensure_python_symlinks(runtime_root: &PathBuf) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let bin_dir = runtime_root.join("python").join("bin");
        let py311 = bin_dir.join("python3.11");
        if !py311.exists() {
            return Err(format!(
                "Python runtime extract failed: missing {}",
                py311.display()
            ));
        }

        let py3 = bin_dir.join("python3");
        if !py3.exists() {
            symlink("python3.11", &py3)
                .map_err(|e| format!("Failed to create python3 symlink: {e}"))?;
        }

        let py = bin_dir.join("python");
        if !py.exists() {
            symlink("python3.11", &py)
                .map_err(|e| format!("Failed to create python symlink: {e}"))?;
        }
    }

    Ok(())
}

fn runtime_python_version(python_path: &PathBuf) -> Option<String> {
    let output = Command::new(python_path.to_str().unwrap())
        .arg("--version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !stdout.is_empty() {
        return Some(stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !stderr.is_empty() {
        Some(stderr)
    } else {
        None
    }
}

fn download_python_runtime(app: &AppHandle) -> Result<PathBuf, String> {
    if cfg!(target_arch = "aarch64") == false || cfg!(target_os = "macos") == false {
        return Err(
            "Automatic Python bootstrap currently supports macOS Apple Silicon only".to_string(),
        );
    }

    let runtime_root = get_python_runtime_root(app);
    let runtime_python = get_runtime_python(app);
    if runtime_python.exists() {
        return Ok(runtime_python);
    }

    let parent = runtime_root
        .parent()
        .ok_or_else(|| "Invalid runtime directory".to_string())?;
    fs::create_dir_all(parent).map_err(|e| format!("Failed to create app data dir: {e}"))?;

    let download_path = parent.join("python-runtime.tar.gz");
    let tmp_runtime_root = parent.join("python_runtime_tmp");
    let _ = fs::remove_dir_all(&tmp_runtime_root);

    emit_setup_progress(app, "Downloading Python 3.11 runtime...");
    let curl_status = Command::new("curl")
        .args([
            "-L",
            "--fail",
            "--retry",
            "3",
            "--connect-timeout",
            "20",
            "-o",
            download_path.to_string_lossy().as_ref(),
            PYTHON_RUNTIME_URL,
        ])
        .status()
        .map_err(|e| format!("Failed to start Python runtime download: {e}"))?;

    if !curl_status.success() {
        return Err("Python runtime download failed".to_string());
    }

    emit_setup_progress(app, "Extracting Python runtime...");
    fs::create_dir_all(&tmp_runtime_root)
        .map_err(|e| format!("Failed to create temp runtime dir: {e}"))?;
    let tar_status = Command::new("tar")
        .args([
            "-xzf",
            download_path.to_string_lossy().as_ref(),
            "-C",
            tmp_runtime_root.to_string_lossy().as_ref(),
        ])
        .status()
        .map_err(|e| format!("Failed to extract Python runtime: {e}"))?;

    let _ = fs::remove_file(&download_path);

    if !tar_status.success() {
        let _ = fs::remove_dir_all(&tmp_runtime_root);
        return Err("Failed to extract Python runtime archive".to_string());
    }

    ensure_python_symlinks(&tmp_runtime_root)?;

    if runtime_root.exists() || fs::symlink_metadata(&runtime_root).is_ok() {
        if runtime_root.is_dir() {
            fs::remove_dir_all(&runtime_root)
                .map_err(|e| format!("Failed to replace runtime directory: {e}"))?;
        } else {
            fs::remove_file(&runtime_root)
                .map_err(|e| format!("Failed to replace runtime file: {e}"))?;
        }
    }
    fs::rename(&tmp_runtime_root, &runtime_root)
        .map_err(|e| format!("Failed to finalize Python runtime: {e}"))?;

    let final_python = get_runtime_python(app);
    if !final_python.exists() {
        return Err(format!(
            "Python runtime installed but interpreter was not found at {}",
            final_python.display()
        ));
    }

    emit_setup_progress(app, "Python runtime is ready.");
    Ok(final_python)
}

fn pip_has_package(python: &PathBuf, name: &str) -> bool {
    Command::new(python.to_str().unwrap())
        .arg("-m")
        .arg("pip")
        .arg("show")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn deps_installed_from_pyproject(app: &AppHandle, python: &PathBuf) -> bool {
    let deps = python_setup::pyproject_dependency_names(app).unwrap_or_default();
    if deps.is_empty() {
        return false;
    }
    deps.iter().all(|dep| pip_has_package(python, dep))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStatus {
    pub python_installed: bool,
    pub python_version: Option<String>,
    pub python_path: Option<String>,
    pub venv_exists: bool,
    pub venv_path: Option<String>,
    pub deps_installed: bool,
}

#[tauri::command]
pub async fn check_setup_status(app: AppHandle) -> Result<SetupStatus, String> {
    let venv_path = get_venv_path(&app);
    let venv_python = get_venv_python(&app);
    let venv_exists = venv_python.exists();

    let runtime_python = get_runtime_python(&app);

    let (python_installed, python_version, python_path) = if venv_exists {
        let output = Command::new(venv_python.to_str().unwrap())
            .arg("--version")
            .output()
            .ok();
        let v = output.as_ref().and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        });
        (true, v, Some(venv_python.to_string_lossy().to_string()))
    } else if runtime_python.exists() {
        (
            true,
            runtime_python_version(&runtime_python),
            Some(runtime_python.to_string_lossy().to_string()),
        )
    } else {
        (false, None, None)
    };

    let deps_installed = if venv_exists {
        deps_installed_from_pyproject(&app, &venv_python)
    } else {
        false
    };

    Ok(SetupStatus {
        python_installed,
        python_version,
        python_path,
        venv_exists,
        venv_path: if venv_exists {
            Some(venv_path.to_string_lossy().to_string())
        } else {
            None
        },
        deps_installed,
    })
}

#[tauri::command]
pub async fn ensure_python_runtime(app: AppHandle) -> Result<String, String> {
    let runtime_python = download_python_runtime(&app)?;
    Ok(runtime_python.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn create_python_venv(app: AppHandle) -> Result<String, String> {
    let venv_path = get_venv_path(&app);
    let venv_python = get_venv_python(&app);

    emit_setup_progress(&app, "Preparing Python runtime...");
    let python_for_venv = download_python_runtime(&app)?;

    if venv_python.exists() {
        emit_setup_progress(&app, "Virtual environment already exists...");
        return Ok(venv_path.to_string_lossy().to_string());
    }

    if venv_path.exists() || fs::symlink_metadata(&venv_path).is_ok() {
        emit_setup_progress(&app, "Cleaning up existing invalid environment...");
        if venv_path.is_dir() {
            fs::remove_dir_all(&venv_path).map_err(|e| e.to_string())?;
        } else {
            fs::remove_file(&venv_path).map_err(|e| e.to_string())?;
        }
    }

    if let Some(parent) = venv_path.parent() {
        fs::create_dir_all(parent).map_err(|e: std::io::Error| e.to_string())?;
    }

    emit_setup_progress(&app, "Creating Python virtual environment...");

    let output = Command::new(python_for_venv.to_str().unwrap())
        .arg("-m")
        .arg("venv")
        .arg("--clear")
        .arg(&venv_path)
        .output()
        .map_err(|e| format!("Failed to create venv: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to create venv: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(venv_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn install_python_deps(app: AppHandle) -> Result<String, String> {
    let pip = get_venv_pip(&app);

    app.emit(
        "setup-progress",
        "Installing Python dependencies (this may take a few minutes)...",
    )
    .ok();

    let result = python_setup::install_python_deps(&app, pip)?;
    app.emit("setup-progress", "Dependencies installed successfully!")
        .ok();
    Ok(result)
}

#[tauri::command]
pub async fn mark_setup_complete(app: AppHandle) -> Result<(), String> {
    let elato_dir = get_elato_dir(&app);
    let marker_file = elato_dir.join(".setup_complete");
    fs::create_dir_all(&elato_dir).map_err(|e: std::io::Error| e.to_string())?;
    let status = check_setup_status(app.clone()).await?;
    let runtime_python = get_runtime_python(&app);
    let manifest = serde_json::json!({
        "setup_complete": true,
        "python_version": status.python_version,
        "python_path": runtime_python.to_string_lossy().to_string(),
        "venv_path": status.venv_path,
        "deps_installed": status.deps_installed,
    });
    let mut f = fs::File::create(&marker_file).map_err(|e| e.to_string())?;
    f.write_all(manifest.to_string().as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn is_first_launch(app: AppHandle) -> Result<bool, String> {
    let elato_dir = get_elato_dir(&app);
    let marker_file = elato_dir.join(".setup_complete");
    let venv_python = get_venv_python(&app);

    Ok(!marker_file.exists() || !venv_python.exists())
}
