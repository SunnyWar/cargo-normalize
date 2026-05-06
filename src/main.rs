use config::{NormalizeConfig, parse_cli};
use processor::Processor;
use std::process::ExitCode;
mod config;
mod processor;
fn main() -> ExitCode {
    let cli = parse_cli();
    let check_mode = cli.is_effective_check();

    let normalize_config = match NormalizeConfig::load_for_path(&cli.path) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(2);
        }
    };

    let processor = Processor::new(check_mode, cli.effective_move_selection(), normalize_config);
    let summary = match processor.run(&cli.path) {
        Ok(summary) => summary,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(2);
        }
    };
    if check_mode {
        if summary.changed > 0 {
            eprintln!(
                "found {} non-normalized file(s) out of {}",
                summary.changed, summary.scanned
            );
            for path in &summary.changed_files {
                eprintln!("{}", path.display());
            }
            return ExitCode::from(1);
        }
        println!("all {} file(s) are normalized", summary.scanned);
        return ExitCode::SUCCESS;
    }
    println!(
        "processed {} file(s), normalized {}",
        summary.scanned, summary.changed
    );
    ExitCode::SUCCESS
}
