use similar::TextDiff;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
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
        let parsed = syn::parse_file(&original)
            .map_err(|err| format!("Failed to parse {}: {err}", path.display()))?;
        let normalized = Normalizer::new(parsed).normalize();
        let rendered = normalize_function_spacing(&prettyplease::unparse(&normalized));
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
}
impl Normalizer {
    pub fn new(file: syn::File) -> Self {
        Self { file }
    }
    pub fn normalize(mut self) -> syn::File {
        self.file.items = reorder_items(self.file.items);
        self.file
    }
}

fn collect_rust_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    if root.is_file() {
        if root.extension() == Some(OsStr::new("rs")) {
            return Ok(vec![root.to_path_buf()]);
        }
        return Err(
            format!("Path {} is a file but not a Rust source (.rs)", root.display()),
        );
    }
    if !root.is_dir() {
        return Err(format!("Path does not exist: {}", root.display()));
    }
    let mut files = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| {
            entry.file_name() != OsStr::new("target")
                && entry.file_name() != OsStr::new(".git")
        })
    {
        let entry = entry.map_err(|err| format!("Directory walk failed: {err}"))?;
        if entry.file_type().is_file()
            && entry.path().extension() == Some(OsStr::new("rs"))
        {
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
        .header(&format!("a/{}", path.display()), &format!("b/{}", path.display()))
        .to_string();
    println!("{unified}");
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| {
            format!("Cannot determine parent directory for {}", path.display())
        })?;
    let mut temp = NamedTempFile::new_in(parent)
        .map_err(|err| {
            format!("Failed to create temp file in {}: {err}", parent.display())
        })?;
    temp.write_all(bytes)
        .map_err(|err| {
            format!("Failed to write temp file for {}: {err}", path.display())
        })?;
    temp.flush()
        .map_err(|err| {
            format!("Failed to flush temp file for {}: {err}", path.display())
        })?;
    temp.persist(path)
        .map_err(|err| {
            format!("Failed to persist temp file to {}: {err}", path.display())
        })?;
    Ok(())
}

fn reorder_items(items: Vec<Item>) -> Vec<Item> {
    let mut imports = Vec::new();
    let mut constants = Vec::new();
    let mut data = Vec::new();
    let mut traits = Vec::new();
    let mut tests = Vec::new();
    let mut others = Vec::new();
    let mut impls_by_type: HashMap<String, Vec<Item>> = HashMap::new();
    let mut fallback_impls = Vec::new();
    for item in items {
        match item {
            Item::Use(_) => imports.push(item),
            Item::Const(_) | Item::Static(_) => constants.push(item),
            Item::Struct(_) | Item::Enum(_) | Item::Union(_) => data.push(item),
            Item::Impl(item_impl) => {
                if let Some(type_name) = inherent_impl_target(&item_impl) {
                    impls_by_type
                        .entry(type_name)
                        .or_default()
                        .push(Item::Impl(item_impl));
                } else {
                    fallback_impls.push(Item::Impl(item_impl));
                }
            }
            Item::Trait(_) => traits.push(item),
            Item::Mod(
                item_mod,
            ) if is_test_module(&item_mod.attrs, &item_mod.ident.to_string()) => {
                tests.push(Item::Mod(item_mod));
            }
            _ => others.push(item),
        }
    }
    let mut out = Vec::new();
    out.extend(imports);
    out.extend(constants);
    for item in data {
        if let Some(data_name) = data_item_name(&item) {
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
    let _ = attr
        .parse_nested_meta(|meta| {
            if meta.path.is_ident("test") {
                found = true;
            }
            Ok(())
        });
    found
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
    if rendered.ends_with('\n') { out } else { out.trim_end_matches('\n').to_string() }
}
