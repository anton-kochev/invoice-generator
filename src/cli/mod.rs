pub mod interactive;
pub mod preset_cmd;

use std::path::Path;

use clap::{Parser, Subcommand};

use crate::config::loader::{load_config, LoadResult};
use crate::config::validator::{ValidatedConfig, ValidationOutcome};
use crate::error::AppError;

/// Invoice Generator CLI
#[derive(Parser)]
#[command(name = "invoice", version, about = "Generate professional invoices")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage invoice presets
    Preset {
        #[command(subcommand)]
        action: PresetAction,
    },
}

#[derive(Debug, Subcommand)]
pub enum PresetAction {
    /// List all configured presets
    List,
    /// Delete a preset by key
    Delete {
        /// The preset key to delete
        key: String,
    },
}

/// Load and validate config for non-interactive subcommands.
/// Never triggers the setup wizard — returns error if config missing or incomplete.
pub fn load_validated_config(dir: &Path) -> Result<ValidatedConfig, AppError> {
    match load_config(dir)? {
        LoadResult::NotFound => Err(AppError::ConfigNotFound),
        LoadResult::Loaded(config) => match config.validate() {
            ValidationOutcome::Complete(v) => Ok(v),
            ValidationOutcome::Incomplete { .. } => Err(AppError::ConfigNotFound),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_args_returns_none_command() {
        // Arrange
        let args = ["invoice"];

        // Act
        let cli = Cli::try_parse_from(args).unwrap();

        // Assert
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_preset_list_parses() {
        // Arrange
        let args = ["invoice", "preset", "list"];

        // Act
        let cli = Cli::try_parse_from(args).unwrap();

        // Assert
        assert!(matches!(
            cli.command,
            Some(Command::Preset {
                action: PresetAction::List
            })
        ));
    }

    #[test]
    fn test_preset_delete_parses_key() {
        // Arrange
        let args = ["invoice", "preset", "delete", "pwc"];

        // Act
        let cli = Cli::try_parse_from(args).unwrap();

        // Assert
        match cli.command {
            Some(Command::Preset {
                action: PresetAction::Delete { key },
            }) => assert_eq!(key, "pwc"),
            other => panic!("Expected Preset Delete, got {other:?}"),
        }
    }

    #[test]
    fn test_preset_delete_missing_key_is_error() {
        // Arrange
        let args = ["invoice", "preset", "delete"];

        // Act
        let result = Cli::try_parse_from(args);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_subcommand_is_error() {
        // Arrange
        let args = ["invoice", "bogus"];

        // Act
        let result = Cli::try_parse_from(args);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_preset_unknown_action_is_error() {
        // Arrange
        let args = ["invoice", "preset", "bogus"];

        // Act
        let result = Cli::try_parse_from(args);

        // Assert
        assert!(result.is_err());
    }
}
