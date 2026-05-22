use clap::{Parser, ValueEnum};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(author, version, about = "Normalize Rust item ordering", long_about = None)]
pub struct Cli {
    /// Check whether files are normalized without writing changes.
    #[arg(long)]
    pub check: bool,
    /// Move a single normalization feature. Repeat to move multiple features.
    #[arg(long, value_enum, action = clap::ArgAction::Append, value_name = "FEATURE")]
    pub move_feature: Vec<MoveFeature>,
    /// Move all normalization features.
    #[arg(long, conflicts_with = "move_feature")]
    pub all: bool,
    /// File or directory to normalize.
    #[arg(long, value_name = "PATH", default_value = ".")]
    pub path: PathBuf,
}

impl Cli {
    pub fn is_effective_check(&self) -> bool {
        self.check || (!self.all && self.move_feature.is_empty())
    }

    pub fn effective_move_selection(&self) -> MoveSelection {
        let effective_check = self.is_effective_check();
        let implicit_all = !self.all && self.move_feature.is_empty() && effective_check;
        MoveSelection {
            all: self.all || implicit_all,
            features: self.move_feature.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MoveSelection {
    pub all: bool,
    pub features: Vec<MoveFeature>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NormalizeConfig {
    #[serde(alias = "priority")]
    pub order: Vec<ItemOrder>,
    pub compact_use_block: bool,
    pub compact_const_block: bool,
    pub compact_mod_block: bool,
    /// If true, insert a one-line comment with the relative path at the top of each .rs file.
    #[serde(default)]
    pub relative_path_comment: bool,
}

impl NormalizeConfig {
    fn position(&self, target: ItemOrder) -> Option<usize> {
        self.order.iter().position(|group| *group == target)
    }

    fn comes_before(&self, left: ItemOrder, right: ItemOrder) -> bool {
        match (self.position(left), self.position(right)) {
            (Some(left_idx), Some(right_idx)) => left_idx <= right_idx,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => true,
        }
    }

    pub fn rank(&self, group: ItemOrder, default_rank: usize) -> usize {
        self.position(group).unwrap_or(default_rank)
    }

    pub fn mods_before_macros(&self) -> bool {
        self.comes_before(ItemOrder::Mods, ItemOrder::Macros)
    }

    pub fn constants_before_types(&self) -> bool {
        self.comes_before(ItemOrder::Constants, ItemOrder::Types)
    }
}

impl NormalizeConfig {
    pub fn load_for_path(path: &Path) -> Result<Self, String> {
        let root = if path.is_file() {
            path.parent()
                .ok_or_else(|| format!("Cannot determine parent directory for {}", path.display()))?
                .to_path_buf()
        } else {
            path.to_path_buf()
        };
        let config_path = root.join("normalize.toml");
        if !config_path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(&config_path)
            .map_err(|err| format!("Failed to read {}: {err}", config_path.display()))?;
        toml::from_str::<Self>(&text)
            .map_err(|err| format!("Failed to parse {}: {err}", config_path.display()))
    }
}

impl Default for NormalizeConfig {
    fn default() -> Self {
        Self {
            order: vec![
                ItemOrder::Attributes,
                ItemOrder::Mods,
                ItemOrder::Imports,
                ItemOrder::Macros,
                ItemOrder::Constants,
                ItemOrder::Types,
                ItemOrder::Enums,
                ItemOrder::Structs,
                ItemOrder::Impls,
                ItemOrder::Traits,
                ItemOrder::Foreign,
                ItemOrder::Functions,
                ItemOrder::Tests,
            ],
            compact_use_block: true,
            compact_const_block: true,
            compact_mod_block: true,
            relative_path_comment: false,
        }
    }
}

#[derive(Debug, Copy, Clone, ValueEnum, PartialEq, Eq)]
pub enum MoveFeature {
    Attributes,
    Imports,
    #[value(name = "modules")]
    Mods,
    Macros,
    Constants,
    Types,
    Enums,
    Structs,
    Impls,
    Traits,
    #[value(name = "extern_blocks")]
    Foreign,
    Functions,
    Tests,
}

#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ItemOrder {
    Attributes,
    Imports,
    #[serde(alias = "modules")]
    Mods,
    Macros,
    Constants,
    Types,
    Enums,
    Structs,
    Impls,
    Traits,
    #[serde(rename = "ffi", alias = "foreign", alias = "extern_blocks")]
    Foreign,
    Functions,
    Tests,
}

pub fn parse_cli() -> Cli {
    let mut args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|arg| arg == "normalize") {
        args.remove(1);
    }
    if args.get(1).is_some_and(|arg| arg == "help") {
        args[1] = "--help".to_owned();
    }
    Cli::parse_from(args)
}
