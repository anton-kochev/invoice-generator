pub mod common;
pub mod generate_cmd;
pub mod interactive;
pub mod preset_cmd;
pub mod recipient_selection;

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
    /// Generate an invoice (non-interactive)
    Generate(GenerateArgs),
}

/// Arguments for the `generate` subcommand.
#[derive(Debug, clap::Args)]
#[command(
    group = clap::ArgGroup::new("item-source")
        .required(true)
        .args(["preset", "items"])
)]
pub struct GenerateArgs {
    /// Billing month (1-12)
    #[arg(long)]
    pub month: u32,
    /// Billing year (e.g. 2026)
    #[arg(long)]
    pub year: u32,
    /// Preset key to use for a single line item
    #[arg(long, requires = "days", conflicts_with = "items")]
    pub preset: Option<String>,
    /// Number of days worked (required with --preset)
    #[arg(long, requires = "preset", conflicts_with = "items")]
    pub days: Option<f64>,
    /// JSON array of line items: [{"preset":"key","days":N,"rate":N}]
    #[arg(long, conflicts_with_all = ["preset", "days"])]
    pub items: Option<String>,
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
        LoadResult::Loaded(config) => match config.validate()? {
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

    // ── Generate subcommand tests ──

    #[test]
    fn test_generate_single_item_parses_all_flags() {
        // Arrange
        let args = [
            "invoice", "generate", "--month", "3", "--year", "2026",
            "--preset", "pwc", "--days", "10",
        ];

        // Act
        let cli = Cli::try_parse_from(args).unwrap();

        // Assert
        match cli.command {
            Some(Command::Generate(g)) => {
                assert_eq!(g.month, 3);
                assert_eq!(g.year, 2026);
                assert_eq!(g.preset.as_deref(), Some("pwc"));
                assert!((g.days.unwrap() - 10.0).abs() < f64::EPSILON);
                assert!(g.items.is_none());
            }
            other => panic!("Expected Generate, got {other:?}"),
        }
    }

    #[test]
    fn test_generate_single_item_missing_month_is_error() {
        // Arrange
        let args = [
            "invoice", "generate", "--year", "2026",
            "--preset", "pwc", "--days", "10",
        ];

        // Act
        let result = Cli::try_parse_from(args);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_single_item_missing_preset_is_error() {
        // Arrange — month+year but no preset or items
        let args = ["invoice", "generate", "--month", "3", "--year", "2026"];

        // Act
        let result = Cli::try_parse_from(args);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_items_json_parses() {
        // Arrange
        let json = r#"[{"preset":"pwc","days":10}]"#;
        let args = [
            "invoice", "generate", "--month", "3", "--year", "2026",
            "--items", json,
        ];

        // Act
        let cli = Cli::try_parse_from(args).unwrap();

        // Assert
        match cli.command {
            Some(Command::Generate(g)) => {
                assert_eq!(g.items.as_deref(), Some(json));
                assert!(g.preset.is_none());
                assert!(g.days.is_none());
            }
            other => panic!("Expected Generate, got {other:?}"),
        }
    }

    #[test]
    fn test_generate_items_and_preset_mutually_exclusive() {
        // Arrange
        let args = [
            "invoice", "generate", "--month", "3", "--year", "2026",
            "--preset", "pwc", "--days", "10",
            "--items", "[{}]",
        ];

        // Act
        let result = Cli::try_parse_from(args);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_items_and_days_mutually_exclusive() {
        // Arrange
        let args = [
            "invoice", "generate", "--month", "3", "--year", "2026",
            "--items", "[{}]", "--days", "5",
        ];

        // Act
        let result = Cli::try_parse_from(args);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_no_item_source_is_error() {
        // Arrange — month+year only, no preset or items
        let args = ["invoice", "generate", "--month", "3", "--year", "2026"];

        // Act
        let result = Cli::try_parse_from(args);

        // Assert
        assert!(result.is_err());
    }
}
