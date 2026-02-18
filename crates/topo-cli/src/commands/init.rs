use crate::Cli;
use anyhow::Result;
use std::fs;
use std::path::Path;

const AGENTS_MD: &str = include_str!("../../templates/AGENTS.md");
const CURSOR_TOPO_MD: &str = include_str!("../../templates/cursor-topo.md");
const COPILOT_INSTRUCTIONS_MD: &str = include_str!("../../templates/copilot-instructions.md");
const CLAUDE_MD_SECTION: &str = include_str!("../../templates/claude-md-section.md");
const TOPO_CONTEXT_SH: &str = include_str!("../../templates/topo-context.sh");
const TOPO_HINT_SH: &str = include_str!("../../templates/topo-hint.sh");
const TOPO_TRACK_SH: &str = include_str!("../../templates/topo-track.sh");

enum WriteResult {
    Created,
    Skipped,
}

fn write_template(path: &Path, content: &str, force: bool) -> Result<WriteResult> {
    if path.exists() && !force {
        return Ok(WriteResult::Skipped);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(WriteResult::Created)
}

const TOPO_START: &str = "<!-- topo:start -->";
const TOPO_END: &str = "<!-- topo:end -->";

fn inject_claude_md(path: &Path, section: &str, force: bool) -> Result<WriteResult> {
    let content = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };

    if let Some(start) = content.find(TOPO_START) {
        if !force {
            return Ok(WriteResult::Skipped);
        }
        // Replace existing section (inclusive of markers)
        let end = content[start..]
            .find(TOPO_END)
            .map(|i| start + i + TOPO_END.len())
            .unwrap_or(content.len());
        let mut new_content = String::with_capacity(content.len());
        new_content.push_str(&content[..start]);
        new_content.push_str(section.trim_end());
        // Preserve anything after the old end marker
        let after = &content[end..];
        if !after.is_empty() {
            new_content.push_str(after);
        } else {
            new_content.push('\n');
        }
        fs::write(path, new_content)?;
    } else if content.is_empty() {
        // New file — just write the section
        fs::write(path, section)?;
    } else {
        // Existing file without markers — append
        let mut new_content = content;
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push('\n');
        new_content.push_str(section);
        fs::write(path, new_content)?;
    }

    Ok(WriteResult::Created)
}

/// Write a hook script, creating parent dirs and setting executable permissions.
fn write_hook(path: &Path, content: &str, force: bool) -> Result<WriteResult> {
    if path.exists() && !force {
        return Ok(WriteResult::Skipped);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;

    // Set executable permission on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(path, perms)?;
    }

    Ok(WriteResult::Created)
}

/// Patch `.claude/settings.json` to register topo hooks.
/// Merges hook entries into existing settings without destroying user config.
fn patch_claude_settings(root: &Path, force: bool) -> Result<WriteResult> {
    let settings_path = root.join(".claude/settings.json");
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Check if hooks are already configured
    if !force
        && let Some(hooks) = settings.get("hooks")
        && (hooks.get("UserPromptSubmit").is_some() || hooks.get("PreToolUse").is_some())
    {
        return Ok(WriteResult::Skipped);
    }

    // Build the hook configuration
    let topo_hooks = serde_json::json!({
        "UserPromptSubmit": [{
            "hooks": [{
                "type": "command",
                "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/topo-context.sh",
                "timeout": 15
            }]
        }],
        "PreToolUse": [{
            "matcher": "Glob|Grep",
            "hooks": [{
                "type": "command",
                "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/topo-hint.sh",
                "timeout": 10
            }]
        }],
        "PostToolUse": [{
            "matcher": "Read",
            "hooks": [{
                "type": "command",
                "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/topo-track.sh",
                "timeout": 5
            }]
        }]
    });

    // Merge into existing settings
    if let Some(existing_hooks) = settings.get_mut("hooks") {
        if let Some(obj) = existing_hooks.as_object_mut() {
            for (key, value) in topo_hooks.as_object().unwrap() {
                obj.insert(key.clone(), value.clone());
            }
        }
    } else {
        settings["hooks"] = topo_hooks;
    }

    // Write back
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let formatted = serde_json::to_string_pretty(&settings)?;
    fs::write(&settings_path, formatted + "\n")?;

    Ok(WriteResult::Created)
}

fn check_topo_on_path() {
    let cmd = if cfg!(windows) {
        std::process::Command::new("where.exe").arg("topo").output()
    } else {
        std::process::Command::new("which").arg("topo").output()
    };

    match cmd {
        Ok(output) if output.status.success() => {
            let path = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or_default()
                .to_string();
            println!("topo found on PATH: {path}");
            println!("Your AI assistant can now run `topo quick \"task\"` via shell.");
        }
        _ => {
            println!("Warning: topo is not on PATH.");
            println!("Install it so your AI assistant can run `topo quick \"task\"`:");
            println!();
            if cfg!(target_os = "macos") {
                println!("  brew install demwunz/tap/topo    # Homebrew");
            }
            println!("  cargo install topo-cli            # Cargo");
            println!("  curl -fsSL https://topo.sh | sh   # Shell script");
        }
    }

    println!();
    println!("Optional: for tools without shell access, topo also runs as an MCP server.");
    println!("See https://github.com/demwunz/topo#mcp for setup instructions.");
}

