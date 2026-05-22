use crate::config::{MoveSelection, NormalizeConfig};
use std::fs;
use std::path::{Path, PathBuf};

use super::io::{atomic_write, collect_rust_files};
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
    pub changed_files: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Processor {
    check: bool,
    move_selection: MoveSelection,
    config: NormalizeConfig,
}

impl Processor {
    pub fn new(check: bool, move_selection: MoveSelection, config: NormalizeConfig) -> Self {
        Self {
            check,
            move_selection,
            config,
        }
    }

    pub fn run(&self, root: &Path) -> Result<ProcessSummary, String> {
        let files = collect_rust_files(root)?;
        let mut summary = ProcessSummary::default();
        for file in files {
            summary.scanned += 1;
            let changed = self.process_file(&file)?;
            if changed {
                summary.changed += 1;
                summary.changed_files.push(file);
            }
        }
        Ok(summary)
    }

    fn process_file(&self, path: &Path) -> Result<bool, String> {
        if !self.move_selection.all && self.move_selection.features.is_empty() {
            return Ok(false);
        }

        let original = fs::read_to_string(path)
            .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
        let promoted_comments = promote_leading_item_comments(&original);
        let parsed = syn::parse_file(&promoted_comments)
            .map_err(|err| format!("Failed to parse {}: {err}", path.display()))?;
        let normalized = Normalizer::new(parsed, &promoted_comments)
            .normalize(&self.config, &self.move_selection);
        let rendered = normalize_function_spacing(&restore_promoted_comment_style(
            &render_segments(normalized, &self.config, Some(path)),
        ));

        if original == rendered {
            return Ok(false);
        }
        if differs_only_by_whitespace(&original, &rendered) {
            return Ok(false);
        }
        if self.check {
            return Ok(true);
        }

        atomic_write(path, rendered.as_bytes())?;
        println!("normalized {}", path.display());
        Ok(true)
    }
}
