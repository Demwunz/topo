use crate::Cli;
use anyhow::Result;
use atlas_index::IndexBuilder;
use atlas_scanner::BundleBuilder;

pub fn run(cli: &Cli, deep: bool, force: bool) -> Result<()> {
    let root = cli.repo_root()?;

    if !cli.is_quiet() {
        eprintln!(
            "Indexing {} (mode: {})...",
            root.display(),
            if deep { "deep" } else { "shallow" }
        );
    }

    // Scan the repository
    let bundle = BundleBuilder::new(&root).build()?;

    if !cli.is_quiet() {
        eprintln!(
            "Scanned {} files (fingerprint: {})",
            bundle.file_count(),
            &bundle.fingerprint[..12]
        );
    }

    if deep {
        // Load existing index (unless force rebuild)
        let existing = if force {
            None
        } else {
            atlas_index::load(&root)?
        };

        // Build fresh index
        let builder = IndexBuilder::new(&root);
        let fresh_index = builder.build(&bundle.files)?;

        // Merge with existing or use fresh
        let final_index = if let Some(existing) = existing {
            let merged = atlas_index::merge_incremental(&existing, &fresh_index);
            if !cli.is_quiet() {
                eprintln!("Incremental update: {} files indexed", merged.total_docs);
            }
            merged
        } else {
            if !cli.is_quiet() {
                eprintln!("Full index build: {} files indexed", fresh_index.total_docs);
            }
            fresh_index
        };

        atlas_index::save(&final_index, &root)?;

        if !cli.is_quiet() {
            eprintln!(
                "Index saved to {}",
                atlas_index::index_path(&root).display()
            );
        }
    }

    if !cli.is_quiet() {
        eprintln!("Done.");
    }

    Ok(())
}
