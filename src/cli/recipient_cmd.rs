use std::io::Write;
use std::path::Path;

use crate::config::types::Recipient;
use crate::config::validator::ValidatedConfig;
use crate::domain::RecipientKey;
use crate::error::AppError;
use crate::setup::prompter::Prompter;
use crate::setup::prompts::{prompt_parsed, prompt_u32_in_range};

/// Format recipients as a table string with columns: Key, Name, Address, Company ID.
///
/// The default recipient is marked with `(default)` appended to its key.
/// Dynamic column widths based on data.
pub fn format_recipient_table(recipients: &[Recipient], default_key: &str) -> String {
    let min_key = 3;
    let min_name = 4;
    let min_addr = 7;
    let min_cid = 10;

    // Compute display keys (with "(default)" suffix for the default)
    let display_keys: Vec<String> = recipients
        .iter()
        .map(|r| {
            let base = r.key.as_ref().map(|k| k.as_str()).unwrap_or("");
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
    let name_w = recipients
        .iter()
        .map(|r| r.name.len())
        .max()
        .unwrap_or(0)
        .max(min_name);
    let addr_w = recipients
        .iter()
        .map(|r| r.address.first().map(|a| a.len()).unwrap_or(1))
        .max()
        .unwrap_or(0)
        .max(min_addr);
    let cid_w = recipients
        .iter()
        .map(|r| r.company_id.as_deref().unwrap_or("-").len())
        .max()
        .unwrap_or(0)
        .max(min_cid);

    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "{:<key_w$}  {:<name_w$}  {:<addr_w$}  {:<cid_w$}\n",
        "Key", "Name", "Address", "Company ID",
    ));

    // Separator
    out.push_str(&format!(
        "{}  {}  {}  {}\n",
        "-".repeat(key_w),
        "-".repeat(name_w),
        "-".repeat(addr_w),
        "-".repeat(cid_w),
    ));

    // Data rows
    for (i, r) in recipients.iter().enumerate() {
        let addr = r.address.first().map(|a| a.as_str()).unwrap_or("-");
        let cid = r.company_id.as_deref().unwrap_or("-");
        out.push_str(&format!(
            "{:<key_w$}  {:<name_w$}  {:<addr_w$}  {:<cid_w$}\n",
            display_keys[i], r.name, addr, cid,
        ));
    }

    out
}

/// Handle `invoice recipient list` — print formatted recipient table.
pub fn handle_recipient_list(
    validated: &ValidatedConfig,
    writer: &mut dyn Write,
) -> Result<(), AppError> {
    let table = format_recipient_table(
        &validated.recipients,
        validated.default_recipient_key.as_str(),
    );
    writer.write_all(table.as_bytes())?;
    Ok(())
}

