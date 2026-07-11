use crate::importing::normalize_import_path;
use crate::wallpaper_media::is_supported_wallpaper_path;
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ImportRejectedPath {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ImportScanResult {
    pub files: Vec<String>,
    pub rejected: Vec<ImportRejectedPath>,
}

pub(crate) fn scan_import_paths(paths: Vec<String>) -> Result<ImportScanResult, String> {
    if paths.is_empty() {
        return Err("No files or folders were selected".to_string());
    }

    let mut files = Vec::new();
    let mut rejected = Vec::new();
    let mut seen = HashSet::new();

    for selected_path in paths {
        let path = match normalize_import_path(&selected_path) {
            Ok(path) => path,
            Err(reason) => {
                rejected.push(ImportRejectedPath {
                    path: selected_path,
                    reason,
                });
                continue;
            }
        };

        collect_import_files(&path, path.is_dir(), &mut seen, &mut files, &mut rejected);
    }

    files.sort();
    Ok(ImportScanResult { files, rejected })
}

fn collect_import_files(
    path: &Path,
    from_folder: bool,
    seen: &mut HashSet<String>,
    files: &mut Vec<String>,
    rejected: &mut Vec<ImportRejectedPath>,
) {
    if path.is_dir() {
        let entries = match std::fs::read_dir(path) {
            Ok(entries) => entries,
            Err(error) => {
                rejected.push(ImportRejectedPath {
                    path: path.to_string_lossy().to_string(),
                    reason: format!("Failed to read folder: {}", error),
                });
                return;
            }
        };

        for entry in entries {
            match entry {
                Ok(entry) => {
                    collect_import_files(&entry.path(), true, seen, files, rejected);
                }
                Err(error) => rejected.push(ImportRejectedPath {
                    path: path.to_string_lossy().to_string(),
                    reason: format!("Failed to read folder entry: {}", error),
                }),
            }
        }
        return;
    }

    if !path.exists() {
        rejected.push(ImportRejectedPath {
            path: path.to_string_lossy().to_string(),
            reason: "Path does not exist".to_string(),
        });
        return;
    }

    if !path.is_file() {
        rejected.push(ImportRejectedPath {
            path: path.to_string_lossy().to_string(),
            reason: "Path is not a file".to_string(),
        });
        return;
    }

    if !is_supported_wallpaper_path(path) {
        if !from_folder {
            rejected.push(ImportRejectedPath {
                path: path.to_string_lossy().to_string(),
                reason: "Unsupported wallpaper format".to_string(),
            });
        }
        return;
    }

    let file_path = path.to_string_lossy().to_string();
    if seen.insert(file_path.clone()) {
        files.push(file_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock moved backwards")
            .as_nanos();

        std::env::temp_dir().join(format!(
            "wallscape-import-scan-{}-{}-{}",
            name,
            std::process::id(),
            stamp
        ))
    }

    #[test]
    fn scan_rejects_empty_selection() {
        let error = scan_import_paths(Vec::new()).expect_err("empty selection should fail");

        assert_eq!(error, "No files or folders were selected");
    }

    #[test]
    fn scan_collects_supported_files_from_nested_folders() {
        let root = unique_dir("folder");
        let nested = root.join("nested");
        fs::create_dir_all(&nested).expect("test folders should be writable");

        let first = root.join("first.jpg");
        let second = nested.join("second.png");
        fs::write(&first, b"image").expect("first test file should be writable");
        fs::write(&second, b"image").expect("second test file should be writable");
        fs::write(root.join("ignored.txt"), b"text").expect("ignored test file should be writable");

        let result = scan_import_paths(vec![root.to_string_lossy().to_string()])
            .expect("folder scan should succeed");

        assert_eq!(result.files.len(), 2);
        assert!(result.files.iter().any(|path| path.ends_with("first.jpg")));
        assert!(result.files.iter().any(|path| path.ends_with("second.png")));
        assert!(result.rejected.is_empty());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn scan_rejects_explicit_unsupported_file() {
        let root = unique_dir("unsupported");
        fs::create_dir_all(&root).expect("test folder should be writable");
        let unsupported = root.join("wallpaper.txt");
        fs::write(&unsupported, b"text").expect("test file should be writable");

        let result = scan_import_paths(vec![unsupported.to_string_lossy().to_string()])
            .expect("explicit unsupported scan should return a result");

        assert!(result.files.is_empty());
        assert_eq!(result.rejected.len(), 1);
        assert_eq!(result.rejected[0].reason, "Unsupported wallpaper format");

        let _ = fs::remove_dir_all(root);
    }
}
