mod config;
mod error;
mod invoice;
mod setup;

use config::loader::{load_config, LoadResult};
use config::types::Config;
use config::validator::{ConfigSection, ValidatedConfig, ValidationOutcome};
use setup::prompter::{InquirePrompter, Prompter};
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

fn run_invoice_flow(
    prompter: &dyn Prompter,
    validated: &ValidatedConfig,
) -> Result<(), error::AppError> {
    let now = time::OffsetDateTime::now_utc();
    let period = invoice::period::collect_invoice_period(
        prompter,
        u32::from(now.month() as u8),
        now.year() as u32,
    )?;
    println!("Invoice period: {period}");

    let selection = invoice::preset_selection::select_preset(
        prompter,
        &validated.presets,
        &validated.defaults.currency,
    )?;

    match &selection {
        invoice::types::PresetSelection::Existing(preset) => {
            let line_item = invoice::line_item::collect_line_item_details(
                prompter,
                &preset,
                1,
            )?;
            println!(
                "Line item: {} — {:.2} days @ {:.2} = {:.2}",
                line_item.description, line_item.days, line_item.rate, line_item.amount
            );
        }
        invoice::types::PresetSelection::CreateNew => {
            println!("Create new preset (not yet implemented)");
        }
    }

    Ok(())
}

fn run() -> Result<(), error::AppError> {
    let cwd = std::env::current_dir().map_err(error::AppError::ConfigIo)?;
    let prompter = InquirePrompter::new();

    let validated = match load_config(&cwd)? {
        LoadResult::NotFound => {
            let mut config = Config::default();
            let all_missing = vec![
                ConfigSection::Sender,
                ConfigSection::Recipient,
                ConfigSection::Payment,
                ConfigSection::Presets,
            ];
            setup::run_setup(&prompter, &mut config, &all_missing, &cwd)?;
            match config.validate() {
                ValidationOutcome::Complete(v) => v,
                ValidationOutcome::Incomplete { .. } => {
                    unreachable!("Setup completed but config still incomplete")
                }
            }
        }
        LoadResult::Loaded(config) => match config.validate() {
            ValidationOutcome::Complete(v) => {
                println!("Config loaded successfully.");
                println!("Sender: {}", v.sender.name);
                println!("Recipient: {}", v.recipient.name);
                v
            }
            ValidationOutcome::Incomplete { mut config, missing } => {
                setup::run_setup(&prompter, &mut config, &missing, &cwd)?;
                match config.validate() {
                    ValidationOutcome::Complete(v) => v,
                    ValidationOutcome::Incomplete { .. } => {
                        unreachable!("Setup completed but config still incomplete")
                    }
                }
            }
        },
    };

    run_invoice_flow(&prompter, &validated)
}
