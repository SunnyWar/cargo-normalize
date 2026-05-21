use crate::config::{MoveSelection, NormalizeConfig};
use syn::{Attribute, Item};

use super::ordering::reorder_items;
use super::segment::segment_items;

pub(crate) const PROMOTED_COMMENT_MARKER: &str = "__cargo_normalize_promoted__";

pub struct Normalizer {
    file: syn::File,
    original: String,
}

impl Normalizer {
    pub(super) fn new(file: syn::File, original: &str) -> Self {
        Self {
            file,
            original: original.to_owned(),
        }
    }

    pub(super) fn normalize(
        self,
        config: &NormalizeConfig,
        selection: &MoveSelection,
    ) -> NormalizedFile {
        let segments = segment_items(self.file.items, &self.original);
        let items = reorder_items(segments, config, selection);
        NormalizedFile {
            shebang: self.file.shebang,
            attrs: self.file.attrs,
            items,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedFile {
    pub(crate) shebang: Option<String>,
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) items: Vec<ItemSegment>,
}

#[derive(Debug, Clone)]
pub(crate) struct ItemSegment {
    pub(crate) item: Item,
    pub(crate) leading_comments: Vec<String>,
    pub(crate) module_doc_comments: Vec<String>,
    pub(crate) source: String,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum CompactGroup {
    Use,
    Const,
    Mod,
}
