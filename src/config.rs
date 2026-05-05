use clap::Parser;
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
    /// File or directory to normalize.
    #[arg(long, value_name = "PATH", default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NormalizeConfig {
    pub order: Vec<ItemOrder>,
    pub compact_use_block: bool,
    pub compact_const_block: bool,
    pub compact_mod_block: bool,
}

impl NormalizeConfig {
    pub fn mods_before_constants(&self) -> bool {
        let mods_index = self
            .order
            .iter()
            .position(|group| *group == ItemOrder::Mods);
        let constants_index = self
            .order
            .iter()
            .position(|group| *group == ItemOrder::Constants);
        match (mods_index, constants_index) {
            (Some(mods_idx), Some(constants_idx)) => mods_idx <= constants_idx,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => true,
        }
    }

    pub fn types_before_functions(&self) -> bool {
        let types_index = self
            .order
            .iter()
            .position(|group| *group == ItemOrder::Types);
        let functions_index = self
            .order
            .iter()
            .position(|group| *group == ItemOrder::Functions);
        match (types_index, functions_index) {
            (Some(types_idx), Some(functions_idx)) => types_idx <= functions_idx,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => true,
        }
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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ItemOrder {
    Imports,
    Mods,
    Constants,
    Enums,
    Structs,
    Impls,
    Traits,
    Types,
    Functions,
    Tests,
}

impl Default for NormalizeConfig {
    fn default() -> Self {
        Self {
            order: vec![
                ItemOrder::Imports,
                ItemOrder::Mods,
                ItemOrder::Constants,
                ItemOrder::Enums,
                ItemOrder::Structs,
                ItemOrder::Impls,
                ItemOrder::Traits,
                ItemOrder::Types,
                ItemOrder::Functions,
                ItemOrder::Tests,
            ],
            compact_use_block: true,
            compact_const_block: true,
            compact_mod_block: true,
        }
    }
}

pub fn parse_cli() -> Cli {
    let mut args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|arg| arg == "normalize") {
        args.remove(1);
    }
    Cli::parse_from(args)
}
