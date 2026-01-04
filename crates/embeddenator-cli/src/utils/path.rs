//! Path manipulation utilities for logical filesystem paths

use std::path::Path;

/// Convert a path to a forward-slash string representation
pub fn path_to_forward_slash_string(path: &Path) -> String {
    path.components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str().map(|v| v.to_string()),
            _ => None,
        })
        .collect::<Vec<String>>()
        .join("/")
}

/// Generate a logical path for a file input
/// 
/// If the path is relative, return it as-is with forward slashes.
/// If the path is absolute and within cwd, return the relative portion.
/// Otherwise, return just the filename.
pub fn logical_path_for_file_input(path: &Path, cwd: &Path) -> String {
    if path.is_relative() {
        return path_to_forward_slash_string(path);
    }

    if let Ok(rel) = path.strip_prefix(cwd) {
        let s = path_to_forward_slash_string(rel);
        if !s.is_empty() {
            return s;
        }
    }

    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("input.bin")
        .to_string()
}
