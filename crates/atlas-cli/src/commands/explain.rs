use crate::Cli;
use anyhow::Result;
use atlas_scanner::BundleBuilder;
use atlas_score::HybridScorer;

pub fn run(cli: &Cli, task: &str, top: usize) -> Result<()> {
    let root = cli.repo_root()?;
    let bundle = BundleBuilder::new(&root).build()?;

    let scorer = HybridScorer::new(task);
    let scored = scorer.score(&bundle.files);

    let display_count = top.min(scored.len());
    let results = &scored[..display_count];

    match cli.effective_format() {
        crate::OutputFormat::Json | crate::OutputFormat::Jsonl => {
            let output: Vec<serde_json::Value> = results
                .iter()
                .map(|f| {
                    serde_json::json!({
                        "path": f.path,
                        "score": f.score,
                        "signals": {
                            "bm25f": f.signals.bm25f,
                            "heuristic": f.signals.heuristic,
                            "pagerank": f.signals.pagerank,
                            "git_recency": f.signals.git_recency,
                        },
                        "tokens": f.tokens,
                        "language": f.language.as_str(),
                        "role": f.role.as_str(),
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        _ => {
            println!("Score breakdown for query: \"{task}\"");
            println!("Showing top {display_count} of {} files\n", scored.len());

            println!(
                "{:<50} {:>8} {:>8} {:>8} {:>8}",
                "PATH", "TOTAL", "BM25F", "HEUR", "ROLE"
            );
            println!("{}", "-".repeat(86));

            for f in results {
                println!(
                    "{:<50} {:>8.4} {:>8.4} {:>8.4} {:>8}",
                    truncate(&f.path, 50),
                    f.score,
                    f.signals.bm25f,
                    f.signals.heuristic,
                    f.role.as_str(),
                );
            }
        }
    }

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("...{}", &s[s.len() - max + 3..])
    }
}
