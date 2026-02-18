use std::io::Write;
use topo_core::ScoredFile;

/// Writes scored files in compact single-line format for hook injection.
///
/// Output format: `path (role, Ntok, score)`
/// Example: `src/auth.rs (impl, 2494tok, 7.01)`
pub struct CompactWriter;

impl CompactWriter {
    pub fn new() -> Self {
        Self
    }

    /// Render scored files as compact single-line entries.
    pub fn render(&self, files: &[ScoredFile]) -> String {
        let mut buf = Vec::new();
        self.write_to(&mut buf, files).expect("write to Vec failed");
        String::from_utf8(buf).expect("compact output is valid UTF-8")
    }

    /// Write compact output to a writer.
    pub fn write_to(&self, writer: &mut dyn Write, files: &[ScoredFile]) -> std::io::Result<()> {
        for file in files {
            writeln!(
                writer,
                "{} ({}, {}tok, {:.2})",
                file.path,
                file.role.as_str(),
                file.tokens,
                file.score,
            )?;
        }
        Ok(())
    }
}

impl Default for CompactWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use topo_core::{FileRole, Language, ScoredFile, SignalBreakdown};

    fn sample_files() -> Vec<ScoredFile> {
        vec![
            ScoredFile {
                path: "src/auth.rs".to_string(),
                score: 7.01,
                signals: SignalBreakdown::default(),
                tokens: 2494,
                language: Language::Rust,
                role: FileRole::Implementation,
            },
            ScoredFile {
                path: "src/commands/init.rs".to_string(),
                score: 6.92,
                signals: SignalBreakdown::default(),
                tokens: 2635,
                language: Language::Rust,
                role: FileRole::Implementation,
            },
            ScoredFile {
                path: "README.md".to_string(),
                score: 6.54,
                signals: SignalBreakdown::default(),
                tokens: 128,
                language: Language::Markdown,
                role: FileRole::Documentation,
            },
        ]
    }

    #[test]
    fn compact_output_one_line_per_file() {
        let writer = CompactWriter::new();
        let output = writer.render(&sample_files());
        let lines: Vec<&str> = output.trim().lines().collect();
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn compact_output_format() {
        let writer = CompactWriter::new();
        let output = writer.render(&sample_files());
        let first_line = output.lines().next().unwrap();
        assert_eq!(first_line, "src/auth.rs (impl, 2494tok, 7.01)");
    }

    #[test]
    fn compact_output_includes_role() {
        let writer = CompactWriter::new();
        let output = writer.render(&sample_files());
        assert!(output.contains("(docs,"));
        assert!(output.contains("(impl,"));
    }

    #[test]
    fn compact_empty_files() {
        let writer = CompactWriter::new();
        let output = writer.render(&[]);
        assert!(output.is_empty());
    }
}
