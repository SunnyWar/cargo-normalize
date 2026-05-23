use super::PROMOTED_COMMENT_MARKER;

pub(super) fn promote_leading_item_comments(source: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0usize;

    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        if !is_plain_line_comment(trimmed) {
            out.push(lines[i].to_owned());
            i += 1;
            continue;
        }

        let block_start = i;
        let mut block_end = i;
        while block_end + 1 < lines.len() {
            let next = lines[block_end + 1].trim_start();
            if is_plain_line_comment(next) || next.is_empty() {
                block_end += 1;
            } else {
                break;
            }
        }

        let mut lookahead = block_end + 1;
        while lookahead < lines.len() && lines[lookahead].trim_start().is_empty() {
            lookahead += 1;
        }

        let attach_to_item =
            lookahead < lines.len() && is_item_declaration_line(lines[lookahead].trim_start());

        if attach_to_item {
            for line in &lines[block_start..=block_end] {
                let trimmed_line = line.trim_start();
                if is_plain_line_comment(trimmed_line) {
                    let indent_len = line.len().saturating_sub(trimmed_line.len());
                    let indent = &line[..indent_len];
                    out.push(format!(
                        "{}///{}{}",
                        indent,
                        PROMOTED_COMMENT_MARKER,
                        &trimmed_line[2..]
                    ));
                } else {
                    out.push((*line).to_owned());
                }
            }
        } else {
            for line in &lines[block_start..=block_end] {
                out.push((*line).to_owned());
            }
        }

        i = block_end + 1;
    }

    let mut text = out.join("\n");
    if source.ends_with('\n') {
        text.push('\n');
    }
    text
}

pub(super) fn normalize_function_spacing(rendered: &str) -> String {
    let mut out = String::with_capacity(rendered.len() + 64);
    let mut prev_non_empty: Option<&str> = None;

    for line in rendered.lines() {
        let trimmed = line.trim();
        let is_top_level_fn = !line.starts_with([' ', '\t'])
            && (trimmed.starts_with("fn ")
                || trimmed.starts_with("pub") && trimmed.contains(" fn "));

        if is_top_level_fn && prev_non_empty == Some("}") && !out.ends_with("\n\n") {
            out.push('\n');
        }

        out.push_str(line);
        out.push('\n');
        if !trimmed.is_empty() {
            prev_non_empty = Some(trimmed);
        }
    }

    if rendered.ends_with('\n') {
        out
    } else {
        out.trim_end_matches('\n').to_string()
    }
}

pub(super) fn restore_promoted_comment_style(rendered: &str) -> String {
    let promoted_prefix = format!("///{PROMOTED_COMMENT_MARKER}");
    let mut out = String::with_capacity(rendered.len());

    for line in rendered.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(&promoted_prefix) {
            let indent_len = line.len().saturating_sub(trimmed.len());
            let indent = &line[..indent_len];
            out.push_str(indent);
            out.push_str("//");
            out.push_str(rest);
            out.push('\n');
            continue;
        }

        out.push_str(line);
        out.push('\n');
    }

    if rendered.ends_with('\n') {
        out
    } else {
        out.trim_end_matches('\n').to_owned()
    }
}

pub(super) fn differs_only_by_whitespace(before: &str, after: &str) -> bool {
    strip_whitespace_and_trailing_commas(before) == strip_whitespace_and_trailing_commas(after)
}

fn strip_whitespace_and_trailing_commas(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_whitespace() {
            continue;
        }

        if ch == ',' {
            let mut lookahead = chars.clone();
            let mut next_non_whitespace = None;
            for next in lookahead {
                if next.is_whitespace() {
                    continue;
                }
                next_non_whitespace = Some(next);
                break;
            }
            if !matches!(next_non_whitespace, Some(')' | ']' | '}')) {
                output.push(',');
            }
            continue;
        }

        output.push(ch);
    }

    output
}

fn is_plain_line_comment(trimmed: &str) -> bool {
    trimmed.starts_with("//") && !trimmed.starts_with("///") && !trimmed.starts_with("//!")
}

fn is_item_declaration_line(trimmed: &str) -> bool {
    let candidates = [
        "fn ",
        "pub fn ",
        "pub(crate) fn ",
        "pub(super) fn ",
        "pub(in ",
        "unsafe fn ",
        "pub unsafe fn ",
        "struct ",
        "pub struct ",
        "enum ",
        "pub enum ",
        "union ",
        "pub union ",
        "impl ",
        "unsafe impl ",
        "trait ",
        "pub trait ",
        "mod ",
        "pub mod ",
        "use ",
        "pub use ",
        "const ",
        "pub const ",
        "static ",
        "pub static ",
    ];
    candidates.iter().any(|prefix| trimmed.starts_with(prefix))
}
