mod config;
mod error;
mod invoice;
mod pdf;
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
    cwd: &std::path::Path,
) -> Result<(), error::AppError> {
    let presets = validated.presets.clone();
    loop {
        let now = time::OffsetDateTime::now_utc();
        let period = invoice::period::collect_invoice_period(
            prompter,
            u32::from(now.month() as u8),
            now.year() as u32,
        )?;

        let line_items = invoice::line_item::collect_all_line_items(
            prompter,
            &presets,
            &validated.defaults.currency,
            cwd,
        )?;

        let summary = invoice::summary::build_summary(
            period,
            line_items,
            &validated.defaults,
        )?;

        let formatted = invoice::display::format_summary(&summary);
        prompter.message(&formatted);

        if prompter.confirm("Generate PDF?", true)? {
            let pdf_bytes = pdf::generate_pdf(&summary, validated)?;
            let name = validated.sender.name.replace(' ', "_");
            let filename = format!(
                "Invoice_{}_{}{}.pdf",
                name,
                summary.period.month_abbrev(),
                summary.period.year()
            );
            let output_path = cwd.join(&filename);

            if output_path.exists() {
                if !prompter.confirm("File already exists. Overwrite?", false)? {
                    prompter.message("PDF generation aborted.");
                    continue;
                }
            }

            std::fs::write(&output_path, &pdf_bytes)?;
            prompter.message(&format!("PDF saved: {}", output_path.display()));
            break;
        }

        prompter.message("Starting over...\n");
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

    run_invoice_flow(&prompter, &validated, &cwd)
}
