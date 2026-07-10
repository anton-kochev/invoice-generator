//! On-disk template registry.
//!
//! Templates live in `<XDG_CONFIG>/invoice-generator/templates/` as plain
//! `.typ` files. A small set is bundled into the binary and seeded into that
//! directory on first run; the rest are fetched on demand from the upstream
//! GitHub repository (see [`super::remote`]).

use std::path::{Path, PathBuf};

use etcetera::{AppStrategy, AppStrategyArgs, choose_app_strategy};

use super::error::PdfError;
use super::manifest::{self, Manifest};
use super::remote;

/// Bundled built-in templates: name + `.typ` source baked into the binary at
/// compile time. Only these three are shipped in-binary; the rest are
/// fetched on demand.
const BUILTIN_TEMPLATES: &[(&str, &str)] = &[
    ("amalthea", include_str!("../../templates/amalthea.typ")),
    ("metis", include_str!("../../templates/metis.typ")),
    ("thebe", include_str!("../../templates/thebe.typ")),
];

/// A locally-installed template, ready to render.
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    /// Slug used in the config file and on the CLI (e.g. `"amalthea"`).
    pub name: String,
    /// Human-readable one-liner from the manifest, when known.
    pub description: Option<String>,
    /// Absolute path to the on-disk `.typ` source file.
    pub source: PathBuf,
}

/// In-memory snapshot of all templates currently installed in the local
/// templates directory.
#[derive(Debug, Clone)]
pub struct TemplateRegistry {
    templates: Vec<Template>,
}

impl TemplateRegistry {
    /// Path to the local templates directory:
    /// `<XDG_CONFIG_HOME>/invoice-generator/templates/`.
    pub fn local_dir() -> Result<PathBuf, PdfError> {
        let strategy = choose_app_strategy(AppStrategyArgs {
            top_level_domain: String::new(),
            author: String::new(),
            app_name: "invoice-generator".into(),
        })
        .map_err(|e| PdfError::Manifest(format!("could not resolve config dir: {e}")))?;
        Ok(strategy.config_dir().join("templates"))
    }

    /// Ensure the local templates directory exists, creating it if necessary.
    pub fn ensure_local_dir() -> Result<PathBuf, PdfError> {
        let dir = Self::local_dir()?;
        std::fs::create_dir_all(&dir).map_err(|e| {
            PdfError::Manifest(format!("create templates dir {}: {e}", dir.display()))
        })?;
        Ok(dir)
    }

    /// Write each bundled built-in template to disk if it is missing.
    ///
    /// **Never overwrites** an existing file — the on-disk copy is treated as
    /// authoritative so users can hand-edit a built-in template without losing
    /// their changes on the next run.
    pub fn write_builtins_if_missing() -> Result<(), PdfError> {
        Self::write_builtins_if_missing_into(&Self::ensure_local_dir()?)
    }

    /// Same as [`write_builtins_if_missing`], but writes into an explicit
    /// directory. Used by tests to avoid touching the user's real config.
    pub(crate) fn write_builtins_if_missing_into(dir: &Path) -> Result<(), PdfError> {
        std::fs::create_dir_all(dir).map_err(|e| {
            PdfError::Manifest(format!("create templates dir {}: {e}", dir.display()))
        })?;
        for (name, source) in BUILTIN_TEMPLATES {
            let path = dir.join(format!("{name}.typ"));
            if path.exists() {
                continue;
            }
            std::fs::write(&path, source.as_bytes()).map_err(|e| {
                PdfError::Manifest(format!("write builtin {}: {e}", path.display()))
            })?;
        }
        Ok(())
    }

    /// Scan the local templates directory and build a registry of every
    /// `.typ` file found there, looking up descriptions in the cached
    /// manifest when possible.
    pub fn scan_local() -> Result<Self, PdfError> {
        let dir = Self::ensure_local_dir()?;
        let manifest = manifest::load_cache_or_seed().ok();
        Self::scan_local_in(&dir, manifest.as_ref())
    }

