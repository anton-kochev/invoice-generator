mod cli;
mod config;
mod domain;
mod error;
mod invoice;
mod locale;
mod pdf;
mod setup;

use clap::Parser;
use cli::{Cli, Command, PresetAction, RecipientAction};
use setup::prompter::InquirePrompter;
use std::process;

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        match e {
            error::AppError::SetupCancelled => {
                println!("Setup cancelled. Your progress has been saved.");
            }
            error::AppError::Config(config::ConfigError::Parse(_)) => {
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

fn run(cli: Cli) -> Result<(), error::AppError> {
    let config_path = config::path::resolve_config_path(
        cli.config.as_deref(),
        &config::path::RealEnv,
    )?;
    config::path::ensure_parent_dir(&config_path)?;
    // The output directory is consumed by the PDF subsystem only, so a failure
    // here is a PDF-write IO failure rather than a config-IO failure.
    let output_dir = std::env::current_dir().map_err(pdf::PdfError::Write)?;
    let prompter = InquirePrompter::new();

    match cli.command {
        None => cli::interactive::run_interactive(&prompter, &config_path, &output_dir),
        Some(Command::Generate(args)) => {
            cli::generate_cmd::handle_generate(&args, &config_path, &output_dir, &mut std::io::stdout())
        }
        Some(Command::Preset { action }) => match action {
            PresetAction::List => {
                let validated = cli::load_validated_config(&config_path)?;
                cli::preset_cmd::handle_preset_list(&validated, &mut std::io::stdout())
            }
            PresetAction::Delete { key } => {
                cli::preset_cmd::handle_preset_delete(
                    &prompter,
                    &config_path,
                    &key,
                    &mut std::io::stdout(),
                )
            }
        },
        Some(Command::Recipient { action }) => match action {
            RecipientAction::List => {
                let validated = cli::load_validated_config(&config_path)?;
                cli::recipient_cmd::handle_recipient_list(&validated, &mut std::io::stdout())
            }
            RecipientAction::Add => {
                cli::recipient_cmd::handle_recipient_add(
                    &prompter,
                    &config_path,
                    &mut std::io::stdout(),
                )
            }
            RecipientAction::Delete { key } => {
                cli::recipient_cmd::handle_recipient_delete(
                    &prompter,
                    &config_path,
                    &key,
                    &mut std::io::stdout(),
                )
            }
        },
    }
}
