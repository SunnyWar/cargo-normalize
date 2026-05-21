use syn::Item;
use syn::spanned::Spanned;

use super::ItemSegment;

pub(super) fn segment_items(items: Vec<Item>, source: &str) -> Vec<ItemSegment> {
    let lines: Vec<&str> = source.lines().collect();
    let mut prev_end_line = 1usize;
    let mut segments = Vec::with_capacity(items.len());
    let mut pending_module_doc_comments: Vec<String> = Vec::new();

    for item in items {
        let span = item.span();
        let start_line = span.start().line.max(1);
        let end_line = span.end().line.max(start_line);
        let leading_comments = extract_leading_comments(&lines, prev_end_line, start_line);
        let module_doc_comments = if let Item::Mod(_) = &item {
            let docs = extract_module_doc_comments(&lines, prev_end_line, start_line);
            pending_module_doc_comments.append(&mut docs.clone());
            let result = pending_module_doc_comments.clone();
            pending_module_doc_comments.clear();
            result
        } else {
            // If not a mod, accumulate any found //! for the next mod
            let docs = extract_module_doc_comments(&lines, prev_end_line, start_line);
            pending_module_doc_comments.extend(docs);
            Vec::new()
        };
        let source = (start_line..=end_line)
            .filter_map(|line_no| {
                lines
                    .get(line_no.saturating_sub(1))
                    .map(|line| (*line).to_owned())
            })
            .collect::<Vec<String>>()
            .join("\n");

        segments.push(ItemSegment {
            item,
            leading_comments,
            module_doc_comments,
            source,
        });
        prev_end_line = end_line.saturating_add(1);
    }

    segments
}

fn extract_module_doc_comments(
    lines: &[&str],
    min_line: usize,
    item_start_line: usize,
) -> Vec<String> {
    if item_start_line <= 1 || lines.is_empty() {
        return Vec::new();
    }
    let mut begin = item_start_line.saturating_sub(1);
    while begin >= min_line {
        let idx = begin.saturating_sub(1);
        if idx >= lines.len() {
            break;
        }
        let trimmed = lines[idx].trim();
        if trimmed.is_empty() || is_module_doc_comment(trimmed) {
            if begin == min_line {
                break;
            }
            begin -= 1;
            continue;
        }
        break;
    }
    let first = if begin < min_line {
        min_line
    } else {
        begin.saturating_add(1)
    };
    if first >= item_start_line {
        return Vec::new();
    }
    let mut block: Vec<String> = (first..item_start_line)
        .filter_map(|line_no| {
            lines
                .get(line_no.saturating_sub(1))
                .map(|line| (*line).to_owned())
        })
        .collect();
    if !block.iter().any(|line| is_module_doc_comment(line.trim())) {
        return Vec::new();
    }
    while block.first().is_some_and(|line| line.trim().is_empty()) {
        block.remove(0);
    }
    while block.last().is_some_and(|line| line.trim().is_empty()) {
        block.pop();
    }
    block
}

fn is_module_doc_comment(trimmed: &str) -> bool {
    trimmed.starts_with("//!")
}

fn extract_leading_comments(
    lines: &[&str],
    min_line: usize,
    item_start_line: usize,
) -> Vec<String> {
    if item_start_line <= 1 || lines.is_empty() {
        return Vec::new();
    }

    let mut begin = item_start_line.saturating_sub(1);
    while begin >= min_line {
        let idx = begin.saturating_sub(1);
        if idx >= lines.len() {
            break;
        }
        let trimmed = lines[idx].trim();
        if trimmed.is_empty() || is_plain_line_comment(trimmed) {
            if begin == min_line {
                break;
            }
            begin -= 1;
            continue;
        }
        break;
    }

    let first = if begin < min_line {
        min_line
    } else {
        begin.saturating_add(1)
    };
    if first >= item_start_line {
        return Vec::new();
    }

    let mut block: Vec<String> = (first..item_start_line)
        .filter_map(|line_no| {
            lines
                .get(line_no.saturating_sub(1))
                .map(|line| (*line).to_owned())
        })
        .collect();

    if !block.iter().any(|line| is_plain_line_comment(line.trim())) {
        return Vec::new();
    }

    while block.first().is_some_and(|line| line.trim().is_empty()) {
        block.remove(0);
    }
    while block.last().is_some_and(|line| line.trim().is_empty()) {
        block.pop();
    }
    block
}

fn is_plain_line_comment(trimmed: &str) -> bool {
    trimmed.starts_with("//") && !trimmed.starts_with("///") && !trimmed.starts_with("//!")
}