    /// Same as [`scan_local`], but operates on an explicit directory and an
    /// explicit manifest. Used by tests.
    pub(crate) fn scan_local_in(dir: &Path, manifest: Option<&Manifest>) -> Result<Self, PdfError> {
        let mut templates = Vec::new();
        if !dir.exists() {
            return Ok(Self { templates });
        }
        let entries = std::fs::read_dir(dir)
            .map_err(|e| PdfError::Manifest(format!("read dir {}: {e}", dir.display())))?;
        for entry in entries {
            let entry = entry
                .map_err(|e| PdfError::Manifest(format!("read entry in {}: {e}", dir.display())))?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("typ") {
                continue;
            }
            let name = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let description = manifest.and_then(|m| {
                m.templates
                    .iter()
                    .find(|e| e.name == name)
                    .map(|e| e.description.clone())
            });
            templates.push(Template {
                name,
                description,
                source: path,
            });
        }
        templates.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Self { templates })
    }

    /// All templates in the registry, sorted by name.
    pub fn templates(&self) -> &[Template] {
        &self.templates
    }

    /// Look up a template by its slug.
    pub fn find_by_name(&self, name: &str) -> Option<&Template> {
        self.templates.iter().find(|t| t.name == name)
    }

    /// Names of all installed templates, sorted.
    pub fn names(&self) -> Vec<String> {
        self.templates.iter().map(|t| t.name.clone()).collect()
    }

    /// Fetch a remote template's source, write it to the local templates dir,
    /// add it to this registry, and return a reference to the new entry.
    ///
    /// If a template with the same name was already in `self.templates` (e.g.
    /// the user is reinstalling), the existing entry is replaced so callers
    /// observe the freshly-downloaded source.
    pub fn install_from_remote(&mut self, name: &str) -> Result<&Template, PdfError> {
        let dir = Self::ensure_local_dir()?;
        self.install_from_remote_into(&dir, name)
    }

    /// Same as [`install_from_remote`], but writes into an explicit directory.
    pub(crate) fn install_from_remote_into(
        &mut self,
        dir: &Path,
        name: &str,
    ) -> Result<&Template, PdfError> {
        let source = remote::fetch_template(name)?;
        let path = dir.join(format!("{name}.typ"));
        std::fs::write(&path, source.as_bytes())
            .map_err(|e| PdfError::Manifest(format!("write {}: {e}", path.display())))?;
        // Description lookup is best-effort — the manifest may be missing or
        // out-of-date when the remote install is triggered, so we don't fail
        // the install over it.
        let description = manifest::load_cache_or_seed()
            .ok()
            .and_then(|m| m.templates.iter().find(|e| e.name == name).cloned())
            .map(|e| e.description);
        let template = Template {
            name: name.to_string(),
            description,
            source: path,
        };
        // Replace any existing entry with the same name so the registry
        // reflects the just-downloaded source.
        if let Some(existing) = self.templates.iter_mut().find(|t| t.name == name) {
            *existing = template;
        } else {
            self.templates.push(template);
            self.templates.sort_by(|a, b| a.name.cmp(&b.name));
        }
        // Return a reference to the entry we just installed.
        Ok(self
            .templates
            .iter()
            .find(|t| t.name == name)
            .expect("just inserted"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_builtins_if_missing_creates_three_files() {
        // Arrange
        let dir = TempDir::new().unwrap();

        // Act
        TemplateRegistry::write_builtins_if_missing_into(dir.path()).unwrap();

        // Assert — exactly the three bundled templates land on disk.
        for name in ["amalthea", "metis", "thebe"] {
            let path = dir.path().join(format!("{name}.typ"));
            assert!(path.exists(), "{} should exist", path.display());
            let bytes = std::fs::read(&path).unwrap();
            assert!(!bytes.is_empty(), "{} should be non-empty", path.display());
        }
    }

    #[test]
    fn test_write_builtins_if_missing_does_not_overwrite_existing() {
        // Arrange — pre-populate amalthea with sentinel content; the next call
        // must leave it untouched.
        let dir = TempDir::new().unwrap();
        let amalthea = dir.path().join("amalthea.typ");
        std::fs::write(&amalthea, b"// user-edited content").unwrap();

        // Act
        TemplateRegistry::write_builtins_if_missing_into(dir.path()).unwrap();

        // Assert
        let bytes = std::fs::read(&amalthea).unwrap();
        assert_eq!(
            bytes, b"// user-edited content",
            "Existing user-edited template must not be overwritten"
        );
        // The other two were missing, so they should now exist.
        assert!(dir.path().join("metis.typ").exists());
        assert!(dir.path().join("thebe.typ").exists());
    }

    #[test]
    fn test_scan_local_in_returns_typ_files_only() {
        // Arrange
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("foo.typ"), b"#let x = 1").unwrap();
        std::fs::write(dir.path().join("bar.typ"), b"#let y = 2").unwrap();
        std::fs::write(dir.path().join("readme.txt"), b"ignore me").unwrap();

        // Act
        let registry = TemplateRegistry::scan_local_in(dir.path(), None).unwrap();

        // Assert
        let names = registry.names();
        assert_eq!(names, vec!["bar".to_string(), "foo".to_string()]);
    }

    #[test]
    fn test_scan_local_in_attaches_descriptions_from_manifest() {
        // Arrange
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test-tpl.typ"), b"// test-tpl").unwrap();
        let manifest = Manifest {
            templates: vec![manifest::ManifestEntry {
                name: "test-tpl".into(),
                file: "test-tpl.typ".into(),
                description: "Clean & minimal".into(),
            }],
        };

        // Act
        let registry = TemplateRegistry::scan_local_in(dir.path(), Some(&manifest)).unwrap();

        // Assert
        let entry = registry.find_by_name("test-tpl").unwrap();
        assert_eq!(entry.description.as_deref(), Some("Clean & minimal"));
    }

    #[test]
    fn test_scan_local_in_missing_dir_returns_empty() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let nonexistent = dir.path().join("does-not-exist");

        // Act
        let registry = TemplateRegistry::scan_local_in(&nonexistent, None).unwrap();

        // Assert
        assert!(registry.templates().is_empty());
    }
}