pub fn run(cli: &Cli, force: bool, hooks: bool) -> Result<()> {
    let root = cli.repo_root()?;
    let quiet = cli.is_quiet();

    // AGENTS.md at repo root
    let agents_path = root.join("AGENTS.md");
    match write_template(&agents_path, AGENTS_MD, force)? {
        WriteResult::Created => {
            if !quiet {
                println!("  Created AGENTS.md");
            }
        }
        WriteResult::Skipped => {
            if !quiet {
                println!("  Skipped AGENTS.md (already exists, use --force to overwrite)");
            }
        }
    }

    // .cursor/rules/topo.md
    let cursor_path = root.join(".cursor/rules/topo.md");
    match write_template(&cursor_path, CURSOR_TOPO_MD, force)? {
        WriteResult::Created => {
            if !quiet {
                println!("  Created .cursor/rules/topo.md");
            }
        }
        WriteResult::Skipped => {
            if !quiet {
                println!(
                    "  Skipped .cursor/rules/topo.md (already exists, use --force to overwrite)"
                );
            }
        }
    }

    // .github/copilot-instructions.md (only if .github/ exists)
    let github_dir = root.join(".github");
    if github_dir.is_dir() {
        let copilot_path = github_dir.join("copilot-instructions.md");
        match write_template(&copilot_path, COPILOT_INSTRUCTIONS_MD, force)? {
            WriteResult::Created => {
                if !quiet {
                    println!("  Created .github/copilot-instructions.md");
                }
            }
            WriteResult::Skipped => {
                if !quiet {
                    println!(
                        "  Skipped .github/copilot-instructions.md (already exists, use --force to overwrite)"
                    );
                }
            }
        }
    } else if !quiet {
        println!("  Skipped .github/copilot-instructions.md (no .github/ directory)");
    }

    // CLAUDE.md — inject topo section (never overwrite user content)
    let claude_path = root.join("CLAUDE.md");
    match inject_claude_md(&claude_path, CLAUDE_MD_SECTION, force)? {
        WriteResult::Created => {
            if !quiet {
                println!("  Created CLAUDE.md (topo section)");
            }
        }
        WriteResult::Skipped => {
            if !quiet {
                println!(
                    "  Skipped CLAUDE.md (topo section already present, use --force to update)"
                );
            }
        }
    }

    // Claude Code hooks (--hooks, on by default)
    if hooks {
        if !quiet {
            println!();
            println!("Claude Code hooks:");
        }

        let hooks_dir = root.join(".claude/hooks");
        let context_path = hooks_dir.join("topo-context.sh");
        match write_hook(&context_path, TOPO_CONTEXT_SH, force)? {
            WriteResult::Created => {
                if !quiet {
                    println!("  Created .claude/hooks/topo-context.sh");
                }
            }
            WriteResult::Skipped => {
                if !quiet {
                    println!(
                        "  Skipped .claude/hooks/topo-context.sh (already exists, use --force to overwrite)"
                    );
                }
            }
        }

        let hint_path = hooks_dir.join("topo-hint.sh");
        match write_hook(&hint_path, TOPO_HINT_SH, force)? {
            WriteResult::Created => {
                if !quiet {
                    println!("  Created .claude/hooks/topo-hint.sh");
                }
            }
            WriteResult::Skipped => {
                if !quiet {
                    println!(
                        "  Skipped .claude/hooks/topo-hint.sh (already exists, use --force to overwrite)"
                    );
                }
            }
        }

        let track_path = hooks_dir.join("topo-track.sh");
        match write_hook(&track_path, TOPO_TRACK_SH, force)? {
            WriteResult::Created => {
                if !quiet {
                    println!("  Created .claude/hooks/topo-track.sh");
                }
            }
            WriteResult::Skipped => {
                if !quiet {
                    println!(
                        "  Skipped .claude/hooks/topo-track.sh (already exists, use --force to overwrite)"
                    );
                }
            }
        }

        match patch_claude_settings(&root, force)? {
            WriteResult::Created => {
                if !quiet {
                    println!("  Patched .claude/settings.json (hook registration)");
                }
            }
            WriteResult::Skipped => {
                if !quiet {
                    println!(
                        "  Skipped .claude/settings.json (hooks already registered, use --force to update)"
                    );
                }
            }
        }
    }

    if !quiet {
        println!();
        check_topo_on_path();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn templates_are_non_empty() {
        assert!(!AGENTS_MD.is_empty());
        assert!(!CURSOR_TOPO_MD.is_empty());
        assert!(!COPILOT_INSTRUCTIONS_MD.is_empty());
        assert!(!TOPO_CONTEXT_SH.is_empty());
        assert!(!TOPO_HINT_SH.is_empty());
        assert!(!TOPO_TRACK_SH.is_empty());
    }

    #[test]
    fn hook_templates_are_valid_bash() {
        assert!(TOPO_CONTEXT_SH.starts_with("#!/usr/bin/env bash"));
        assert!(TOPO_HINT_SH.starts_with("#!/usr/bin/env bash"));
        assert!(TOPO_TRACK_SH.starts_with("#!/usr/bin/env bash"));
    }

    #[test]
    fn write_hook_creates_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("hooks/test.sh");
        let result = write_hook(&path, "#!/bin/bash\necho hi", false).unwrap();
        assert!(matches!(result, WriteResult::Created));
        assert_eq!(fs::read_to_string(&path).unwrap(), "#!/bin/bash\necho hi");
    }

    #[cfg(unix)]
    #[test]
    fn write_hook_sets_executable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.sh");
        write_hook(&path, "#!/bin/bash", false).unwrap();
        let perms = fs::metadata(&path).unwrap().permissions();
        assert_eq!(perms.mode() & 0o111, 0o111); // executable bits set
    }

    #[test]
    fn patch_claude_settings_creates_new() {
        let dir = tempdir().unwrap();
        let result = patch_claude_settings(dir.path(), false).unwrap();
        assert!(matches!(result, WriteResult::Created));
        let content = fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(settings["hooks"]["UserPromptSubmit"].is_array());
        assert!(settings["hooks"]["PreToolUse"].is_array());
        assert!(settings["hooks"]["PostToolUse"].is_array());
    }

    #[test]
    fn patch_claude_settings_merges_existing() {
        let dir = tempdir().unwrap();
        let settings_dir = dir.path().join(".claude");
        fs::create_dir_all(&settings_dir).unwrap();
        fs::write(
            settings_dir.join("settings.json"),
            r#"{"allowedTools": ["bash"]}"#,
        )
        .unwrap();
        let result = patch_claude_settings(dir.path(), false).unwrap();
        assert!(matches!(result, WriteResult::Created));
        let content = fs::read_to_string(settings_dir.join("settings.json")).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
        // Preserved existing config
        assert_eq!(settings["allowedTools"][0], "bash");
        // Added hooks
        assert!(settings["hooks"]["UserPromptSubmit"].is_array());
    }

    #[test]
    fn patch_claude_settings_skips_when_present() {
        let dir = tempdir().unwrap();
        // First patch
        patch_claude_settings(dir.path(), false).unwrap();
        // Second patch should skip
        let result = patch_claude_settings(dir.path(), false).unwrap();
        assert!(matches!(result, WriteResult::Skipped));
    }

    #[test]
    fn write_template_creates_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.md");
        let result = write_template(&path, "hello", false).unwrap();
        assert!(matches!(result, WriteResult::Created));
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn write_template_skips_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.md");
        fs::write(&path, "original").unwrap();
        let result = write_template(&path, "new content", false).unwrap();
        assert!(matches!(result, WriteResult::Skipped));
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
    }

    #[test]
    fn write_template_force_overwrites() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.md");
        fs::write(&path, "original").unwrap();
        let result = write_template(&path, "new content", true).unwrap();
        assert!(matches!(result, WriteResult::Created));
        assert_eq!(fs::read_to_string(&path).unwrap(), "new content");
    }

    #[test]
    fn write_template_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a/b/c/test.md");
        let result = write_template(&path, "nested", false).unwrap();
        assert!(matches!(result, WriteResult::Created));
        assert_eq!(fs::read_to_string(&path).unwrap(), "nested");
    }

    #[test]
    fn inject_claude_md_creates_new_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        let result = inject_claude_md(&path, CLAUDE_MD_SECTION, false).unwrap();
        assert!(matches!(result, WriteResult::Created));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains(TOPO_START));
        assert!(content.contains(TOPO_END));
        assert!(content.contains("topo quick"));
    }

    #[test]
    fn inject_claude_md_appends_to_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        fs::write(&path, "# My Project\n\nExisting content.\n").unwrap();
        let result = inject_claude_md(&path, CLAUDE_MD_SECTION, false).unwrap();
        assert!(matches!(result, WriteResult::Created));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("# My Project"));
        assert!(content.contains(TOPO_START));
        assert!(content.contains(TOPO_END));
    }

    #[test]
    fn inject_claude_md_skips_when_present() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        fs::write(&path, format!("# Project\n\n{CLAUDE_MD_SECTION}")).unwrap();
        let result = inject_claude_md(&path, CLAUDE_MD_SECTION, false).unwrap();
        assert!(matches!(result, WriteResult::Skipped));
    }

    #[test]
    fn inject_claude_md_force_replaces() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        let old_section = "<!-- topo:start -->\nold content\n<!-- topo:end -->\n";
        fs::write(&path, format!("# Project\n\n{old_section}")).unwrap();
        let result = inject_claude_md(&path, CLAUDE_MD_SECTION, true).unwrap();
        assert!(matches!(result, WriteResult::Created));
        let content = fs::read_to_string(&path).unwrap();
        assert!(!content.contains("old content"));
        assert!(content.contains("topo quick"));
        assert!(content.starts_with("# Project"));
    }
}
