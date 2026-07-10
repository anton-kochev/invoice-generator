//! Sender CRUD handlers — list, add, and delete senders managed in `config.yaml`.
//!
//! Mirrors [`crate::cli::recipient_cmd`] 1:1 for the sender case. `Sender` has
//! fewer fields than `Recipient` (no `company_id` / `vat_number`), so the table
//! is four columns wide (`Key | Name | Address | Email`) and the add flow has
//! two fewer prompts.

use std::io::Write;
use std::path::Path;

use crate::cli::CliError;
use crate::config::ConfigError;
use crate::config::types::Sender;
use crate::config::validator::{ValidatedConfig, ValidatedSender};
use crate::domain::SenderKey;
use crate::error::AppError;
use crate::setup::prompter::Prompter;
use crate::setup::prompts::{prompt_parsed, prompt_u32_in_range};

/// Format senders as a table string with columns: Key, Name, Address, Email.
///
/// The default sender is marked with `(default)` appended to its key.
/// Dynamic column widths based on data.
pub fn format_sender_table(senders: &[ValidatedSender], default_key: &str) -> String {
    let min_key = 3;
    let min_name = 4;
    let min_addr = 7;
    let min_email = 5;

    // Compute display keys (with "(default)" suffix for the default)
    let display_keys: Vec<String> = senders
        .iter()
        .map(|s| {
            let base = s.key().as_str();
            if base == default_key {
                format!("{base} (default)")
            } else {
                base.to_string()
            }
        })
        .collect();

    let key_w = display_keys
        .iter()
        .map(|k| k.len())
        .max()
        .unwrap_or(0)
        .max(min_key);
    let name_w = senders
        .iter()
        .map(|s| s.name().len())
        .max()
        .unwrap_or(0)
        .max(min_name);
    let addr_w = senders
        .iter()
        .map(|s| s.address().first().map(|a| a.len()).unwrap_or(1))
        .max()
        .unwrap_or(0)
        .max(min_addr);
    let email_w = senders
        .iter()
        .map(|s| s.email().len())
        .max()
        .unwrap_or(0)
        .max(min_email);

    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "{:<key_w$}  {:<name_w$}  {:<addr_w$}  {:<email_w$}\n",
        "Key", "Name", "Address", "Email",
    ));

    // Separator
    out.push_str(&format!(
        "{}  {}  {}  {}\n",
        "-".repeat(key_w),
        "-".repeat(name_w),
        "-".repeat(addr_w),
        "-".repeat(email_w),
    ));

    // Data rows
    for (i, s) in senders.iter().enumerate() {
        let addr = s.address().first().map(|a| a.as_str()).unwrap_or("-");
        out.push_str(&format!(
            "{:<key_w$}  {:<name_w$}  {:<addr_w$}  {:<email_w$}\n",
            display_keys[i],
            s.name(),
            addr,
            s.email(),
        ));
    }

    out
}

/// Handle `invoice sender list` — print formatted sender table.
pub fn handle_sender_list(
    validated: &ValidatedConfig,
    writer: &mut dyn Write,
) -> Result<(), AppError> {
    let table = format_sender_table(&validated.senders, validated.default_sender_key().as_str());
    writer
        .write_all(table.as_bytes())
        .map_err(CliError::OutputWrite)?;
    Ok(())
}

/// Handle `invoice sender add` — interactively add a new sender.
pub fn handle_sender_add(
    prompter: &dyn Prompter,
    config_path: &Path,
    writer: &mut dyn Write,
) -> Result<(), AppError> {
    use crate::config::loader::{LoadResult, load_config};
    use crate::config::writer::append_sender;

    // Load config to check for duplicate keys
    let config = match load_config(config_path)? {
        LoadResult::Loaded(c) => *c,
        LoadResult::NotFound => return Err(ConfigError::NotFound.into()),
    };

    let existing_senders = config.senders.as_deref().unwrap_or_default();

    // Prompt for key, validating shape and rejecting duplicates
    let key = prompt_parsed(
        prompter,
        |p| p.required_text("Sender key (short identifier):"),
        |raw: String| {
            let candidate = SenderKey::try_new(raw).map_err(|e| e.to_string())?;
            if existing_senders
                .iter()
                .any(|s| s.key.as_ref() == Some(&candidate))
            {
                Err(format!(
                    "Key \"{}\" already exists. Choose another:",
                    candidate.as_str()
                ))
            } else {
                Ok(candidate)
            }
        },
    )?;

    let name = prompter.required_text("Name:")?;
    // Sender's mailing address may be absent (e.g. individual freelancers). Unlike
    // recipient add — recipient address is the billing destination — we let the user
    // press Enter on line 1 to skip.
    let address = prompter.optional_multi_line("Address")?;
    let email = prompter.required_text("Email:")?;

    let set_default = prompter.confirm("Set as default sender?", false)?;

    let key_for_msg = key.clone();
    let sender = Sender {
        key: Some(key),
        name,
        address,
        email,
        extras: None,
    };

    append_sender(config_path, sender, set_default)?;
    writeln!(
        writer,
        "✓ Sender \"{}\" added to {}",
        key_for_msg.as_str(),
        config_path.display()
    )
    .map_err(CliError::OutputWrite)?;
    Ok(())
}

