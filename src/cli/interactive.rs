use std::path::Path;

use crate::config::loader::{LoadResult, load_config, missing_field_hints};
use crate::config::types::Config;
use crate::config::validator::{
    ConfigSection, ValidatedConfig, ValidatedRecipient, ValidationOutcome,
};
use crate::error::AppError;
use crate::pdf::manifest::{self, ManifestEntry};
use crate::pdf::registry::{Template, TemplateRegistry};
use crate::pdf::remote;
use crate::pdf::PdfError;
use crate::setup::prompter::Prompter;
use crate::{invoice, pdf, setup};

use super::recipient_selection::select_recipient;

/// The full v1.0 interactive flow: load config → maybe setup → invoice → PDF.
pub fn run_interactive(
    prompter: &dyn Prompter,
    config_path: &Path,
    output_dir: &Path,
) -> Result<(), AppError> {
    // Best-effort: copy bundled built-in templates into the user's config dir
    // on every run. Never overwrites existing files.
    if let Err(e) = TemplateRegistry::write_builtins_if_missing() {
        eprintln!("Warning: could not seed bundled templates: {e}");
    }
    // Best-effort: seed the manifest cache from the bundled JSON if missing.
    if let Err(e) = manifest::load_cache_or_seed() {
        eprintln!("Warning: could not seed template manifest cache: {e}");
    }

    let validated = match load_config(config_path)? {
        LoadResult::NotFound => {
            let mut config = Config::default();
            let all_missing = vec![
                ConfigSection::Sender,
                ConfigSection::Recipient,
                ConfigSection::Payment,
                ConfigSection::Presets,
            ];
            setup::run_setup(prompter, &mut config, &all_missing, config_path)?;
            match config.validate()? {
                ValidationOutcome::Complete(v) => v,
                ValidationOutcome::Incomplete { .. } => {
                    unreachable!("Setup completed but config still incomplete")
                }
            }
        }
        LoadResult::Loaded(config) => {
            // Print hints about missing optional fields (interactive only)
            if let Ok(raw) = std::fs::read_to_string(config_path) {
                let hints = missing_field_hints(&raw);
                if !hints.is_empty() {
                    eprintln!("Tip: Your config can use these fields in the \"defaults\" section:");
                    for hint in &hints {
                        eprintln!("{hint}");
                    }
                }
            }

            match (*config).validate()? {
                ValidationOutcome::Complete(v) => {
                    println!("Config loaded successfully.");
                    println!("Sender: {}", v.sender.name);
                    println!("Recipient: {}", v.default_recipient().name());
                    v
                }
                ValidationOutcome::Incomplete {
                    mut config,
                    missing,
                } => {
                    setup::run_setup(prompter, &mut config, &missing, config_path)?;
                    match config.validate()? {
                        ValidationOutcome::Complete(v) => v,
                        ValidationOutcome::Incomplete { .. } => {
                            unreachable!("Setup completed but config still incomplete")
                        }
                    }
                }
            }
        }
    };

    let recipient = select_recipient(
        prompter,
        &validated.recipients,
        validated.default_recipient_key().as_str(),
    )?;
    run_invoice_flow(prompter, &validated, &recipient, config_path, output_dir)
}

/// Resolve the configured default template name to an installed [`Template`],
/// or fall back to the first installed template if the configured slug is
/// missing.
fn resolve_default_template(
    registry: &TemplateRegistry,
    configured: &str,
) -> Option<Template> {
    registry
        .find_by_name(configured)
        .cloned()
        .or_else(|| registry.templates().first().cloned())
}

/// Show the local-templates prompt and return the user's choice. Returns
/// `Ok(Some(template))` on a normal pick, or `Ok(None)` if the user chose
/// to browse remote templates (caller should switch flows).
fn pick_local_template(
    prompter: &dyn Prompter,
    registry: &TemplateRegistry,
    current: &Template,
) -> Result<Option<Template>, AppError> {
    let mut list = String::from("\nAvailable templates:\n");
    let templates = registry.templates();
    for (i, t) in templates.iter().enumerate() {
        let marker = if t.name == current.name {
            " (default)"
        } else {
            ""
        };
        let desc = t
            .description
            .as_deref()
            .map(|d| format!(" — {d}"))
            .unwrap_or_default();
        list.push_str(&format!("  [{}] {}{}{}\n", i + 1, t.name, desc, marker));
    }
    let browse_idx = templates.len() + 1;
    list.push_str(&format!(
        "  [{browse_idx}] Browse remote templates…\n"
    ));
    prompter.message(&list);

    let choice = prompter.u32_with_default("Select template:", 1)?;
    if choice as usize == browse_idx {
        return Ok(None);
    }
    if (choice as usize) >= 1 && (choice as usize) <= templates.len() {
        Ok(Some(templates[choice as usize - 1].clone()))
    } else {
        // Out-of-range — keep the current template.
        Ok(Some(current.clone()))
    }
}

