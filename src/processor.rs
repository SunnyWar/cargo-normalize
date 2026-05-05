use similar::TextDiff;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::{Attribute, Item, ItemImpl, Type};
use tempfile::NamedTempFile;
use walkdir::WalkDir;
#[derive(Debug, Clone, Default)]
pub struct ProcessSummary {
    pub scanned: usize,
    pub changed: usize,
}
#[derive(Debug, Clone)]
pub struct Processor {
    check: bool,
}
impl Processor {
    pub fn new(check: bool) -> Self {
        Self { check }
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
        let normalized = Normalizer::new(parsed, &promoted_comments).normalize();
        let rendered = normalize_function_spacing(&render_segments(normalized));
        if original == rendered {
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
pub struct Normalizer {
    file: syn::File,
    original: String,
}
impl Normalizer {
    fn new(file: syn::File, original: &str) -> Self {
        Self {
            file,
            original: original.to_owned(),
        }
    }
    fn normalize(self) -> NormalizedFile {
        let segments = segment_items(self.file.items, &self.original);
        let items = reorder_items(segments);

        NormalizedFile {
            shebang: self.file.shebang,
            attrs: self.file.attrs,
            items,
        }
    }
}

#[derive(Debug, Clone)]
struct NormalizedFile {
    shebang: Option<String>,
    attrs: Vec<Attribute>,
    items: Vec<ItemSegment>,
}

#[derive(Debug, Clone)]
struct ItemSegment {
    item: Item,
    leading_comments: Vec<String>,
}

fn collect_rust_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    if root.is_file() {
        if root.extension() == Some(OsStr::new("rs")) {
            return Ok(vec![root.to_path_buf()]);
        }
        return Err(format!(
            "Path {} is a file but not a Rust source (.rs)",
            root.display()
        ));
    }
    if !root.is_dir() {
        return Err(format!("Path does not exist: {}", root.display()));
    }
    let mut files = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_entry(|entry| {
        entry.file_name() != OsStr::new("target") && entry.file_name() != OsStr::new(".git")
    }) {
        let entry = entry.map_err(|err| format!("Directory walk failed: {err}"))?;
        if entry.file_type().is_file() && entry.path().extension() == Some(OsStr::new("rs")) {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn print_diff(path: &Path, before: &str, after: &str) {
    let diff = TextDiff::from_lines(before, after);
    let unified = diff
        .unified_diff()
        .context_radius(3)
        .header(
            &format!("a/{}", path.display()),
            &format!("b/{}", path.display()),
        )
        .to_string();
    println!("{unified}");
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("Cannot determine parent directory for {}", path.display()))?;
    let mut temp = NamedTempFile::new_in(parent)
        .map_err(|err| format!("Failed to create temp file in {}: {err}", parent.display()))?;
    temp.write_all(bytes)
        .map_err(|err| format!("Failed to write temp file for {}: {err}", path.display()))?;
    temp.flush()
        .map_err(|err| format!("Failed to flush temp file for {}: {err}", path.display()))?;
    temp.persist(path)
        .map_err(|err| format!("Failed to persist temp file to {}: {err}", path.display()))?;
    Ok(())
}

fn reorder_items(items: Vec<ItemSegment>) -> Vec<ItemSegment> {
    let mut imports = Vec::new();
    let mut constants = Vec::new();
    let mut data = Vec::new();
    let mut traits = Vec::new();
    let mut tests = Vec::new();
    let mut others = Vec::new();
    let mut impls_by_type: HashMap<String, Vec<ItemSegment>> = HashMap::new();
    let mut fallback_impls = Vec::new();
    for item in items {
        match &item.item {
            Item::Use(_) => imports.push(item),
            Item::Const(_) | Item::Static(_) => constants.push(item),
            Item::Struct(_) | Item::Enum(_) | Item::Union(_) => data.push(item),
            Item::Impl(item_impl) => {
                if let Some(type_name) = inherent_impl_target(&item_impl) {
                    impls_by_type.entry(type_name).or_default().push(item);
                } else {
                    fallback_impls.push(item);
                }
            }
            Item::Trait(_) => traits.push(item),
            Item::Mod(item_mod) if is_test_module(&item_mod.attrs, &item_mod.ident.to_string()) => {
                tests.push(item);
            }
            _ => others.push(item),
        }
    }
    let mut out = Vec::new();
    out.extend(imports);
    out.extend(constants);
    for item in data {
        if let Some(data_name) = data_item_name(&item.item) {
            out.push(item);
            if let Some(mut impls) = impls_by_type.remove(&data_name) {
                out.append(&mut impls);
            }
        } else {
            out.push(item);
        }
    }
    for (_, mut impls) in impls_by_type {
        out.append(&mut impls);
    }
    out.extend(fallback_impls);
    out.extend(traits);
    out.extend(others);
    out.extend(tests);
    out
}

fn data_item_name(item: &Item) -> Option<String> {
    match item {
        Item::Struct(item_struct) => Some(item_struct.ident.to_string()),
        Item::Enum(item_enum) => Some(item_enum.ident.to_string()),
        Item::Union(item_union) => Some(item_union.ident.to_string()),
        _ => None,
    }
}

fn inherent_impl_target(item_impl: &ItemImpl) -> Option<String> {
    if item_impl.trait_.is_some() {
        return None;
    }
    match item_impl.self_ty.as_ref() {
        Type::Path(type_path) if type_path.qself.is_none() => {
            let segment = type_path.path.segments.last()?;
            Some(segment.ident.to_string())
        }
        _ => None,
    }
}

fn is_test_module(attrs: &[Attribute], module_name: &str) -> bool {
    module_name == "tests" || attrs.iter().any(attr_is_cfg_test)
}

fn attr_is_cfg_test(attr: &Attribute) -> bool {
    if !attr.path().is_ident("cfg") {
        return false;
    }
    let mut found = false;
    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("test") {
            found = true;
        }
        Ok(())
    });
    found
}

fn segment_items(items: Vec<Item>, source: &str) -> Vec<ItemSegment> {
    let lines: Vec<&str> = source.lines().collect();
    let mut prev_end_line = 1usize;
    let mut segments = Vec::with_capacity(items.len());

    for item in items {
        let span = item.span();
        let start_line = span.start().line.max(1);
        let end_line = span.end().line.max(start_line);
        let leading_comments = extract_leading_comments(&lines, prev_end_line, start_line);

        segments.push(ItemSegment {
            item,
            leading_comments,
        });

        prev_end_line = end_line.saturating_add(1);
    }

    segments
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

fn render_segments(normalized: NormalizedFile) -> String {
    let mut out = String::new();

    if normalized.shebang.is_some() || !normalized.attrs.is_empty() {
        let preamble = prettyplease::unparse(&syn::File {
            shebang: normalized.shebang,
            attrs: normalized.attrs,
            items: Vec::new(),
        });

        if !preamble.trim().is_empty() {
            out.push_str(preamble.trim_end());
            out.push_str("\n\n");
        }
    }

    for segment in normalized.items {
        if !segment.leading_comments.is_empty() {
            for line in segment.leading_comments {
                out.push_str(&line);
                out.push('\n');
            }
        }

        let item_source = prettyplease::unparse(&syn::File {
            shebang: None,
            attrs: Vec::new(),
            items: vec![segment.item],
        });

        out.push_str(item_source.trim_end());
        out.push_str("\n\n");
    }

    out.truncate(out.trim_end_matches('\n').len());
    out.push('\n');
    out
}

fn promote_leading_item_comments(source: &str) -> String {
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
                    out.push(format!("{}///{}", indent, &trimmed_line[2..]));
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

fn normalize_function_spacing(rendered: &str) -> String {
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
