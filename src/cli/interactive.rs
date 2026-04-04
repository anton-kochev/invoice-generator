use std::path::Path;

use crate::config::loader::{load_config, LoadResult};
use crate::config::types::Config;
use crate::config::validator::{ConfigSection, ValidatedConfig, ValidationOutcome};
use crate::error::AppError;
use crate::setup::prompter::Prompter;
use crate::{invoice, pdf, setup};

/// The full v1.0 interactive flow: load config → maybe setup → invoice → PDF.
pub fn run_interactive(prompter: &dyn Prompter, cwd: &Path) -> Result<(), AppError> {
    let validated = match load_config(cwd)? {
        LoadResult::NotFound => {
            let mut config = Config::default();
            let all_missing = vec![
                ConfigSection::Sender,
                ConfigSection::Recipient,
                ConfigSection::Payment,
                ConfigSection::Presets,
            ];
            setup::run_setup(prompter, &mut config, &all_missing, cwd)?;
            match config.validate()? {
                ValidationOutcome::Complete(v) => v,
                ValidationOutcome::Incomplete { .. } => {
                    unreachable!("Setup completed but config still incomplete")
                }
            }
        }
        LoadResult::Loaded(config) => match config.validate()? {
            ValidationOutcome::Complete(v) => {
                println!("Config loaded successfully.");
                println!("Sender: {}", v.sender.name);
                println!("Recipient: {}", v.recipient.name);
                v
            }
            ValidationOutcome::Incomplete { mut config, missing } => {
                setup::run_setup(prompter, &mut config, &missing, cwd)?;
                match config.validate()? {
                    ValidationOutcome::Complete(v) => v,
                    ValidationOutcome::Incomplete { .. } => {
                        unreachable!("Setup completed but config still incomplete")
                    }
                }
            }
        },
    };

    run_invoice_flow(prompter, &validated, cwd)
}

/// Run the interactive invoice generation loop.
pub fn run_invoice_flow(
    prompter: &dyn Prompter,
    validated: &ValidatedConfig,
    cwd: &Path,
) -> Result<(), AppError> {
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
            let pdf_bytes = pdf::generate_pdf(&summary, validated, &validated.recipient)?;
            let output_path = super::common::pdf_output_path(
                &validated.sender.name,
                &summary.period,
                cwd,
            );

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
