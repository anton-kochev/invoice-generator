use std::io::Write;
use std::path::Path;

use crate::config::types::Recipient;
use crate::config::validator::ValidatedConfig;
use crate::error::AppError;
use crate::setup::prompter::Prompter;

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
            let base = r.key.as_deref().unwrap_or("");
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
    let table = format_recipient_table(&validated.recipients, &validated.default_recipient_key);
    writer.write_all(table.as_bytes())?;
    Ok(())
}

/// Handle `invoice recipient add` — interactively add a new recipient.
/// (Full implementation in Story 7.5)
pub fn handle_recipient_add(
    _prompter: &dyn Prompter,
    _dir: &Path,
    _writer: &mut dyn Write,
) -> Result<(), AppError> {
    todo!("Story 7.5")
}

/// Handle `invoice recipient delete <key>` — confirm and remove a recipient.
/// (Full implementation in Story 7.6)
pub fn handle_recipient_delete(
    _prompter: &dyn Prompter,
    _dir: &Path,
    _key: &str,
    _writer: &mut dyn Write,
) -> Result<(), AppError> {
    todo!("Story 7.6")
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