/// Handle `invoice recipient add` — interactively add a new recipient.
pub fn handle_recipient_add(
    prompter: &dyn Prompter,
    config_path: &Path,
    writer: &mut dyn Write,
) -> Result<(), AppError> {
    use crate::config::loader::{load_config, LoadResult};
    use crate::config::writer::append_recipient;

    // Load config to check for duplicate keys
    let config = match load_config(config_path)? {
        LoadResult::Loaded(c) => *c,
        LoadResult::NotFound => return Err(AppError::ConfigNotFound),
    };

    let existing_recipients = config.recipients.as_deref().unwrap_or_default();

    // Prompt for key, validating shape and rejecting duplicates
    let key = prompt_parsed(
        prompter,
        |p| p.required_text("Recipient key (short identifier):"),
        |raw: String| {
            let candidate = RecipientKey::try_new(raw).map_err(|e| e.to_string())?;
            if existing_recipients
                .iter()
                .any(|r| r.key.as_ref() == Some(&candidate))
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

    let name = prompter.required_text("Company name:")?;
    let address = prompter.multi_line("Address")?;
    let company_id = prompter.optional_text("Company ID (blank to skip):")?;
    let vat_number = prompter.optional_text("VAT number (blank to skip):")?;

    let set_default = prompter.confirm("Set as default recipient?", false)?;

    let key_for_msg = key.clone();
    let recipient = Recipient {
        key: Some(key),
        name,
        address,
        company_id,
        vat_number,
    };

    append_recipient(config_path, recipient, set_default)?;
    writeln!(
        writer,
        "✓ Recipient \"{}\" added to {}",
        key_for_msg.as_str(),
        config_path.display()
    )?;
    Ok(())
}

/// Handle `invoice recipient delete <key>` — confirm and remove a recipient.
pub fn handle_recipient_delete(
    prompter: &dyn Prompter,
    config_path: &Path,
    key: &str,
    writer: &mut dyn Write,
) -> Result<(), AppError> {
    use crate::config::loader::{load_config, LoadResult};
    use crate::config::writer::{remove_recipient, set_default_recipient};

    // Load config to get recipient details for confirmation
    let config = match load_config(config_path)? {
        LoadResult::Loaded(c) => *c,
        LoadResult::NotFound => return Err(AppError::ConfigNotFound),
    };

    let recipients = config.recipients.as_deref().unwrap_or_default();

    // Find the recipient first to get its name for the confirmation prompt
    let recipient = recipients
        .iter()
        .find(|r| r.key.as_ref().is_some_and(|k| k.as_str() == key))
        .ok_or_else(|| AppError::RecipientNotFound {
            key: key.to_string(),
            available: recipients
                .iter()
                .filter_map(|r| r.key.as_ref().map(|k| k.as_str().to_string()))
                .collect(),
        })?;

    // Guard: cannot delete the last recipient
    if recipients.len() <= 1 {
        return Err(AppError::LastRecipient);
    }

    let prompt = format!("Delete recipient \"{}\" ({})?", key, recipient.name);

    if !prompter.confirm(&prompt, false)? {
        return Ok(());
    }

    let is_default = config
        .default_recipient
        .as_ref()
        .is_some_and(|k| k.as_str() == key);

    remove_recipient(config_path, key)?;

    // If deleting the default, reassign
    if is_default {
        // Reload to get remaining recipients
        let updated = match load_config(config_path)? {
            LoadResult::Loaded(c) => *c,
            LoadResult::NotFound => return Err(AppError::ConfigNotFound),
        };
        let remaining = updated.recipients.as_deref().unwrap_or_default();

        if remaining.len() == 1 {
            // Auto-assign the only remaining recipient
            let new_key = remaining[0].key.as_ref().map(|k| k.as_str()).unwrap_or("");
            set_default_recipient(config_path, new_key)?;
        } else if remaining.len() > 1 {
            // Prompt for new default
            prompter.message("\nSelect new default recipient:\n");
            for (i, r) in remaining.iter().enumerate() {
                prompter.message(&format!(
                    "  [{}] {} \u{2014} {}",
                    i + 1,
                    r.key.as_ref().map(|k| k.as_str()).unwrap_or(""),
                    r.name,
                ));
            }
            let max = remaining.len() as u32;
            let choice =
                prompt_u32_in_range(prompter, "Select recipient number:", 1..=max, 1)?;
            let new_key = remaining[choice as usize - 1]
                .key
                .as_ref()
                .map(|k| k.as_str())
                .unwrap_or("");
            set_default_recipient(config_path, new_key)?;
        }
    }

    writeln!(
        writer,
        "✓ Recipient \"{key}\" deleted from {}",
        config_path.display()
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::loader::{load_config, LoadResult};
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};
    use crate::setup::test_helpers::*;

    #[test]
    fn test_format_recipient_table_contains_header_row() {
        // Arrange
        let recipients = vec![synthetic_recipient_acme()];

        // Act
        let output = format_recipient_table(&recipients, "acme");

        // Assert
        assert!(output.contains("Key"), "Missing 'Key' header");
        assert!(output.contains("Name"), "Missing 'Name' header");
        assert!(output.contains("Address"), "Missing 'Address' header");
        assert!(
            output.contains("Company ID"),
            "Missing 'Company ID' header"
        );
    }

    #[test]
    fn test_format_recipient_table_contains_recipient_data() {
        // Arrange
        let recipients = vec![synthetic_recipient_acme()];

        // Act
        let output = format_recipient_table(&recipients, "acme");

        // Assert
        assert!(output.contains("acme"), "Missing key 'acme'");
        assert!(output.contains("Acme Corp"), "Missing name");
        assert!(output.contains("100 Acme Blvd"), "Missing address");
        assert!(output.contains("AC-12345"), "Missing company ID");
    }

    #[test]
    fn test_format_recipient_table_marks_default() {
        // Arrange
        let recipients = vec![synthetic_recipient_acme(), synthetic_recipient_globex()];

        // Act
        let output = format_recipient_table(&recipients, "acme");

        // Assert
        assert!(output.contains("(default)"), "Missing '(default)' marker");
        // The globex line should NOT contain "(default)"
        let lines: Vec<&str> = output.lines().collect();
        let globex_line = lines.iter().find(|l| l.contains("Globex")).unwrap();
        assert!(
            !globex_line.contains("(default)"),
            "Globex should not be marked as default"
        );
    }

    #[test]
    fn test_format_recipient_table_multiple_shows_all() {
        // Arrange
        let recipients = vec![synthetic_recipient_acme(), synthetic_recipient_globex()];

        // Act
        let output = format_recipient_table(&recipients, "acme");

        // Assert
        assert!(output.contains("Acme Corp"), "Missing 'Acme Corp'");
        assert!(output.contains("Globex Inc"), "Missing 'Globex Inc'");
    }

    #[test]
    fn test_format_recipient_table_missing_company_id_shows_placeholder() {
        // Arrange — globex has company_id: None
        let recipients = vec![synthetic_recipient_globex()];

        // Act
        let output = format_recipient_table(&recipients, "globex");

        // Assert
        let lines: Vec<&str> = output.lines().collect();
        let data_line = lines.last().unwrap();
        assert!(
            data_line.contains("-"),
            "Missing '-' placeholder for company ID"
        );
    }

    #[test]
    fn test_format_recipient_table_empty_shows_header_only() {
        // Arrange
        let recipients: Vec<Recipient> = vec![];

        // Act
        let output = format_recipient_table(&recipients, "");

        // Assert
        assert!(output.contains("Key"), "Missing header");
        assert!(!output.contains("Acme"), "Should not contain data");
    }

    #[test]
    fn test_handle_recipient_list_writes_table() {
        // Arrange
        let validated = validated(v2_config_two_recipients());
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_recipient_list(&validated, &mut buf).unwrap();

        // Assert
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Acme Corp"), "Missing 'Acme Corp'");
        assert!(output.contains("Globex Inc"), "Missing 'Globex Inc'");
        assert!(output.contains("(default)"), "Missing default marker");
    }

    // ── handle_recipient_add tests ──

    #[test]
    fn test_handle_recipient_add_happy_path_all_fields() {
        // Arrange
        let config = v2_complete_config();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("newcorp".into()),
            MockResponse::Text("New Corp LLC".into()),
            MockResponse::Lines(vec!["1 New St".into()]),
            MockResponse::OptionalText(Some("NC-123".into())),
            MockResponse::OptionalText(Some("VAT999".into())),
            MockResponse::Confirm(true),
        ]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_recipient_add(&prompter, &cfg_path(&dir), &mut buf).unwrap();

        // Assert
        let loaded = match load_config(&cfg_path(&dir)).unwrap() {
            LoadResult::Loaded(c) => *c,
            _ => panic!("Expected Loaded"),
        };
        let recipients = loaded.recipients.unwrap();
        assert_eq!(recipients.len(), 2);
        assert_eq!(recipients[1].key, Some(RecipientKey::try_new("newcorp").unwrap()));
        assert_eq!(recipients[1].name, "New Corp LLC");
        assert_eq!(recipients[1].company_id, Some("NC-123".into()));
        assert_eq!(recipients[1].vat_number, Some("VAT999".into()));
        assert_eq!(loaded.default_recipient, Some(RecipientKey::try_new("newcorp").unwrap()));
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("added"), "Expected 'added' in output");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_recipient_add_optional_fields_skipped() {
        // Arrange
        let config = v2_complete_config();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("newcorp".into()),
            MockResponse::Text("New Corp".into()),
            MockResponse::Lines(vec!["Street".into()]),
            MockResponse::OptionalText(None),
            MockResponse::OptionalText(None),
            MockResponse::Confirm(false),
        ]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_recipient_add(&prompter, &cfg_path(&dir), &mut buf).unwrap();

        // Assert
        let loaded = match load_config(&cfg_path(&dir)).unwrap() {
            LoadResult::Loaded(c) => *c,
            _ => panic!("Expected Loaded"),
        };
        let recipients = loaded.recipients.unwrap();
        assert_eq!(recipients[1].company_id, None);
        assert_eq!(recipients[1].vat_number, None);
        assert_eq!(loaded.default_recipient, Some(RecipientKey::try_new("acme").unwrap()));
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_recipient_add_duplicate_key_reprompts() {
        // Arrange
        let config = v2_complete_config();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("acme".into()),  // duplicate!
            MockResponse::Text("acme2".into()), // unique
            MockResponse::Text("Acme Two".into()),
            MockResponse::Lines(vec!["Street".into()]),
            MockResponse::OptionalText(None),
            MockResponse::OptionalText(None),
            MockResponse::Confirm(false),
        ]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_recipient_add(&prompter, &cfg_path(&dir), &mut buf).unwrap();

        // Assert
        let loaded = match load_config(&cfg_path(&dir)).unwrap() {
            LoadResult::Loaded(c) => *c,
            _ => panic!("Expected Loaded"),
        };
        let recipients = loaded.recipients.unwrap();
        assert_eq!(recipients.len(), 2);
        assert_eq!(recipients[1].key, Some(RecipientKey::try_new("acme2").unwrap()));
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(
            all.contains("already exists"),
            "Expected 'already exists' message, got: {all}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_recipient_add_prints_confirmation() {
        // Arrange
        let config = v2_complete_config();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("newcorp".into()),
            MockResponse::Text("New Corp".into()),
            MockResponse::Lines(vec!["Street".into()]),
            MockResponse::OptionalText(None),
            MockResponse::OptionalText(None),
            MockResponse::Confirm(false),
        ]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_recipient_add(&prompter, &cfg_path(&dir), &mut buf).unwrap();

        // Assert
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("✓"), "Expected checkmark in output");
        assert!(output.contains("newcorp"), "Expected key in output");
        assert!(
            output.contains("config.yaml"),
            "Expected filename in output"
        );
    }

    #[test]
    fn test_handle_recipient_add_no_config_returns_error() {
        // Arrange
        let dir = setup_dir(None);
        let prompter = MockPrompter::new(vec![]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_recipient_add(&prompter, &cfg_path(&dir), &mut buf);

        // Assert
        assert!(matches!(result, Err(AppError::ConfigNotFound)));
        prompter.assert_exhausted();
    }

    // ── handle_recipient_delete tests ──

    #[test]
    fn test_handle_recipient_delete_confirmed_removes_recipient() {
        // Arrange
        let config = v2_config_two_recipients();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![MockResponse::Confirm(true)]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_recipient_delete(&prompter, &cfg_path(&dir), "globex", &mut buf);

        // Assert
        assert!(result.is_ok());
        let loaded = match load_config(&cfg_path(&dir)).unwrap() {
            LoadResult::Loaded(c) => *c,
            _ => panic!("Expected Loaded"),
        };
        let recipients = loaded.recipients.unwrap();
        assert_eq!(recipients.len(), 1);
        assert_eq!(recipients[0].key, Some(RecipientKey::try_new("acme").unwrap()));
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("deleted"), "Expected 'deleted' in output");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_recipient_delete_user_declines() {
        // Arrange
        let config = v2_config_two_recipients();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![MockResponse::Confirm(false)]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_recipient_delete(&prompter, &cfg_path(&dir), "globex", &mut buf);

        // Assert
        assert!(result.is_ok());
        let loaded = match load_config(&cfg_path(&dir)).unwrap() {
            LoadResult::Loaded(c) => *c,
            _ => panic!("Expected Loaded"),
        };
        let recipients = loaded.recipients.unwrap();
        assert_eq!(recipients.len(), 2);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.is_empty(), "Expected no output on decline");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_recipient_delete_unknown_key_returns_error() {
        // Arrange
        let config = v2_config_two_recipients();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_recipient_delete(&prompter, &cfg_path(&dir), "nope", &mut buf);

        // Assert
        assert!(matches!(result, Err(AppError::RecipientNotFound { .. })));
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_recipient_delete_last_recipient_refused() {
        // Arrange
        let config = v2_complete_config();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_recipient_delete(&prompter, &cfg_path(&dir), "acme", &mut buf);

        // Assert
        assert!(matches!(result, Err(AppError::LastRecipient)));
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_recipient_delete_default_two_recipients_auto_assigns() {
        // Arrange
        let config = v2_config_two_recipients();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![MockResponse::Confirm(true)]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_recipient_delete(&prompter, &cfg_path(&dir), "acme", &mut buf);

        // Assert
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
        let loaded = match load_config(&cfg_path(&dir)).unwrap() {
            LoadResult::Loaded(c) => *c,
            _ => panic!("Expected Loaded"),
        };
        assert_eq!(loaded.default_recipient, Some(RecipientKey::try_new("globex").unwrap()));
        assert_eq!(loaded.recipients.unwrap().len(), 1);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_recipient_delete_default_prompts_new_default() {
        // Arrange
        let mut config = v2_config_two_recipients();
        let mut recipients = config.recipients.take().unwrap();
        recipients.push(Recipient {
            key: Some(RecipientKey::try_new("initech").unwrap()),
            name: "Initech Corp".into(),
            address: vec!["300 Initech Way".into()],
            company_id: None,
            vat_number: None,
        });
        config.recipients = Some(recipients);
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![
            MockResponse::Confirm(true),
            MockResponse::U32(2),
        ]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_recipient_delete(&prompter, &cfg_path(&dir), "acme", &mut buf);

        // Assert
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
        let loaded = match load_config(&cfg_path(&dir)).unwrap() {
            LoadResult::Loaded(c) => *c,
            _ => panic!("Expected Loaded"),
        };
        assert_eq!(loaded.default_recipient, Some(RecipientKey::try_new("initech").unwrap()));
        assert_eq!(loaded.recipients.unwrap().len(), 2);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_recipient_delete_confirmation_includes_key_and_name() {
        // Arrange
        let config = v2_config_two_recipients();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![MockResponse::Confirm(true)]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_recipient_delete(&prompter, &cfg_path(&dir), "globex", &mut buf).unwrap();

        // Assert
        let prompts = prompter.prompts.borrow();
        assert_eq!(prompts.len(), 1);
        assert!(prompts[0].contains("globex"), "Expected 'globex' in prompt: {}", prompts[0]);
        assert!(prompts[0].contains("Globex Inc"), "Expected 'Globex Inc' in prompt: {}", prompts[0]);
    }

    #[test]
    fn test_handle_recipient_delete_no_config_returns_error() {
        // Arrange
        let dir = setup_dir(None);
        let prompter = MockPrompter::new(vec![]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_recipient_delete(&prompter, &cfg_path(&dir), "acme", &mut buf);

        // Assert
        assert!(matches!(result, Err(AppError::ConfigNotFound)));
        prompter.assert_exhausted();
    }
}
