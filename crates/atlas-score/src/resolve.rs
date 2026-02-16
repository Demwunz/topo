use crate::pagerank::ImportGraph;
use atlas_core::Language;
use std::collections::HashMap;
use std::path::Path;

/// Build a lookup from file stem and directory names to file paths.
///
/// For `src/auth/handler.rs` we index: `"handler"` → `["src/auth/handler.rs"]`,
/// and for `src/auth/mod.rs` we also index `"auth"` → `["src/auth/mod.rs"]`.
pub fn build_file_index(paths: &[&str]) -> HashMap<String, Vec<String>> {
    let mut index: HashMap<String, Vec<String>> = HashMap::new();

    for &path in paths {
        let p = Path::new(path);

        // Index by file stem (e.g., "handler" from "src/auth/handler.rs")
        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
            let stem_lower = stem.to_lowercase();
            index
                .entry(stem_lower.clone())
                .or_default()
                .push(path.to_string());

            // For mod.rs / index.js / __init__.py, also index the parent directory name
            if matches!(stem, "mod" | "index" | "__init__")
                && let Some(parent) = p
                    .parent()
                    .and_then(|d| d.file_name())
                    .and_then(|n| n.to_str())
            {
                index
                    .entry(parent.to_lowercase())
                    .or_default()
                    .push(path.to_string());
            }
        }
    }

    index
}

/// Resolve a single raw import to candidate repo file paths.
///
/// Returns an empty vec for external/unresolved imports (no matching repo file).
pub fn resolve_import(
    raw_import: &str,
    importing_file: &str,
    language: Language,
    file_index: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    let candidates = match language {
        Language::Rust => resolve_rust(raw_import, file_index),
        Language::JavaScript | Language::TypeScript => {
            resolve_js(raw_import, importing_file, file_index)
        }
        Language::Python => resolve_python(raw_import, importing_file, file_index),
        Language::Go => resolve_go(raw_import, file_index),
        Language::Java => resolve_java(raw_import, file_index),
        _ => Vec::new(),
    };

    // Filter out self-imports
    candidates
        .into_iter()
        .filter(|c| c != importing_file)
        .collect()
}

/// Build an ImportGraph from files with their content.
///
/// 1. Extracts raw imports from each file
/// 2. Builds a file stem index for resolution
/// 3. Resolves imports to repo file paths
/// 4. Constructs the directed graph
pub fn build_import_graph(
    file_imports: &[(String, Language, Vec<String>)],
    all_paths: &[&str],
) -> ImportGraph {
    let file_index = build_file_index(all_paths);
    let mut graph = ImportGraph::new();

    // Add all files as nodes
    for path in all_paths {
        graph.add_node(path);
    }

    // Resolve imports and add edges
    for (path, language, raw_imports) in file_imports {
        for raw in raw_imports {
            let resolved = resolve_import(raw, path, *language, &file_index);
            for target in resolved {
                graph.add_edge(path, &target);
            }
        }
    }

    graph
}

/// Rust: match module name against file stems.
/// e.g., `"auth"` matches `src/auth.rs` or `src/auth/mod.rs`.
fn resolve_rust(module: &str, file_index: &HashMap<String, Vec<String>>) -> Vec<String> {
    file_index
        .get(&module.to_lowercase())
        .cloned()
        .unwrap_or_default()
}

/// JS/TS: relative paths resolve relative to importing file; bare specifiers match stems.
fn resolve_js(
    import_path: &str,
    importing_file: &str,
    file_index: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    if import_path.starts_with('.') {
        // Relative import: resolve relative to importing file's directory
        let base = Path::new(importing_file).parent().unwrap_or(Path::new(""));
        let resolved = base.join(import_path);

        // Extract the stem from the resolved path
        let stem = resolved.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        if stem.is_empty() {
            return Vec::new();
        }

        // Match against file index, but prefer files in the resolved directory
        let stem_lower = stem.to_lowercase();
        let candidates = file_index.get(&stem_lower).cloned().unwrap_or_default();

        // Try to narrow to files near the expected path
        let resolved_str = resolved.to_string_lossy();
        let near: Vec<String> = candidates
            .iter()
            .filter(|c| {
                let c_no_ext = Path::new(c.as_str())
                    .with_extension("")
                    .to_string_lossy()
                    .into_owned();
                c_no_ext == resolved_str.as_ref() || c.starts_with(resolved_str.as_ref())
            })
            .cloned()
            .collect();

        if near.is_empty() { candidates } else { near }
    } else {
        // Bare specifier: match last path segment against file stems
        let segment = import_path.rsplit('/').next().unwrap_or(import_path);
        file_index
            .get(&segment.to_lowercase())
            .cloned()
            .unwrap_or_default()
    }
}