/// Handle `invoice sender delete <key>` — confirm and remove a sender.
pub fn handle_sender_delete(
    prompter: &dyn Prompter,
    config_path: &Path,
    key: &str,
    writer: &mut dyn Write,
) -> Result<(), AppError> {
    use crate::config::loader::{LoadResult, load_config};
    use crate::config::writer::{remove_sender, set_default_sender};

    // Load config to get sender details for confirmation
    let config = match load_config(config_path)? {
        LoadResult::Loaded(c) => *c,
        LoadResult::NotFound => return Err(ConfigError::NotFound.into()),
    };

    let senders = config.senders.as_deref().unwrap_or_default();

    // Find the sender first to get its name for the confirmation prompt
    let sender = senders
        .iter()
        .find(|s| s.key.as_ref().is_some_and(|k| k.as_str() == key))
        .ok_or_else(|| ConfigError::SenderNotFound {
            key: key.to_string(),
            available: senders
                .iter()
                .filter_map(|s| s.key.as_ref().map(|k| k.as_str().to_string()))
                .collect(),
        })?;

    // Guard: cannot delete the last sender
    if senders.len() <= 1 {
        return Err(ConfigError::LastSender.into());
    }

    let prompt = format!("Delete sender \"{}\" ({})?", key, sender.name);

    if !prompter.confirm(&prompt, false)? {
        return Ok(());
    }

    let is_default = config
        .default_sender
        .as_ref()
        .is_some_and(|k| k.as_str() == key);

    remove_sender(config_path, key)?;

    // If deleting the default, reassign
    if is_default {
        // Reload to get remaining senders
        let updated = match load_config(config_path)? {
            LoadResult::Loaded(c) => *c,
            LoadResult::NotFound => return Err(ConfigError::NotFound.into()),
        };
        let remaining = updated.senders.as_deref().unwrap_or_default();

        if remaining.len() == 1 {
            // Auto-assign the only remaining sender
            let new_key = remaining[0].key.as_ref().map(|k| k.as_str()).unwrap_or("");
            set_default_sender(config_path, new_key)?;
        } else if remaining.len() > 1 {
            // Prompt for new default
            prompter.message("\nSelect new default sender:\n");
            for (i, s) in remaining.iter().enumerate() {
                prompter.message(&format!(
                    "  [{}] {} \u{2014} {}",
                    i + 1,
                    s.key.as_ref().map(|k| k.as_str()).unwrap_or(""),
                    s.name,
                ));
            }
            let max = remaining.len() as u32;
            let choice = prompt_u32_in_range(prompter, "Select sender number:", 1..=max, 1)?;
            let new_key = remaining[choice as usize - 1]
                .key
                .as_ref()
                .map(|k| k.as_str())
                .unwrap_or("");
            set_default_sender(config_path, new_key)?;
        }
    }

    writeln!(
        writer,
        "✓ Sender \"{key}\" deleted from {}",
        config_path.display()
    )
    .map_err(CliError::OutputWrite)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::loader::{LoadResult, load_config};
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};
    use crate::setup::test_helpers::*;

    // ── format_sender_table tests ──

    #[test]
    fn test_format_sender_table_contains_header_row() {
        // Arrange
        let senders = vec![synthetic_validated_alice()];

        // Act
        let output = format_sender_table(&senders, "alice");

        // Assert
        assert!(output.contains("Key"), "Missing 'Key' header");
        assert!(output.contains("Name"), "Missing 'Name' header");
        assert!(output.contains("Address"), "Missing 'Address' header");
        assert!(output.contains("Email"), "Missing 'Email' header");
    }

    #[test]
    fn test_format_sender_table_contains_sender_data() {
        // Arrange
        let senders = vec![synthetic_validated_alice()];

        // Act
        let output = format_sender_table(&senders, "alice");

        // Assert
        assert!(output.contains("alice"), "Missing key 'alice'");
        assert!(output.contains("Alice Smith"), "Missing name");
        assert!(output.contains("42 Elm St"), "Missing address");
        assert!(output.contains("alice@example.com"), "Missing email");
    }

    #[test]
    fn test_format_sender_table_marks_default() {
        // Arrange
        let senders = vec![synthetic_validated_alice(), synthetic_validated_bob()];

        // Act
        let output = format_sender_table(&senders, "alice");

        // Assert
        assert!(output.contains("(default)"), "Missing '(default)' marker");
        // The bob line should NOT contain "(default)"
        let lines: Vec<&str> = output.lines().collect();
        let bob_line = lines.iter().find(|l| l.contains("Bob Jones")).unwrap();
        assert!(
            !bob_line.contains("(default)"),
            "Bob should not be marked as default"
        );
    }

    #[test]
    fn test_format_sender_table_multiple_shows_all() {
        // Arrange
        let senders = vec![synthetic_validated_alice(), synthetic_validated_bob()];

        // Act
        let output = format_sender_table(&senders, "alice");

        // Assert
        assert!(output.contains("Alice Smith"), "Missing 'Alice Smith'");
        assert!(output.contains("Bob Jones"), "Missing 'Bob Jones'");
    }

    #[test]
    fn test_format_sender_table_empty_shows_header_only() {
        // Arrange
        let senders: Vec<ValidatedSender> = vec![];

        // Act
        let output = format_sender_table(&senders, "");

        // Assert
        assert!(output.contains("Key"), "Missing header");
        assert!(!output.contains("Alice"), "Should not contain data");
    }

    // ── handle_sender_list tests ──

    #[test]
    fn test_handle_sender_list_writes_table() {
        // Arrange
        let validated = validated(v2_config_two_senders());
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_sender_list(&validated, &mut buf).unwrap();

        // Assert
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Alice Smith"), "Missing 'Alice Smith'");
        assert!(output.contains("Bob Jones"), "Missing 'Bob Jones'");
        assert!(output.contains("(default)"), "Missing default marker");
    }

    // ── handle_sender_add tests ──

    #[test]
    fn test_handle_sender_add_happy_path_all_fields() {
        // Arrange
        let config = v2_complete_config_with_senders();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("carol".into()),
            MockResponse::Text("Carol King".into()),
            MockResponse::OptionalLines(vec!["19 Birch Road".into()]),
            MockResponse::Text("carol@example.com".into()),
            MockResponse::Confirm(true),
        ]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_sender_add(&prompter, &cfg_path(&dir), &mut buf).unwrap();

        // Assert
        let loaded = match load_config(&cfg_path(&dir)).unwrap() {
            LoadResult::Loaded(c) => *c,
            _ => panic!("Expected Loaded"),
        };
        let senders = loaded.senders.unwrap();
        assert_eq!(senders.len(), 2);
        assert_eq!(senders[1].key, Some(SenderKey::try_new("carol").unwrap()));
        assert_eq!(senders[1].name, "Carol King");
        assert_eq!(senders[1].email, "carol@example.com");
        assert_eq!(
            loaded.default_sender,
            Some(SenderKey::try_new("carol").unwrap())
        );
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("added"), "Expected 'added' in output");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_sender_add_duplicate_key_reprompts() {
        // Arrange
        let config = v2_complete_config_with_senders();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("alice".into()),  // duplicate!
            MockResponse::Text("alice2".into()), // unique
            MockResponse::Text("Alice Two".into()),
            MockResponse::OptionalLines(vec!["Street".into()]),
            MockResponse::Text("alice2@example.com".into()),
            MockResponse::Confirm(false),
        ]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_sender_add(&prompter, &cfg_path(&dir), &mut buf).unwrap();

        // Assert
        let loaded = match load_config(&cfg_path(&dir)).unwrap() {
            LoadResult::Loaded(c) => *c,
            _ => panic!("Expected Loaded"),
        };
        let senders = loaded.senders.unwrap();
        assert_eq!(senders.len(), 2);
        assert_eq!(senders[1].key, Some(SenderKey::try_new("alice2").unwrap()));
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(
            all.contains("already exists"),
            "Expected 'already exists' message, got: {all}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_sender_add_prints_confirmation() {
        // Arrange
        let config = v2_complete_config_with_senders();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("carol".into()),
            MockResponse::Text("Carol King".into()),
            MockResponse::OptionalLines(vec!["Street".into()]),
            MockResponse::Text("carol@example.com".into()),
            MockResponse::Confirm(false),
        ]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_sender_add(&prompter, &cfg_path(&dir), &mut buf).unwrap();

        // Assert
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("✓"), "Expected checkmark in output");
        assert!(output.contains("carol"), "Expected key in output");
        assert!(
            output.contains("config.yaml"),
            "Expected filename in output"
        );
    }

    #[test]
    fn test_handle_sender_add_blank_address_omits_address_in_yaml() {
        // Arrange
        let config = v2_complete_config_with_senders();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("carol".into()),
            MockResponse::Text("Carol King".into()),
            MockResponse::OptionalLines(vec![]),
            MockResponse::Text("carol@example.com".into()),
            MockResponse::Confirm(false),
        ]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_sender_add(&prompter, &cfg_path(&dir), &mut buf).unwrap();

        // Assert — locate the carol sender block in raw YAML and scan its lines
        // for an `address:` key. Substring matching is unsafe because recipients
        // also have `address:` fields.
        let yaml = std::fs::read_to_string(cfg_path(&dir)).unwrap();
        let lines: Vec<&str> = yaml.lines().collect();
        let start = lines
            .iter()
            .position(|l| l.contains("key: carol"))
            .expect("Expected `key: carol` line in YAML");
        // Scan forward until we hit the next `- key:` (start of next list item)
        // or the end of the senders block. Use indentation/list-marker pattern
        // as the terminator.
        let mut end = lines.len();
        for (i, l) in lines.iter().enumerate().skip(start + 1) {
            let trimmed = l.trim_start();
            if trimmed.starts_with("- key:") || (!l.starts_with(' ') && !l.starts_with('-')) {
                end = i;
                break;
            }
        }
        let block = &lines[start..end];
        let has_address = block.iter().any(|l| l.trim_start().starts_with("address:"));
        assert!(
            !has_address,
            "Expected no `address:` field in carol block, got:\n{}",
            block.join("\n")
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_sender_add_blank_address_loads_back_as_empty_vec() {
        // Arrange
        let config = v2_complete_config_with_senders();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("carol".into()),
            MockResponse::Text("Carol King".into()),
            MockResponse::OptionalLines(vec![]),
            MockResponse::Text("carol@example.com".into()),
            MockResponse::Confirm(false),
        ]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_sender_add(&prompter, &cfg_path(&dir), &mut buf).unwrap();

        // Assert
        let loaded = match load_config(&cfg_path(&dir)).unwrap() {
            LoadResult::Loaded(c) => *c,
            _ => panic!("Expected Loaded"),
        };
        let senders = loaded.senders.unwrap();
        let carol_key = SenderKey::try_new("carol").unwrap();
        let carol = senders
            .iter()
            .find(|s| s.key.as_ref() == Some(&carol_key))
            .expect("Expected carol sender in loaded config");
        assert!(carol.address.is_empty());
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_sender_add_no_config_returns_error() {
        // Arrange
        let dir = setup_dir(None);
        let prompter = MockPrompter::new(vec![]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_sender_add(&prompter, &cfg_path(&dir), &mut buf);

        // Assert
        assert!(matches!(
            result,
            Err(AppError::Config(ConfigError::NotFound))
        ));
        prompter.assert_exhausted();
    }

    // ── handle_sender_delete tests ──

    #[test]
    fn test_handle_sender_delete_confirmed_removes_sender() {
        // Arrange
        let config = v2_config_two_senders();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![MockResponse::Confirm(true)]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_sender_delete(&prompter, &cfg_path(&dir), "bob", &mut buf);

        // Assert
        assert!(result.is_ok());
        let loaded = match load_config(&cfg_path(&dir)).unwrap() {
            LoadResult::Loaded(c) => *c,
            _ => panic!("Expected Loaded"),
        };
        let senders = loaded.senders.unwrap();
        assert_eq!(senders.len(), 1);
        assert_eq!(senders[0].key, Some(SenderKey::try_new("alice").unwrap()));
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("deleted"), "Expected 'deleted' in output");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_sender_delete_user_declines() {
        // Arrange
        let config = v2_config_two_senders();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![MockResponse::Confirm(false)]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_sender_delete(&prompter, &cfg_path(&dir), "bob", &mut buf);

        // Assert
        assert!(result.is_ok());
        let loaded = match load_config(&cfg_path(&dir)).unwrap() {
            LoadResult::Loaded(c) => *c,
            _ => panic!("Expected Loaded"),
        };
        let senders = loaded.senders.unwrap();
        assert_eq!(senders.len(), 2);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.is_empty(), "Expected no output on decline");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_sender_delete_unknown_key_returns_error() {
        // Arrange
        let config = v2_config_two_senders();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_sender_delete(&prompter, &cfg_path(&dir), "nope", &mut buf);

        // Assert
        assert!(matches!(
            result,
            Err(AppError::Config(ConfigError::SenderNotFound { .. }))
        ));
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_sender_delete_last_sender_refused() {
        // Arrange
        let config = v2_complete_config_with_senders();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_sender_delete(&prompter, &cfg_path(&dir), "alice", &mut buf);

        // Assert
        assert!(matches!(
            result,
            Err(AppError::Config(ConfigError::LastSender))
        ));
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_sender_delete_default_two_senders_auto_assigns() {
        // Arrange
        let config = v2_config_two_senders();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![MockResponse::Confirm(true)]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_sender_delete(&prompter, &cfg_path(&dir), "alice", &mut buf);

        // Assert
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
        let loaded = match load_config(&cfg_path(&dir)).unwrap() {
            LoadResult::Loaded(c) => *c,
            _ => panic!("Expected Loaded"),
        };
        assert_eq!(
            loaded.default_sender,
            Some(SenderKey::try_new("bob").unwrap())
        );
        assert_eq!(loaded.senders.unwrap().len(), 1);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_sender_delete_default_prompts_new_default() {
        // Arrange
        let mut config = v2_config_two_senders();
        let mut senders = config.senders.take().unwrap();
        senders.push(Sender {
            key: Some(SenderKey::try_new("carol").unwrap()),
            name: "Carol King".into(),
            address: vec!["19 Birch Road".into()],
            email: "carol@example.com".into(),
            extras: None,
        });
        config.senders = Some(senders);
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![MockResponse::Confirm(true), MockResponse::U32(2)]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_sender_delete(&prompter, &cfg_path(&dir), "alice", &mut buf);

        // Assert
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
        let loaded = match load_config(&cfg_path(&dir)).unwrap() {
            LoadResult::Loaded(c) => *c,
            _ => panic!("Expected Loaded"),
        };
        assert_eq!(
            loaded.default_sender,
            Some(SenderKey::try_new("carol").unwrap())
        );
        assert_eq!(loaded.senders.unwrap().len(), 2);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_sender_delete_confirmation_includes_key_and_name() {
        // Arrange
        let config = v2_config_two_senders();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![MockResponse::Confirm(true)]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_sender_delete(&prompter, &cfg_path(&dir), "bob", &mut buf).unwrap();

        // Assert
        let prompts = prompter.prompts.borrow();
        assert_eq!(prompts.len(), 1);
        assert!(
            prompts[0].contains("bob"),
            "Expected 'bob' in prompt: {}",
            prompts[0]
        );
        assert!(
            prompts[0].contains("Bob Jones"),
            "Expected 'Bob Jones' in prompt: {}",
            prompts[0]
        );
    }

    #[test]
    fn test_handle_sender_delete_no_config_returns_error() {
        // Arrange
        let dir = setup_dir(None);
        let prompter = MockPrompter::new(vec![]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_sender_delete(&prompter, &cfg_path(&dir), "alice", &mut buf);

        // Assert
        assert!(matches!(
            result,
            Err(AppError::Config(ConfigError::NotFound))
        ));
        prompter.assert_exhausted();
    }
}
