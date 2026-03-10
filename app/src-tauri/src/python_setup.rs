use std::path::PathBuf;
use std::process::Command;

use tauri::{AppHandle, Manager};

fn parse_pyproject_dependencies(pyproject: &str) -> Vec<String> {
    let mut deps: Vec<String> = Vec::new();
    let mut in_deps = false;

    for raw_line in pyproject.lines() {
        let line = raw_line.trim();

        if !in_deps {
            if line.starts_with("dependencies") && line.contains('[') {
                in_deps = true;
            }
            continue;
        }

        if line.starts_with(']') {
            break;
        }

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let first_quote = line.find('"');
        let last_quote = line.rfind('"');
        if let (Some(a), Some(b)) = (first_quote, last_quote) {
            if b > a {
                let dep = line[a + 1..b].trim();
                if !dep.is_empty() {
                    deps.push(dep.to_string());
                }
            }
        }
    }

    deps
}

fn normalize_dependency_name(spec: &str) -> Option<String> {
    let trimmed = spec.split(';').next().unwrap_or("").trim();
    if trimmed.is_empty() {
        return None;
    }

    let before_at = trimmed.split('@').next().unwrap_or("").trim();
    if before_at.is_empty() {
        return None;
    }

    let mut end = before_at.len();
    for (idx, ch) in before_at.char_indices() {
        if matches!(ch, '=' | '<' | '>' | '!' | '~') {
            end = idx;
            break;
        }
    }

    let name = &before_at[..end];
    let name = name.split('[').next().unwrap_or("").trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn resolve_pyproject_path(app: &AppHandle) -> Result<PathBuf, String> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");

    let repo_path = repo_root
        .join("resources")
        .join("python-backend")
        .join("pyproject.toml");
    if repo_path.exists() {
        return Ok(repo_path);
    }

    let resource_dir = app.path().resource_dir().map_err(|e| e.to_string())?;
    Ok(resource_dir.join("python-backend").join("pyproject.toml"))
}

pub fn pyproject_dependency_names(app: &AppHandle) -> Result<Vec<String>, String> {
    let pyproject_path = resolve_pyproject_path(app)?;
    if !pyproject_path.exists() {
        return Err(format!(
            "pyproject.toml not found at {}",
            pyproject_path.display()
        ));
    }

    let pyproject = std::fs::read_to_string(&pyproject_path)
        .map_err(|e| format!("Failed to read pyproject.toml: {}", e))?;

    let deps = parse_pyproject_dependencies(&pyproject);
    let mut out: Vec<String> = deps
        .into_iter()
        .filter_map(|dep| normalize_dependency_name(&dep))
        .collect();
    out.sort();
    out.dedup();
    Ok(out)
}

pub fn install_python_deps(app: &AppHandle, pip_path: PathBuf) -> Result<String, String> {
    if !pip_path.exists() {
        return Err("Virtual environment not found. Please create it first.".to_string());
    }

    let _ = Command::new(pip_path.to_str().unwrap())
        .arg("install")
        .arg("--upgrade")
        .arg("pip")
        .output();

    let pyproject_path = resolve_pyproject_path(app)?;
    if !pyproject_path.exists() {
        return Err(format!(
            "pyproject.toml not found at {}",
            pyproject_path.display()
        ));
    }

    let pyproject = std::fs::read_to_string(&pyproject_path)
        .map_err(|e| format!("Failed to read pyproject.toml: {}", e))?;

    let deps = parse_pyproject_dependencies(&pyproject);
    if deps.is_empty() {
        return Err("No dependencies found in pyproject.toml".to_string());
    }

    // Install mlx-audio without deps to avoid resolver conflicts.
    let mut mlx_audio_spec: Option<String> = None;
    let mut rest: Vec<String> = Vec::new();
    for dep in deps {
        if dep.starts_with("mlx-audio") {
            // Reject duplicates to avoid ambiguity.
            if mlx_audio_spec.is_some() {
                return Err(
                    "Multiple mlx-audio entries found in pyproject.toml dependencies".to_string(),
                );
            }
            mlx_audio_spec = Some(dep);
        } else {
            rest.push(dep);
        }
    }

    if let Some(spec) = mlx_audio_spec {
        let output = Command::new(pip_path.to_str().unwrap())
            .args([
                "install",
                "--upgrade",
                "--force-reinstall",
                "--no-deps",
                &spec,
            ])
            .output()
            .map_err(|e| format!("Failed to install mlx-audio: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "Failed to install mlx-audio: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    let mut cmd = Command::new(pip_path.to_str().unwrap());
    cmd.arg("install").arg("--upgrade").arg("--force-reinstall");
    for dep in rest {
        cmd.arg(dep);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to install deps: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to install dependencies: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok("Dependencies installed successfully".to_string())
}
