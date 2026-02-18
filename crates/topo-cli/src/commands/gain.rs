use crate::Cli;
use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::io::BufRead;

/// Stats entry from `.topo/stats.jsonl`.
#[derive(serde::Deserialize)]
struct StatsEntry {
    #[allow(dead_code)]
    timestamp: String,
    event: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    files_suggested: Option<usize>,
    #[serde(default)]
    tokens_suggested: Option<u64>,
}

pub fn run(cli: &Cli) -> Result<()> {
    let root = cli.repo_root()?;
    let stats_path = root.join(".topo/stats.jsonl");

    if !stats_path.exists() {
        println!("No topo stats found.");
        println!();
        println!("Stats are collected automatically when Claude Code hooks are installed.");
        println!("Run `topo init` to set up hooks.");
        return Ok(());
    }

    let file = fs::File::open(&stats_path)?;
    let reader = std::io::BufReader::new(file);

    let mut sessions = 0u64;
    let mut total_files_suggested = 0u64;
    let mut total_tokens_suggested = 0u64;
    let mut files_opened: HashSet<String> = HashSet::new();
    let mut suggestion_events = 0u64;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let entry: StatsEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => continue, // skip malformed lines
        };

        match entry.event.as_str() {
            "session_start" => {
                sessions += 1;
            }
            "topo_query" => {
                suggestion_events += 1;
                if let Some(n) = entry.files_suggested {
                    total_files_suggested += n as u64;
                }
                if let Some(t) = entry.tokens_suggested {
                    total_tokens_suggested += t;
                }
            }
            "file_read" => {
                if let Some(path) = entry.path {
                    files_opened.insert(path);
                }
            }
            _ => {}
        }
    }

    match cli.effective_format() {
        crate::OutputFormat::Json | crate::OutputFormat::Jsonl => {
            let output = serde_json::json!({
                "sessions": sessions,
                "suggestion_events": suggestion_events,
                "files_suggested": total_files_suggested,
                "files_opened": files_opened.len(),
                "tokens_suggested": total_tokens_suggested,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        _ => {
            println!("Topo context savings:");
            println!("  Sessions:         {sessions}");
            println!("  Suggestions:      {suggestion_events}");
            println!("  Files suggested:  {total_files_suggested}");
            println!("  Files opened:     {}", files_opened.len());
            println!("  Tokens suggested: {total_tokens_suggested}");
            if suggestion_events > 0 {
                let avg = total_files_suggested as f64 / suggestion_events as f64;
                println!("  Avg files/query:  {avg:.1}");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stats_entries() {
        let json = r#"{"timestamp":"2025-01-01T00:00:00Z","event":"topo_query","files_suggested":10,"tokens_suggested":5000}"#;
        let entry: StatsEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.event, "topo_query");
        assert_eq!(entry.files_suggested, Some(10));
        assert_eq!(entry.tokens_suggested, Some(5000));
    }

    #[test]
    fn parses_file_read_entry() {
        let json =
            r#"{"timestamp":"2025-01-01T00:00:00Z","event":"file_read","path":"src/main.rs"}"#;
        let entry: StatsEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.event, "file_read");
        assert_eq!(entry.path, Some("src/main.rs".to_string()));
    }

    #[test]
    fn parses_session_start_entry() {
        let json = r#"{"timestamp":"2025-01-01T00:00:00Z","event":"session_start"}"#;
        let entry: StatsEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.event, "session_start");
    }
}
