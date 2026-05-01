use std::path::Path;

use super::prompter::Prompter;
use crate::config::types::Config;
use crate::config::writer::save_config;
use crate::error::AppError;

/// Prompt for a custom footer string and persist it to disk.
///
/// Empty / whitespace-only input leaves [`Branding::footer_text`] absent so the
/// template default applies. Any other branding fields already present
/// (logo, accent_color, font) are preserved.
///
/// [`Branding::footer_text`]: crate::config::types::Branding::footer_text
pub fn collect_branding(
    prompter: &dyn Prompter,
    config: &mut Config,
    config_path: &Path,
) -> Result<(), AppError> {
    prompter.message("\n--- Footer ---\n");
    let raw = prompter.optional_text(
        "Custom footer text (shown at bottom of invoice — leave empty for template default):",
    )?;

    if let Some(text) = raw.and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }) {
        let mut branding = config.branding.take().unwrap_or_default();
        branding.footer_text = Some(text);
        config.branding = Some(branding);
        save_config(config_path, config)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::loader::load_config;
    use crate::config::types::Branding;
    use crate::domain::HexColor;
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};
    use crate::setup::test_helpers::*;

    #[test]
    fn test_collect_branding_with_footer_records_some() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![MockResponse::OptionalText(Some("Thanks!".into()))]);

        // Act
        collect_branding(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let branding = config.branding.as_ref().unwrap();
        assert_eq!(branding.footer_text.as_deref(), Some("Thanks!"));
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_branding_blank_footer_leaves_unchanged() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![MockResponse::OptionalText(None)]);

        // Act
        collect_branding(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert — no branding mutation when input is blank.
        assert!(config.branding.is_none());
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_branding_whitespace_only_treated_as_empty() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![MockResponse::OptionalText(Some("   ".into()))]);

        // Act
        collect_branding(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert — whitespace-only input is filtered out the same as blank.
        assert!(config.branding.is_none());
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_branding_trims_surrounding_whitespace() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter =
            MockPrompter::new(vec![MockResponse::OptionalText(Some("  Thanks!  ".into()))]);

        // Act
        collect_branding(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let branding = config.branding.as_ref().unwrap();
        assert_eq!(branding.footer_text.as_deref(), Some("Thanks!"));
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_branding_persists_to_disk() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![MockResponse::OptionalText(Some(
            "Persisted footer".into(),
        ))]);

        // Act
        collect_branding(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let branding = loaded.branding.unwrap();
        assert_eq!(branding.footer_text.as_deref(), Some("Persisted footer"));
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_branding_preserves_existing_fields() {
        // Arrange — pre-seed branding with an accent color; new footer must not wipe it.
        let dir = setup_dir(None);
        let accent = HexColor::try_new("#3aa9ff").unwrap();
        let mut config = Config {
            branding: Some(Branding {
                accent_color: Some(accent.clone()),
                ..Branding::default()
            }),
            ..Config::default()
        };
        let prompter = MockPrompter::new(vec![MockResponse::OptionalText(Some(
            "Footer with accent".into(),
        ))]);

        // Act
        collect_branding(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let branding = config.branding.as_ref().unwrap();
        assert_eq!(branding.accent_color.as_ref(), Some(&accent));
        assert_eq!(branding.footer_text.as_deref(), Some("Footer with accent"));
        prompter.assert_exhausted();
    }
}
