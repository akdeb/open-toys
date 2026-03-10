use std::path::PathBuf;

use tauri::{AppHandle, Manager};

pub(crate) fn get_elato_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("Failed to resolve app data directory")
}

pub(crate) fn get_voices_dir(app: &AppHandle) -> PathBuf {
    get_elato_dir(app).join("voices")
}

pub(crate) fn get_images_dir(app: &AppHandle) -> PathBuf {
    get_elato_dir(app).join("images")
}

pub(crate) fn get_venv_path(app: &AppHandle) -> PathBuf {
    get_elato_dir(app).join("python_env")
}

pub(crate) fn get_python_runtime_root(app: &AppHandle) -> PathBuf {
    get_elato_dir(app).join("python_runtime")
}

pub(crate) fn get_runtime_python(app: &AppHandle) -> PathBuf {
    let root = get_python_runtime_root(app);
    let bin = root.join("python").join("bin");
    let python = bin.join("python");
    if python.exists() {
        return python;
    }

    let python3 = bin.join("python3");
    if python3.exists() {
        return python3;
    }

    bin.join("python3.11")
}

pub(crate) fn get_venv_python(app: &AppHandle) -> PathBuf {
    let venv = get_venv_path(app);
    if cfg!(target_os = "windows") {
        venv.join("Scripts").join("python.exe")
    } else {
        let bin = venv.join("bin");
        let python = bin.join("python");
        if python.exists() {
            return python;
        }
        bin.join("python3")
    }
}

pub(crate) fn get_venv_pip(app: &AppHandle) -> PathBuf {
    let venv = get_venv_path(app);
    if cfg!(target_os = "windows") {
        venv.join("Scripts").join("pip.exe")
    } else {
        let bin = venv.join("bin");
        let pip = bin.join("pip");
        if pip.exists() {
            return pip;
        }
        bin.join("pip3")
    }
}