/// Show the remote-templates prompt: load manifest, filter to entries not yet
/// installed locally, prompt the user, install on selection. Returns the
/// installed template, or `None` if the user backed out.
fn pick_remote_template(
    prompter: &dyn Prompter,
    registry: &mut TemplateRegistry,
) -> Result<Option<Template>, AppError> {
    // Always try the cached manifest first; refresh from upstream as a
    // fallback if the cache yields nothing useful.
    let manifest = match manifest::load_cache_or_seed() {
        Ok(m) => m,
        Err(_) => {
            prompter.message("Fetching template manifest from upstream…");
            match remote::fetch_manifest() {
                Ok(m) => {
                    if let Err(e) = manifest::write_cache(&m) {
                        eprintln!("Warning: could not cache fetched manifest: {e}");
                    }
                    m
                }
                Err(e) => {
                    prompter.message(&format!("Could not fetch remote manifest: {e}"));
                    return Ok(None);
                }
            }
        }
    };

    let installed: Vec<String> = registry.names();
    let remote_only: Vec<&ManifestEntry> = manifest
        .templates
        .iter()
        .filter(|e| !installed.iter().any(|n| n == &e.name))
        .collect();

    if remote_only.is_empty() {
        prompter.message("No remote templates available beyond what's already installed.");
        return Ok(None);
    }

    let mut list = String::from("\nRemote templates available for install:\n");
    for (i, entry) in remote_only.iter().enumerate() {
        list.push_str(&format!(
            "  [{}] {} — {}\n",
            i + 1,
            entry.name,
            entry.description
        ));
    }
    let back_idx = remote_only.len() + 1;
    list.push_str(&format!("  [{back_idx}] ← Back\n"));
    prompter.message(&list);

    let choice = prompter.u32_with_default("Select remote template:", 1)?;
    if choice as usize == back_idx {
        return Ok(None);
    }
    let idx = (choice as usize).saturating_sub(1);
    if idx >= remote_only.len() {
        return Ok(None);
    }
    let name = remote_only[idx].name.clone();
    prompter.message(&format!("Installing template '{name}' from upstream…"));
    match registry.install_from_remote(&name) {
        Ok(t) => {
            let installed = t.clone();
            prompter.message(&format!("Installed template '{}'.", installed.name));
            Ok(Some(installed))
        }
        Err(e) => {
            prompter.message(&format!("Could not install '{name}': {e}"));
            Ok(None)
        }
    }
}

