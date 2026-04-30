pub mod common;
pub mod generate_cmd;
pub mod interactive;
pub mod preset_cmd;
pub mod recipient_cmd;
pub mod recipient_selection;

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

use crate::config::loader::{load_config, LoadResult};
use crate::config::validator::{ValidatedConfig, ValidationOutcome};
use crate::error::AppError;

/// Invoice Generator CLI
#[derive(Parser)]
#[command(name = "invoice", version, about = "Generate professional invoices")]
pub struct Cli {
    /// Path to config file (overrides INVOICE_GENERATOR_CONFIG and the XDG default)
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,

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
    /// Manage recipient profiles
    Recipient {
        #[command(subcommand)]
        action: RecipientAction,
    },
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
    /// Recipient profile key (defaults to default_recipient)
    #[arg(long)]
    pub client: Option<String>,
    /// Template to use for PDF generation (overrides config default)
    #[arg(long)]
    pub template: Option<String>,
    /// Locale for PDF formatting (overrides config default)
    #[arg(long)]
    pub locale: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum RecipientAction {
    /// List all configured recipients
    List,
    /// Add a new recipient
    Add,
    /// Delete a recipient by key
    Delete {
        /// The recipient key to delete
        key: String,
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
pub fn load_validated_config(config_path: &Path) -> Result<ValidatedConfig, AppError> {
    match load_config(config_path)? {
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

    #[test]
    fn test_generate_client_flag_parses() {
        // Arrange
        let args = [
            "invoice", "generate", "--month", "3", "--year", "2026",
            "--preset", "dev", "--days", "10", "--client", "acme",
        ];

        // Act
        let cli = Cli::try_parse_from(args).unwrap();

        // Assert
        match cli.command {
            Some(Command::Generate(g)) => {
                assert_eq!(g.client.as_deref(), Some("acme"));
            }
            other => panic!("Expected Generate, got {other:?}"),
        }
    }

    #[test]
    fn test_generate_without_client_flag_defaults_to_none() {
        // Arrange
        let args = [
            "invoice", "generate", "--month", "3", "--year", "2026",
            "--preset", "dev", "--days", "10",
        ];

        // Act
        let cli = Cli::try_parse_from(args).unwrap();

        // Assert
        match cli.command {
            Some(Command::Generate(g)) => {
                assert!(g.client.is_none());
            }
            other => panic!("Expected Generate, got {other:?}"),
        }
    }

    // ── Recipient subcommand tests ──

    #[test]
    fn test_generate_template_flag_parses() {
        // Arrange
        let args = ["invoice", "generate", "--month", "3", "--year", "2026", "--preset", "dev", "--days", "10", "--template", "amalthea"];
        // Act
        let cli = Cli::try_parse_from(args).unwrap();
        // Assert
        match cli.command.unwrap() {
            Command::Generate(g) => assert_eq!(g.template, Some("amalthea".into())),
            _ => panic!("Expected Generate"),
        }
    }

    #[test]
    fn test_generate_without_template_flag_defaults_to_none() {
        // Arrange
        let args = ["invoice", "generate", "--month", "3", "--year", "2026", "--preset", "dev", "--days", "10"];
        // Act
        let cli = Cli::try_parse_from(args).unwrap();
        // Assert
        match cli.command.unwrap() {
            Command::Generate(g) => assert!(g.template.is_none()),
            _ => panic!("Expected Generate"),
        }
    }

    #[test]
    fn test_generate_locale_flag_parses() {
        // Arrange
        let args = ["invoice", "generate", "--month", "3", "--year", "2026", "--preset", "dev", "--days", "10", "--locale", "de-DE"];
        // Act
        let cli = Cli::try_parse_from(args).unwrap();
        // Assert
        match cli.command.unwrap() {
            Command::Generate(g) => assert_eq!(g.locale, Some("de-DE".into())),
            _ => panic!("Expected Generate"),
        }
    }

    #[test]
    fn test_generate_without_locale_flag_defaults_to_none() {
        // Arrange
        let args = ["invoice", "generate", "--month", "3", "--year", "2026", "--preset", "dev", "--days", "10"];
        // Act
        let cli = Cli::try_parse_from(args).unwrap();
        // Assert
        match cli.command.unwrap() {
            Command::Generate(g) => assert!(g.locale.is_none()),
            _ => panic!("Expected Generate"),
        }
    }

    #[test]
    fn test_generate_template_with_items_mode_parses() {
        // Arrange
        let args = ["invoice", "generate", "--month", "3", "--year", "2026", "--items", r#"[{"preset":"dev","days":5}]"#, "--template", "thebe"];
        // Act
        let cli = Cli::try_parse_from(args).unwrap();
        // Assert
        match cli.command.unwrap() {
            Command::Generate(g) => assert_eq!(g.template, Some("thebe".into())),
            _ => panic!("Expected Generate"),
        }
    }

    #[test]
    fn test_recipient_list_parses() {
        // Arrange
        let args = ["invoice", "recipient", "list"];

        // Act
        let cli = Cli::try_parse_from(args).unwrap();

        // Assert
        assert!(matches!(
            cli.command,
            Some(Command::Recipient {
                action: RecipientAction::List
            })
        ));
    }

    #[test]
    fn test_recipient_add_parses() {
        // Arrange
        let args = ["invoice", "recipient", "add"];

        // Act
        let cli = Cli::try_parse_from(args).unwrap();

        // Assert
        assert!(matches!(
            cli.command,
            Some(Command::Recipient {
                action: RecipientAction::Add
            })
        ));
    }

    #[test]
    fn test_recipient_delete_parses_key() {
        // Arrange
        let args = ["invoice", "recipient", "delete", "acme"];

        // Act
        let cli = Cli::try_parse_from(args).unwrap();

        // Assert
        match cli.command {
            Some(Command::Recipient {
                action: RecipientAction::Delete { key },
            }) => assert_eq!(key, "acme"),
            other => panic!("Expected Recipient Delete, got {other:?}"),
        }
    }

    #[test]
    fn test_recipient_delete_missing_key_is_error() {
        // Arrange
        let args = ["invoice", "recipient", "delete"];

        // Act
        let result = Cli::try_parse_from(args);

        // Assert
        assert!(result.is_err());
    }
}
