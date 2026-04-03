mod config;
mod error;

use config::loader::{load_config, LoadResult};
use config::validator::ValidationOutcome;
use std::process;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        if matches!(e, error::AppError::ConfigParse(_)) {
            eprintln!("Fix the file or delete it to re-run setup.");
        }
        process::exit(1);
    }
}

fn run() -> Result<(), error::AppError> {
    let cwd = std::env::current_dir().map_err(error::AppError::ConfigIo)?;
    match load_config(&cwd)? {
        LoadResult::NotFound => {
            println!("No config file found. First-run setup would start here.");
            Ok(())
        }
        LoadResult::Loaded(config) => match config.validate() {
            ValidationOutcome::Complete(validated) => {
                println!("Config loaded successfully.");
                println!("Sender: {}", validated.sender.name);
                println!("Recipient: {}", validated.recipient.name);
                // TODO: Story 3.1 - proceed to invoice flow
                Ok(())
            }
            ValidationOutcome::Incomplete { missing, .. } => {
                eprintln!("Config is incomplete. Missing sections:");
                for section in &missing {
                    eprintln!("  - {section}");
                }
                eprintln!("Setup would resume from: {}", missing[0]);
                eprintln!("Fix the file or delete it to re-run setup.");
                std::process::exit(1);
            }
        },
    }
}
