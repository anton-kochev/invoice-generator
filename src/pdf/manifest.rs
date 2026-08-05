//! Bundled + on-disk template manifest.
//!
//! The bundled `templates/manifest.json` ships with the binary and lists all
//! templates the maintainers know about. On first run, that bundled JSON is
//! seeded into the user's cache directory; thereafter the cache is the source
//! of truth (refreshed by `invoice-generator template refresh`).

use std::path::PathBuf;

use etcetera::{AppStrategy, AppStrategyArgs, choose_app_strategy};
use serde::{Deserialize, Serialize};

use super::error::PdfError;

/// JSON file name used inside the cache directory.
const CACHE_FILENAME: &str = "manifest.json";

/// Bundled manifest baked into the binary at compile time. Used as the seed
/// when the cache file does not yet exist.
pub const BUNDLED_MANIFEST: &str = include_str!("../../templates/manifest.json");

/// Top-level manifest JSON document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub templates: Vec<ManifestEntry>,
}

/// A single template entry in [`Manifest`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestEntry {
    pub name: String,
    pub file: String,
    pub description: String,
}

/// Path to the on-disk manifest cache: `<XDG_CACHE_HOME>/invoice-generator/manifest.json`.
pub fn cache_path() -> Result<PathBuf, PdfError> {
    let strategy = choose_app_strategy(AppStrategyArgs {
        top_level_domain: String::new(),
        author: String::new(),
        app_name: "invoice-generator".into(),
    })
    .map_err(|e| PdfError::Manifest(format!("could not resolve cache dir: {e}")))?;
    Ok(strategy.cache_dir().join(CACHE_FILENAME))
}

/// Load the cached manifest. If no cache file exists, parse [`BUNDLED_MANIFEST`]
/// and write it to the cache path before returning.
pub fn load_cache_or_seed() -> Result<Manifest, PdfError> {
    let path = cache_path()?;
    if path.exists() {
        let bytes = std::fs::read(&path)
            .map_err(|e| PdfError::Manifest(format!("read {}: {e}", path.display())))?;
        return serde_json::from_slice(&bytes)
            .map_err(|e| PdfError::Manifest(format!("parse cache {}: {e}", path.display())));
    }
    let manifest: Manifest = serde_json::from_str(BUNDLED_MANIFEST)
        .map_err(|e| PdfError::Manifest(format!("parse bundled manifest: {e}")))?;
    write_cache(&manifest)?;
    Ok(manifest)
}

/// Atomically write `manifest` as JSON to the cache path. Writes to a `.tmp`
/// sibling file and renames into place to avoid partial writes if the process
/// is interrupted mid-write.
pub fn write_cache(manifest: &Manifest) -> Result<(), PdfError> {
    let path = cache_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            PdfError::Manifest(format!("create cache dir {}: {e}", parent.display()))
        })?;
    }
    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|e| PdfError::Manifest(format!("serialize manifest: {e}")))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &bytes)
        .map_err(|e| PdfError::Manifest(format!("write {}: {e}", tmp.display())))?;
    std::fs::rename(&tmp, &path)
        .map_err(|e| PdfError::Manifest(format!("rename {}: {e}", path.display())))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundled_manifest_parses_cleanly() {
        // Arrange & Act
        let parsed: Manifest = serde_json::from_str(BUNDLED_MANIFEST).unwrap();

        // Assert
        assert!(
            !parsed.templates.is_empty(),
            "Bundled manifest must list at least one template"
        );
        for entry in &parsed.templates {
            assert!(!entry.name.is_empty(), "Template name must not be empty");
            assert!(
                entry.file.ends_with(".typ"),
                "Template file '{}' must end with .typ",
                entry.file
            );
            assert!(
                !entry.description.is_empty(),
                "Template '{}' has empty description",
                entry.name
            );
        }
    }

    #[test]
    fn test_bundled_manifest_contains_seven_templates() {
        // Arrange & Act
        let parsed: Manifest = serde_json::from_str(BUNDLED_MANIFEST).unwrap();

        // Assert — seven templates: callisto, thebe, amalthea, metis, io,
        // europa, adrastea.
        assert_eq!(parsed.templates.len(), 7);
        let names: Vec<&str> = parsed.templates.iter().map(|e| e.name.as_str()).collect();
        for expected in [
            "callisto", "thebe", "amalthea", "metis", "io", "europa", "adrastea",
        ] {
            assert!(
                names.contains(&expected),
                "Expected '{expected}' in bundled manifest, got: {names:?}"
            );
        }
    }
}
