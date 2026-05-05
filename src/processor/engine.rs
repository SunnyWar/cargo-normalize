use crate::config::NormalizeConfig;
use std::fs;
use std::path::Path;

use super::io::{atomic_write, collect_rust_files, print_diff};
use super::model::Normalizer;
use super::render::render_segments;
use super::text::{
    differs_only_by_whitespace, normalize_function_spacing, promote_leading_item_comments,
    restore_promoted_comment_style,
};

#[derive(Debug, Clone, Default)]
pub struct ProcessSummary {
    pub scanned: usize,
    pub changed: usize,
}

#[derive(Debug, Clone)]
pub struct Processor {
    check: bool,
    config: NormalizeConfig,
}

impl Processor {
    pub fn new(check: bool, config: NormalizeConfig) -> Self {
        Self { check, config }
    }

    pub fn run(&self, root: &Path) -> Result<ProcessSummary, String> {
        let files = collect_rust_files(root)?;
        let mut summary = ProcessSummary::default();
        for file in files {
            summary.scanned += 1;
            let changed = self.process_file(&file)?;
            if changed {
                summary.changed += 1;
            }
        }
        Ok(summary)
    }

    fn process_file(&self, path: &Path) -> Result<bool, String> {
        let original = fs::read_to_string(path)
            .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
        let promoted_comments = promote_leading_item_comments(&original);
        let parsed = syn::parse_file(&promoted_comments)
            .map_err(|err| format!("Failed to parse {}: {err}", path.display()))?;
        let normalized = Normalizer::new(parsed, &promoted_comments).normalize(&self.config);
        let rendered = normalize_function_spacing(&restore_promoted_comment_style(
            &render_segments(normalized, &self.config),
        ));

        if original == rendered {
            return Ok(false);
        }
        if differs_only_by_whitespace(&original, &rendered) {
            return Ok(false);
        }
        if self.check {
            print_diff(path, &original, &rendered);
            return Ok(true);
        }

        atomic_write(path, rendered.as_bytes())?;
        println!("normalized {}", path.display());
        Ok(true)
    }
}
