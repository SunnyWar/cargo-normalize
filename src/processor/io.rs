use similar::TextDiff;
use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use walkdir::WalkDir;

pub(super) fn collect_rust_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    if root.is_file() {
        if root.extension() == Some(OsStr::new("rs")) {
            return Ok(vec![root.to_path_buf()]);
        }
        return Err(format!(
            "Path {} is a file but not a Rust source (.rs)",
            root.display()
        ));
    }
    if !root.is_dir() {
        return Err(format!("Path does not exist: {}", root.display()));
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_entry(|entry| {
        entry.file_name() != OsStr::new("target") && entry.file_name() != OsStr::new(".git")
    }) {
        let entry = entry.map_err(|err| format!("Directory walk failed: {err}"))?;
        if entry.file_type().is_file() && entry.path().extension() == Some(OsStr::new("rs")) {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

pub(super) fn print_diff(path: &Path, before: &str, after: &str) {
    let diff = TextDiff::from_lines(before, after);
    let unified = diff
        .unified_diff()
        .context_radius(3)
        .header(
            &format!("a/{}", path.display()),
            &format!("b/{}", path.display()),
        )
        .to_string();
    println!("{unified}");
}

pub(super) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("Cannot determine parent directory for {}", path.display()))?;
    let mut temp = NamedTempFile::new_in(parent)
        .map_err(|err| format!("Failed to create temp file in {}: {err}", parent.display()))?;
    temp.write_all(bytes)
        .map_err(|err| format!("Failed to write temp file for {}: {err}", path.display()))?;
    temp.flush()
        .map_err(|err| format!("Failed to flush temp file for {}: {err}", path.display()))?;
    temp.persist(path)
        .map_err(|err| format!("Failed to persist temp file to {}: {err}", path.display()))?;
    Ok(())
}