/// Python: relative imports resolve relative to importing file; absolute match stems.
fn resolve_python(
    import_path: &str,
    importing_file: &str,
    file_index: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    if import_path.starts_with('.') {
        // Relative import
        let module = import_path.trim_start_matches('.');
        if module.is_empty() {
            // `from . import X` — try the parent package's __init__.py
            let parent = Path::new(importing_file)
                .parent()
                .and_then(|d| d.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("");
            return file_index
                .get(&parent.to_lowercase())
                .cloned()
                .unwrap_or_default();
        }
        // e.g., `.utils` — match "utils" against stems
        let parts: Vec<&str> = module.split('.').collect();
        let last = parts.last().copied().unwrap_or("");
        file_index
            .get(&last.to_lowercase())
            .cloned()
            .unwrap_or_default()
    } else {
        // Absolute import: match first/last segment against stems
        let parts: Vec<&str> = import_path.split('.').collect();
        // Try last segment first (more specific), then first
        for segment in [parts.last().copied(), parts.first().copied()]
            .iter()
            .flatten()
        {
            let candidates = file_index
                .get(&segment.to_lowercase())
                .cloned()
                .unwrap_or_default();
            if !candidates.is_empty() {
                return candidates;
            }
        }
        Vec::new()
    }
}

/// Go: match last path segment against file stems.
fn resolve_go(import_path: &str, file_index: &HashMap<String, Vec<String>>) -> Vec<String> {
    let segment = import_path.rsplit('/').next().unwrap_or(import_path);
    file_index
        .get(&segment.to_lowercase())
        .cloned()
        .unwrap_or_default()
}

/// Java: match last segment of qualified name against file stems.
fn resolve_java(import_path: &str, file_index: &HashMap<String, Vec<String>>) -> Vec<String> {
    // Handle wildcard imports: com.example.* → match "example"
    let path = import_path.trim_end_matches(".*");
    let segment = path.rsplit('.').next().unwrap_or(path);
    file_index
        .get(&segment.to_lowercase())
        .cloned()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_file_index_basic() {
        let paths = vec![
            "src/auth.rs",
            "src/auth/mod.rs",
            "src/handler.rs",
            "src/lib.rs",
        ];
        let idx = build_file_index(&paths);

        assert!(idx["auth"].contains(&"src/auth.rs".to_string()));
        assert!(idx["auth"].contains(&"src/auth/mod.rs".to_string()));
        assert!(idx["handler"].contains(&"src/handler.rs".to_string()));
        assert!(idx["lib"].contains(&"src/lib.rs".to_string()));
    }

    #[test]
    fn build_file_index_mod_indexes_parent() {
        let paths = vec!["src/auth/mod.rs"];
        let idx = build_file_index(&paths);

        // "mod" stem entry
        assert!(idx["mod"].contains(&"src/auth/mod.rs".to_string()));
        // parent directory "auth" entry
        assert!(idx["auth"].contains(&"src/auth/mod.rs".to_string()));
    }

    #[test]
    fn build_file_index_js_index() {
        let paths = vec!["src/components/index.ts"];
        let idx = build_file_index(&paths);

        assert!(idx["index"].contains(&"src/components/index.ts".to_string()));
        assert!(idx["components"].contains(&"src/components/index.ts".to_string()));
    }

    #[test]
    fn resolve_rust_module() {
        let paths = vec!["src/auth.rs", "src/db.rs"];
        let idx = build_file_index(&paths);

        let result = resolve_import("auth", "src/main.rs", Language::Rust, &idx);
        assert_eq!(result, vec!["src/auth.rs".to_string()]);
    }

    #[test]
    fn resolve_js_relative() {
        let paths = vec!["src/utils.ts", "src/handler.ts"];
        let idx = build_file_index(&paths);

        let result = resolve_import("./utils", "src/handler.ts", Language::TypeScript, &idx);
        assert!(result.contains(&"src/utils.ts".to_string()));
    }

    #[test]
    fn resolve_js_bare_specifier_no_match() {
        let paths = vec!["src/handler.ts"];
        let idx = build_file_index(&paths);

        // "react" has no matching file — external dependency
        let result = resolve_import("react", "src/handler.ts", Language::JavaScript, &idx);
        assert!(result.is_empty());
    }

    #[test]
    fn resolve_python_relative() {
        let paths = vec!["src/utils.py", "src/main.py"];
        let idx = build_file_index(&paths);

        let result = resolve_import(".utils", "src/main.py", Language::Python, &idx);
        assert!(result.contains(&"src/utils.py".to_string()));
    }

    #[test]
    fn resolve_go_last_segment() {
        let paths = vec!["pkg/http/handler.go"];
        let idx = build_file_index(&paths);

        let result = resolve_import("myapp/pkg/http", "cmd/main.go", Language::Go, &idx);
        // "http" last segment doesn't match "handler" stem — no match expected
        assert!(result.is_empty());

        // But matching the actual file stem works
        let result2 = resolve_import("myapp/pkg/handler", "cmd/main.go", Language::Go, &idx);
        assert!(result2.contains(&"pkg/http/handler.go".to_string()));
    }

    #[test]
    fn resolve_java_qualified() {
        let paths = vec!["src/main/java/AuthService.java"];
        let idx = build_file_index(&paths);

        let result = resolve_import(
            "com.example.auth.AuthService",
            "src/main/java/App.java",
            Language::Java,
            &idx,
        );
        assert!(result.contains(&"src/main/java/AuthService.java".to_string()));
    }

    #[test]
    fn resolve_java_wildcard() {
        let paths = vec!["src/main/java/Utils.java"];
        let idx = build_file_index(&paths);

        let result = resolve_import(
            "com.example.utils.*",
            "src/main/java/App.java",
            Language::Java,
            &idx,
        );
        assert!(result.contains(&"src/main/java/Utils.java".to_string()));
    }

    #[test]
    fn resolve_filters_self_import() {
        let paths = vec!["src/auth.rs"];
        let idx = build_file_index(&paths);

        let result = resolve_import("auth", "src/auth.rs", Language::Rust, &idx);
        assert!(result.is_empty());
    }

    #[test]
    fn build_import_graph_basic() {
        let all_paths = vec!["src/main.rs", "src/auth.rs", "src/utils.rs"];
        let file_imports = vec![
            (
                "src/main.rs".to_string(),
                Language::Rust,
                vec!["auth".to_string()],
            ),
            (
                "src/auth.rs".to_string(),
                Language::Rust,
                vec!["utils".to_string()],
            ),
        ];

        let graph = build_import_graph(&file_imports, &all_paths);

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);

        // utils should have highest PageRank (most downstream)
        let scores = graph.normalized_pagerank();
        assert!(scores["src/utils.rs"] > scores["src/main.rs"]);
    }

    #[test]
    fn build_import_graph_external_imports_ignored() {
        let all_paths = vec!["src/main.rs"];
        let file_imports = vec![(
            "src/main.rs".to_string(),
            Language::Rust,
            vec!["serde".to_string(), "tokio".to_string()],
        )];

        let graph = build_import_graph(&file_imports, &all_paths);

        // External imports should not create edges
        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn build_import_graph_diamond() {
        // main → auth, main → db, auth → utils, db → utils
        let all_paths = vec!["src/main.rs", "src/auth.rs", "src/db.rs", "src/utils.rs"];
        let file_imports = vec![
            (
                "src/main.rs".to_string(),
                Language::Rust,
                vec!["auth".to_string(), "db".to_string()],
            ),
            (
                "src/auth.rs".to_string(),
                Language::Rust,
                vec!["utils".to_string()],
            ),
            (
                "src/db.rs".to_string(),
                Language::Rust,
                vec!["utils".to_string()],
            ),
        ];

        let graph = build_import_graph(&file_imports, &all_paths);
        let scores = graph.normalized_pagerank();

        // utils should have the highest PageRank (imported by auth + db)
        assert_eq!(scores["src/utils.rs"], 1.0);
        assert!(scores["src/utils.rs"] > scores["src/main.rs"]);
    }
}
