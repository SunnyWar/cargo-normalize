use config::parse_cli;
use processor::Processor;
use std::process::ExitCode;
mod config;
mod processor;
fn main() -> ExitCode {
    let cli = parse_cli();
    let processor = Processor::new(cli.check);
    let summary = match processor.run(&cli.path) {
        Ok(summary) => summary,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(2);
        }
    };
    if cli.check {
        if summary.changed > 0 {
            eprintln!(
                "found {} non-normalized file(s) out of {}", summary.changed, summary
                .scanned
            );
            return ExitCode::from(1);
        }
        println!("all {} file(s) are normalized", summary.scanned);
        return ExitCode::SUCCESS;
    }
    println!("processed {} file(s), normalized {}", summary.scanned, summary.changed);
    ExitCode::SUCCESS
}
