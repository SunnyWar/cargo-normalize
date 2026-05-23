use crate::config::NormalizeConfig;
use syn::Item;

use super::{CompactGroup, NormalizedFile};

use std::path::Path;

pub(super) fn render_segments(
    normalized: NormalizedFile,
    config: &NormalizeConfig,
    file_path: Option<&Path>,
) -> String {
    let mut out = String::new();

    // Insert relative path comment if enabled and path is provided via config

    if config.relative_path_comment {
        if let Some(path) = file_path {
            // Compute relative path to workspace root (current directory)
            use std::path::PathBuf;
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let rel = path.strip_prefix(&cwd).unwrap_or(path);
            out.push_str(&format!("// {}\n", rel.display()));
        } else {
            out.push_str("// [RELATIVE_PATH_UNKNOWN]\n");
        }
    }

    if normalized.shebang.is_some() || !normalized.attrs.is_empty() {
        let preamble = prettyplease::unparse(&syn::File {
            shebang: normalized.shebang,
            attrs: normalized.attrs,
            items: Vec::new(),
        });
        let preamble = normalize_crate_attribute_layout(&preamble);
        if !preamble.trim().is_empty() {
            out.push_str(preamble.trim_end());
            out.push_str("\n\n");
        }
    }

    // --- Emit all mod items, blank line, all use items, blank line, then the rest ---
    let mut mods = Vec::new();
    let mut uses = Vec::new();
    let mut others = Vec::new();

    for segment in &normalized.items {
        match &segment.item {
            Item::Mod(_) => mods.push(segment),
            Item::Use(_) => uses.push(segment),
            _ => others.push(segment),
        }
    }

    if !mods.is_empty() {
        for segment in &mods {
            if !segment.module_doc_comments.is_empty() {
                for line in &segment.module_doc_comments {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            if !segment.leading_comments.is_empty() {
                for line in &segment.leading_comments {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            out.push_str(segment.source.trim_end());
            out.push('\n');
        }
    }
    if !mods.is_empty() && !uses.is_empty() {
        out.push('\n');
    }
    if !uses.is_empty() {
        for segment in &uses {
            if !segment.module_doc_comments.is_empty() {
                for line in &segment.module_doc_comments {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            if !segment.leading_comments.is_empty() {
                for line in &segment.leading_comments {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            out.push_str(segment.source.trim_end());
            out.push('\n');
        }
    }
    if (!mods.is_empty() || !uses.is_empty()) && !others.is_empty() {
        out.push('\n');
    }
    if !others.is_empty() {
        for segment in &others {
            if !segment.module_doc_comments.is_empty() {
                for line in &segment.module_doc_comments {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            if !segment.leading_comments.is_empty() {
                for line in &segment.leading_comments {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            out.push_str(segment.source.trim_end());
            out.push('\n');
        }
    }

    out.truncate(out.trim_end_matches('\n').len());
    out.push('\n');
    out
}

fn normalize_crate_attribute_layout(preamble: &str) -> String {
    preamble
        .replace("] #![", "]\n#![")
        .replace("]#![", "]\n#![")
}

fn compact_group_for_item(item: &Item, config: &NormalizeConfig) -> Option<CompactGroup> {
    match item {
        Item::Use(_) if config.compact_use_block => Some(CompactGroup::Use),
        Item::Const(_) | Item::Static(_) if config.compact_const_block => Some(CompactGroup::Const),
        Item::Mod(_) if config.compact_mod_block => Some(CompactGroup::Mod),
        _ => None,
    }
}
