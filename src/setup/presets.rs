use std::path::Path;

use crate::config::types::{Config, Preset};
use crate::config::writer::save_config;
use crate::error::AppError;
use super::prompter::Prompter;

/// Collect invoice presets interactively and persist them to disk.
pub fn collect_presets(
    prompter: &dyn Prompter,
    config: &mut Config,
    config_path: &Path,
) -> Result<(), AppError> {
    prompter.message("\n--- Presets ---\n");

    let mut presets = Vec::new();
    let mut count = 1;

    loop {
        prompter.message(&format!("Preset #{count}:"));

        let key = prompter.required_text("Short key (e.g. 'dev'):")?;
        let description = prompter.required_text("Description:")?;
        let default_rate = prompter.positive_f64("Default daily rate:")?;

        presets.push(Preset {
            key,
            description,
            default_rate,
            currency: None,
            tax_rate: None,
        });

        if !prompter.confirm("Add another preset?", false)? {
            break;
        }

        count += 1;
    }

    config.presets = Some(presets);
    save_config(config_path, config)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::loader::load_config;
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};
    use crate::setup::test_helpers::*;

    #[test]
    fn test_collect_presets_single_preset_decline_more() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("dev".into()),
            MockResponse::Text("Development Services".into()),
            MockResponse::F64(100.0),
            MockResponse::Confirm(false),
        ]);

        // Act
        collect_presets(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let presets = config.presets.as_ref().unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].key, "dev");
        assert_eq!(presets[0].description, "Development Services");
        assert_eq!(presets[0].default_rate, 100.0);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_presets_two_presets_via_add_another() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("dev".into()),
            MockResponse::Text("Development".into()),
            MockResponse::F64(100.0),
            MockResponse::Confirm(true),
            MockResponse::Text("design".into()),
            MockResponse::Text("Design Work".into()),
            MockResponse::F64(80.0),
            MockResponse::Confirm(false),
        ]);

        // Act
        collect_presets(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let presets = config.presets.unwrap();
        assert_eq!(presets.len(), 2);
        assert_eq!(presets[0].key, "dev");
        assert_eq!(presets[1].key, "design");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_presets_three_via_loop() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("a".into()),
            MockResponse::Text("A".into()),
            MockResponse::F64(10.0),
            MockResponse::Confirm(true),
            MockResponse::Text("b".into()),
            MockResponse::Text("B".into()),
            MockResponse::F64(20.0),
            MockResponse::Confirm(true),
            MockResponse::Text("c".into()),
            MockResponse::Text("C".into()),
            MockResponse::F64(30.0),
            MockResponse::Confirm(false),
        ]);

        // Act
        collect_presets(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let presets = config.presets.unwrap();
        assert_eq!(presets.len(), 3);
        assert_eq!(presets[2].key, "c");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_presets_decimal_rate() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("qa".into()),
            MockResponse::Text("QA".into()),
            MockResponse::F64(99.50),
            MockResponse::Confirm(false),
        ]);

        // Act
        collect_presets(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let presets = config.presets.unwrap();
        assert!((presets[0].default_rate - 99.50).abs() < f64::EPSILON);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_presets_persists_to_disk() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("dev".into()),
            MockResponse::Text("Dev".into()),
            MockResponse::F64(100.0),
            MockResponse::Confirm(false),
        ]);

        // Act
        collect_presets(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let presets = loaded.presets.unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].key, "dev");
        prompter.assert_exhausted();
    }
}
