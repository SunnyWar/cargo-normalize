use crate::config::NormalizeConfig;
use syn::Item;

use super::{NormalizedFile};

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

    // --- Emit all non-test mod items, blank line, all use items, blank line, then the rest, then #[cfg(test)] mod tests last ---
    let mut mods = Vec::new();
    let mut uses = Vec::new();
    let mut others = Vec::new();
    let mut test_mod: Option<&super::model::ItemSegment> = None;

    for segment in &normalized.items {
        match &segment.item {
            Item::Mod(item_mod) => {
                let is_test_mod = item_mod.ident == "tests"
                    && item_mod.attrs.iter().any(|attr| {
                        if attr.path().is_ident("cfg") {
                            let mut found_test = false;
                            let _ = attr.parse_nested_meta(|meta| {
                                if meta.path.is_ident("test") {
                                    found_test = true;
                                }
                                Ok(())
                            });
                            found_test
                        } else {
                            false
                        }
                    });
                if is_test_mod {
                    test_mod = Some(segment);
                } else {
                    mods.push(segment);
                }
            }
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
    // Always emit #[cfg(test)] mod tests last, with a blank line before if there is other content
    if let Some(segment) = test_mod {
        if !out.trim().is_empty() {
            out.push('\n');
        }
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

    out.truncate(out.trim_end_matches('\n').len());
    out.push('\n');
    out
}

fn normalize_crate_attribute_layout(preamble: &str) -> String {
    preamble
        .replace("] #![", "]\n#![")
        .replace("]#![", "]\n#![")
}
