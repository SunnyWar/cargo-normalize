use crate::config::NormalizeConfig;
use syn::Item;

use super::{CompactGroup, NormalizedFile};

pub(super) fn render_segments(normalized: NormalizedFile, config: &NormalizeConfig) -> String {
    let mut out = String::new();
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

    let compact_groups: Vec<Option<CompactGroup>> = normalized
        .items
        .iter()
        .map(|segment| compact_group_for_item(&segment.item, config))
        .collect();

    let total = normalized.items.len();
    for (idx, segment) in normalized.items.into_iter().enumerate() {
        if !segment.leading_comments.is_empty() {
            for line in segment.leading_comments {
                out.push_str(&line);
                out.push('\n');
            }
        }

        out.push_str(segment.source.trim_end());
        let is_last = idx + 1 == total;
        if !is_last {
            if compact_groups[idx].is_some() && compact_groups[idx] == compact_groups[idx + 1] {
                out.push('\n');
            } else {
                out.push_str("\n\n");
            }
        } else {
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
