mod config;
mod error;

use config::loader::{load_config, LoadResult};
use std::process;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
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
        LoadResult::Loaded(config) => {
            println!("Config loaded successfully.");
            if let Some(sender) = &config.sender {
                println!("Sender: {}", sender.name);
            }
            if let Some(recipient) = &config.recipient {
                println!("Recipient: {}", recipient.name);
            }
            Ok(())
        }
    }
}
