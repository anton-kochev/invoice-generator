mod cli;
mod config;
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

fn run(cli: Cli) -> Result<(), error::AppError> {
    let cwd = std::env::current_dir().map_err(error::AppError::ConfigIo)?;
    let prompter = InquirePrompter::new();

    match cli.command {
        None => cli::interactive::run_interactive(&prompter, &cwd),
        Some(Command::Generate(args)) => {
            // NOTE: main.rs wiring is updated by a separate slice (resolve_config_path).
            // For now pass cwd for both config_path and output_dir to keep the build green.
            cli::generate_cmd::handle_generate(&args, &cwd, &cwd, &mut std::io::stdout())
        }
        Some(Command::Preset { action }) => match action {
            PresetAction::List => {
                let validated = cli::load_validated_config(&cwd)?;
                cli::preset_cmd::handle_preset_list(&validated, &mut std::io::stdout())
            }
            PresetAction::Delete { key } => {
                cli::preset_cmd::handle_preset_delete(
                    &prompter,
                    &cwd,
                    &key,
                    &mut std::io::stdout(),
                )
            }
        },
        Some(Command::Recipient { action }) => match action {
            RecipientAction::List => {
                let validated = cli::load_validated_config(&cwd)?;
                cli::recipient_cmd::handle_recipient_list(&validated, &mut std::io::stdout())
            }
            RecipientAction::Add => {
                cli::recipient_cmd::handle_recipient_add(
                    &prompter,
                    &cwd,
                    &mut std::io::stdout(),
                )
            }
            RecipientAction::Delete { key } => {
                cli::recipient_cmd::handle_recipient_delete(
                    &prompter,
                    &cwd,
                    &key,
                    &mut std::io::stdout(),
                )
            }
        },
    }
}
