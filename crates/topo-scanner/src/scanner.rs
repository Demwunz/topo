use crate::hash;
use ignore::WalkBuilder;
use std::path::Path;
use topo_core::{FileInfo, FileRole, Language};

/// Walks a directory tree, respecting .gitignore rules, and produces `FileInfo` entries.
pub struct Scanner<'a> {
    root: &'a Path,
}

impl<'a> Scanner<'a> {
    pub fn new(root: &'a Path) -> Self {
        Self { root }
    }

    /// Directories that are always excluded from scanning, regardless of .gitignore.
    /// These are either VCS internals or universally non-source content.
    const ALWAYS_SKIP_DIRS: &'static [&'static str] = &[
        ".git",
        "node_modules",
        ".topo",
        "__pycache__",
        ".venv",
        "venv",
        ".env",
        ".svn",
        ".hg",
    ];

    /// Scan the directory tree and return metadata for all non-ignored files.
    pub fn scan(&self) -> anyhow::Result<Vec<FileInfo>> {
        let mut files = Vec::new();

        let walker = WalkBuilder::new(self.root)
            .hidden(false) // don't skip dotfiles by default
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .filter_entry(|entry| {
                // Skip directories that should always be excluded
                if entry.file_type().is_some_and(|ft| ft.is_dir())
                    && let Some(name) = entry.file_name().to_str()
                    && Self::ALWAYS_SKIP_DIRS.contains(&name)
                {
                    return false;
                }
                true
            })
            .build();

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Skip directories
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                continue;
            }

            let path = entry.path();

            // Get relative path from root
            let rel_path = match path.strip_prefix(self.root) {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Skip empty relative paths (the root itself)
            if rel_path.as_os_str().is_empty() {
                continue;
            }

            // Always use forward slashes for consistent cross-platform paths
            let rel_str = rel_path.to_string_lossy().replace('\\', "/");

            // Get file metadata
            let metadata = match path.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            // Skip non-regular files
            if !metadata.is_file() {
                continue;
            }

            let size = metadata.len();
            let language = Language::from_path(rel_path);
            let role = FileRole::from_path(rel_path);

            let sha256 = match hash::sha256_file(path) {
                Ok(h) => h,
                Err(_) => continue,
            };

            files.push(FileInfo {
                path: rel_str,
                size,
                language,
                role,
                sha256,
            });
        }

        // Sort by path for deterministic output
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }
}
