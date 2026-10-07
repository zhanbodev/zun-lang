use std::path::{Path, PathBuf};

use super::manifest::ProjectConfig;

pub fn resolve_manifest_path(dir: &Path) -> PathBuf {
    dir.join("package.toml")
}

pub fn resolve_project_root(path: &Path) -> PathBuf {
    let start = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(Path::new("."))
    };

    for candidate in start.ancestors() {
        if resolve_manifest_path(candidate).exists() {
            return candidate.to_path_buf();
        }
    }

    start.to_path_buf()
}

pub fn resolve_main_file(
    path: &Path,
    project_root: &Path,
    manifest: Option<&ProjectConfig>,
) -> PathBuf {
    if path.is_file() {
        return path.to_path_buf();
    }

    let entry = manifest.map(|cfg| cfg.entry.as_str()).unwrap_or("main.zun");
    project_root.join(entry)
}
