use clap::Parser;
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

pub fn parse_cli() -> Cli {
    let mut args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|arg| arg == "normalize") {
        args.remove(1);
    }
    Cli::parse_from(args)
}
