use std::path::Path;

use crate::config::loader::{load_config, missing_field_hints, LoadResult, CONFIG_FILENAME};
use crate::config::types::{Config, Recipient, TemplateKey};
use crate::config::validator::{ConfigSection, ValidatedConfig, ValidationOutcome};
use crate::error::AppError;
use crate::setup::prompter::Prompter;
use crate::{invoice, pdf, setup};

use super::recipient_selection::select_recipient;

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
        LoadResult::Loaded(config) => {
            // Print hints about missing optional fields (interactive only)
            if let Ok(raw) = std::fs::read_to_string(cwd.join(CONFIG_FILENAME)) {
                let hints = missing_field_hints(&raw);
                if !hints.is_empty() {
                    eprintln!(
                        "Tip: Your config can use these fields in the \"defaults\" section:"
                    );
                    for hint in &hints {
                        eprintln!("{hint}");
                    }
                }
            }

            match config.validate()? {
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
        }},
    };

    let recipient = select_recipient(prompter, &validated.recipients, &validated.default_recipient_key)?;
    run_invoice_flow(prompter, &validated, &recipient, cwd)
}

/// Run the interactive invoice generation loop.
pub fn run_invoice_flow(
    prompter: &dyn Prompter,
    validated: &ValidatedConfig,
    recipient: &Recipient,
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

        // Show current template and offer to change
        prompter.message(&format!(
            "Template: {} ({})",
            validated.template, validated.template.description()
        ));

        let template = if prompter.confirm("Change template?", false)? {
            let mut list = String::from("\nAvailable templates:\n");
            for (i, t) in TemplateKey::ALL.iter().enumerate() {
                let marker = if *t == validated.template {
                    " (default)"
                } else {
                    ""
                };
                list.push_str(&format!(
                    "  [{}] {} — {}{}\n",
                    i + 1,
                    t,
                    t.description(),
                    marker
                ));
            }
            prompter.message(&list);
            let choice = prompter.u32_with_default("Select template:", 1)?;
            if choice >= 1 && (choice as usize) <= TemplateKey::ALL.len() {
                TemplateKey::ALL[choice as usize - 1]
            } else {
                validated.template
            }
        } else {
            validated.template
        };

        if prompter.confirm("Generate PDF?", true)? {
            let pdf_bytes = pdf::generate_pdf(&summary, validated, recipient, cwd, template, validated.locale)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::*;
    use crate::config::validator::ValidatedBranding;
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};

    fn make_validated_config() -> ValidatedConfig {
        let recipient = Recipient {
            key: Some("acme".into()),
            name: "Acme Corp".into(),
            address: vec!["123 Test St".into()],
            company_id: None,
            vat_number: None,
        };
        ValidatedConfig {
            sender: Sender {
                name: "Test User".into(),
                address: vec!["456 Dev Ave".into()],
                email: "test@example.com".into(),
            },
            recipient: recipient.clone(),
            recipients: vec![recipient],
            default_recipient_key: "acme".into(),
            payment: vec![PaymentMethod {
                label: "SEPA".into(),
                iban: "DE00000000000000".into(),
                bic_swift: "TESTBIC".into(),
            }],
            presets: vec![Preset {
                key: "dev".into(),
                description: "Development".into(),
                default_rate: 800.0,
                currency: None,
                tax_rate: None,
            }],
            defaults: Defaults::default(),
            branding: ValidatedBranding::default(),
            template: TemplateKey::Leda,
            locale: crate::locale::Locale::EnUs,
        }
    }

    fn flow_responses_no_change() -> Vec<MockResponse> {
        vec![
            MockResponse::U32(3),    // month
            MockResponse::U32(2026), // year
            MockResponse::U32(1),    // select preset
            MockResponse::F64(10.0), // days
            MockResponse::F64(800.0), // rate
            MockResponse::Confirm(false), // no more items
            MockResponse::Confirm(false), // don't change template
            MockResponse::Confirm(true),  // generate PDF
        ]
    }

    fn flow_responses_with_change() -> Vec<MockResponse> {
        vec![
            MockResponse::U32(3),
            MockResponse::U32(2026),
            MockResponse::U32(1),
            MockResponse::F64(10.0),
            MockResponse::F64(800.0),
            MockResponse::Confirm(false), // no more items
            MockResponse::Confirm(true),  // yes change template
            MockResponse::U32(2),         // select template #2
            MockResponse::Confirm(true),  // generate PDF
        ]
    }

    #[test]
    fn test_invoice_flow_shows_template_info() {
        // Arrange
        let config = make_validated_config();
        let recipient = config.recipient.clone();
        let dir = tempfile::tempdir().unwrap();
        let prompter = MockPrompter::new(flow_responses_no_change());
        // Act
        run_invoice_flow(&prompter, &config, &recipient, dir.path()).unwrap();
        // Assert
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m.contains("leda")),
            "Should show template info containing 'leda', got: {messages:?}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn test_invoice_flow_decline_change_uses_default_template() {
        // Arrange
        let config = make_validated_config();
        let recipient = config.recipient.clone();
        let dir = tempfile::tempdir().unwrap();
        let prompter = MockPrompter::new(flow_responses_no_change());
        // Act
        let result = run_invoice_flow(&prompter, &config, &recipient, dir.path());
        // Assert
        assert!(result.is_ok(), "Should succeed with default template");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_invoice_flow_accept_change_shows_template_list() {
        // Arrange
        let config = make_validated_config();
        let recipient = config.recipient.clone();
        let dir = tempfile::tempdir().unwrap();
        let prompter = MockPrompter::new(flow_responses_with_change());
        // Act
        run_invoice_flow(&prompter, &config, &recipient, dir.path()).unwrap();
        // Assert
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m.contains("callisto") && m.contains("leda") && m.contains("thebe")),
            "Should show all template names, got: {messages:?}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn test_invoice_flow_template_change_does_not_modify_config() {
        // Arrange
        let config = make_validated_config();
        let recipient = config.recipient.clone();
        let dir = tempfile::tempdir().unwrap();
        let prompter = MockPrompter::new(flow_responses_with_change());
        // Act
        run_invoice_flow(&prompter, &config, &recipient, dir.path()).unwrap();
        // Assert
        assert_eq!(config.template, TemplateKey::Leda, "Config should not be modified");
        prompter.assert_exhausted();
    }
}
