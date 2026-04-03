mod config;
mod error;
mod invoice;
mod setup;

use config::loader::{load_config, LoadResult};
use config::types::Config;
use config::validator::{ConfigSection, ValidationOutcome};
use setup::prompter::InquirePrompter;
use std::process;

fn main() {
    if let Err(e) = run() {
        match e {
            error::AppError::SetupCancelled => {
                println!("Setup cancelled. Your progress has been saved.");
            }
            error::AppError::ConfigParse(_) => {
                eprintln!("Error: {e}");
                eprintln!("Fix the file or delete it to re-run setup.");
                process::exit(1);
            }
            _ => {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        }
    }
}

fn run() -> Result<(), error::AppError> {
    let cwd = std::env::current_dir().map_err(error::AppError::ConfigIo)?;
    let prompter = InquirePrompter::new();

    match load_config(&cwd)? {
        LoadResult::NotFound => {
            let mut config = Config::default();
            let all_missing = vec![
                ConfigSection::Sender,
                ConfigSection::Recipient,
                ConfigSection::Payment,
                ConfigSection::Presets,
            ];
            setup::run_setup(&prompter, &mut config, &all_missing, &cwd)?;

            let now = time::OffsetDateTime::now_utc();
            let period = invoice::period::collect_invoice_period(
                &prompter,
                u32::from(now.month() as u8),
                now.year() as u32,
            )?;
            println!("Invoice period: {period}");
            Ok(())
        }
        LoadResult::Loaded(config) => match config.validate() {
            ValidationOutcome::Complete(validated) => {
                println!("Config loaded successfully.");
                println!("Sender: {}", validated.sender.name);
                println!("Recipient: {}", validated.recipient.name);

                let now = time::OffsetDateTime::now_utc();
                let period = invoice::period::collect_invoice_period(
                    &prompter,
                    u32::from(now.month() as u8),
                    now.year() as u32,
                )?;
                println!("Invoice period: {period}");
                Ok(())
            }
            ValidationOutcome::Incomplete { mut config, missing } => {
                setup::run_setup(&prompter, &mut config, &missing, &cwd)?;

                let now = time::OffsetDateTime::now_utc();
                let period = invoice::period::collect_invoice_period(
                    &prompter,
                    u32::from(now.month() as u8),
                    now.year() as u32,
                )?;
                println!("Invoice period: {period}");
                Ok(())
            }
        },
    }
}