/// Run the interactive invoice generation loop.
pub fn run_invoice_flow(
    prompter: &dyn Prompter,
    validated: &ValidatedConfig,
    recipient: &ValidatedRecipient,
    config_path: &Path,
    output_dir: &Path,
) -> Result<(), AppError> {
    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));

    // Build a fresh registry snapshot for this run. Failures here become a
    // PdfError so the caller's error reporter handles them uniformly. Held
    // mutably so that `pick_remote_template` can install new entries directly
    // into this same snapshot (no rescan needed).
    let mut registry = TemplateRegistry::scan_local().map_err(AppError::from)?;

    // Resolve the configured default to a `Template`. If it's not installed,
    // fall back to the first available template; if none are installed, the
    // configured name is preserved as a synthetic placeholder so the flow can
    // still drive the user to "Browse remote templates…".
    let default_template = resolve_default_template(&registry, &validated.template)
        .ok_or_else(|| {
            PdfError::TemplateNotFound {
                name: validated.template.clone(),
                available: registry.names(),
            }
        })?;

    loop {
        let now = time::OffsetDateTime::now_utc();
        let period = invoice::period::collect_invoice_period(
            prompter,
            u32::from(now.month() as u8),
            now.year() as u32,
        )?;

        let line_items = invoice::line_item::collect_all_line_items(
            prompter,
            &validated.presets,
            validated.defaults.currency,
            config_path,
        )?;

        let summary = invoice::summary::build_summary(period, line_items, &validated.defaults)?;

        let formatted = invoice::display::format_summary(&summary);
        prompter.message(&formatted);

        // Show current template and offer to change.
        let desc = default_template
            .description
            .as_deref()
            .map(|d| format!(" ({d})"))
            .unwrap_or_default();
        prompter.message(&format!("Template: {}{}", default_template.name, desc));

        let template = if prompter.confirm("Change template?", false)? {
            // First show local templates + "Browse remote…" option.
            let local_choice = pick_local_template(prompter, &registry, &default_template)?;
            match local_choice {
                Some(t) => t,
                None => {
                    // User picked "Browse remote…". On install, use the new
                    // template; on cancel/error, fall back to the default.
                    pick_remote_template(prompter, &mut registry)?
                        .unwrap_or_else(|| default_template.clone())
                }
            }
        } else {
            default_template.clone()
        };

        if prompter.confirm("Generate PDF?", true)? {
            let pdf_bytes = pdf::generate_pdf(
                &summary,
                validated,
                recipient,
                config_dir,
                &template,
                validated.locale,
            )?;
            let output_path =
                super::common::pdf_output_path(&validated.sender.name, &summary.period, output_dir);

            if output_path.exists()
                && !prompter.confirm("File already exists. Overwrite?", false)?
            {
                prompter.message("PDF generation aborted.");
                continue;
            }

            std::fs::write(&output_path, &pdf_bytes).map_err(pdf::PdfError::Write)?;
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
    use crate::config::validator::{ValidatedBranding, ValidatedPaymentMethod};
    use crate::domain::NonEmpty;
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};

    fn make_validated_config() -> ValidatedConfig {
        let recipient = ValidatedRecipient::from_validated_parts(
            crate::domain::RecipientKey::try_new("acme").unwrap(),
            "Acme Corp".into(),
            vec!["123 Test St".into()],
            None,
            None,
        );
        ValidatedConfig::from_validated_parts(
            Sender {
                name: "Test User".into(),
                address: vec!["456 Dev Ave".into()],
                email: "test@example.com".into(),
            },
            NonEmpty::try_from_vec(vec![recipient]).unwrap(),
            0,
            NonEmpty::try_from_vec(vec![ValidatedPaymentMethod::from_validated_parts(
                crate::domain::PaymentMethodKey::try_new("sepa").unwrap(),
                Some("SEPA".into()),
                crate::domain::Iban::try_new("DE89370400440532013000").unwrap(),
                "TESTBIC".into(),
            )])
            .unwrap(),
            NonEmpty::try_from_vec(vec![Preset {
                key: crate::domain::PresetKey::try_new("dev").unwrap(),
                description: "Development".into(),
                default_rate: 800.0,
                currency: None,
                tax_rate: None,
            }])
            .unwrap(),
            Defaults::default(),
            ValidatedBranding::default(),
            "leda".into(),
            crate::locale::Locale::EnUs,
        )
    }

    #[test]
    fn test_resolve_default_template_picks_configured_when_present() {
        // Arrange — registry contains both templates; configured slug picks one.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("leda.typ"), b"// leda").unwrap();
        std::fs::write(dir.path().join("metis.typ"), b"// metis").unwrap();
        let registry = TemplateRegistry::scan_local_in(dir.path(), None).unwrap();

        // Act
        let resolved = resolve_default_template(&registry, "metis").unwrap();

        // Assert
        assert_eq!(resolved.name, "metis");
    }

    #[test]
    fn test_resolve_default_template_falls_back_to_first_when_missing() {
        // Arrange — configured slug is not installed; first available wins.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("amalthea.typ"), b"// a").unwrap();
        std::fs::write(dir.path().join("thebe.typ"), b"// t").unwrap();
        let registry = TemplateRegistry::scan_local_in(dir.path(), None).unwrap();

        // Act
        let resolved = resolve_default_template(&registry, "callisto").unwrap();

        // Assert — registry sorts alphabetically, so amalthea is first.
        assert_eq!(resolved.name, "amalthea");
    }

    #[test]
    fn test_resolve_default_template_returns_none_when_empty() {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        let registry = TemplateRegistry::scan_local_in(dir.path(), None).unwrap();

        // Act
        let resolved = resolve_default_template(&registry, "anything");

        // Assert
        assert!(resolved.is_none());
    }

    /// Compile-time guard: confirm `make_validated_config` and the imports
    /// stay consumed even if all flow-level integration tests are removed in
    /// future refactors.
    #[test]
    fn test_make_validated_config_smoke() {
        let cfg = make_validated_config();
        let _: &str = &cfg.template;
        let _ = MockPrompter::new(vec![MockResponse::Confirm(true)]);
    }
}
