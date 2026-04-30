//! Resolution of the config file path.
//!
//! Resolution order:
//! 1. `--config <path>` CLI flag
//! 2. `INVOICE_GENERATOR_CONFIG` env var (empty string treated as unset)
//! 3. `etcetera::choose_app_strategy(...).config_dir().join("config.yaml")`
//!
//! The parent directory is created on demand by [`ensure_parent_dir`], which is
//! kept separate from [`resolve_config_path`] so resolution remains a pure
//! function (no filesystem side effects, no global state).
//!
//! Env access is abstracted behind the [`EnvReader`] trait so tests can run in
//! parallel without touching `std::env::set_var`.

use std::path::{Path, PathBuf};

use etcetera::{AppStrategy, AppStrategyArgs, choose_app_strategy};

use crate::config::loader::CONFIG_FILENAME;
use crate::error::AppError;

/// Environment variable consulted when no `--config` flag is supplied.
const ENV_VAR: &str = "INVOICE_GENERATOR_CONFIG";

/// Abstraction over environment variable access for testability.
pub trait EnvReader {
    /// Look up `key`. Implementations should treat empty values as "unset".
    fn get(&self, key: &str) -> Option<String>;
}

/// Real environment reader that delegates to [`std::env::var`].
///
/// An empty string is treated as unset so that
/// `INVOICE_GENERATOR_CONFIG= invoice-generator …` falls through to the default
/// path rather than trying to use `""` as a path.
pub struct RealEnv;

impl EnvReader for RealEnv {
    fn get(&self, key: &str) -> Option<String> {
        match std::env::var(key) {
            Ok(s) if !s.is_empty() => Some(s),
            _ => None,
        }
    }
}

/// Resolve the config file path.
///
/// This is a pure function: no filesystem access, no global state.
/// Use [`ensure_parent_dir`] separately to create the directory if needed.
pub fn resolve_config_path(
    flag: Option<&Path>,
    env: &dyn EnvReader,
) -> Result<PathBuf, AppError> {
    if let Some(p) = flag {
        return Ok(p.to_path_buf());
    }
    if let Some(p) = env.get(ENV_VAR) {
        return Ok(PathBuf::from(p));
    }
    default_config_path()
}

fn default_config_path() -> Result<PathBuf, AppError> {
    let strategy = choose_app_strategy(AppStrategyArgs {
        top_level_domain: String::new(),
        author: String::new(),
        app_name: "invoice-generator".into(),
    })
    .map_err(|e| AppError::ConfigPath(format!("strategy: {e}")))?;
    Ok(strategy.config_dir().join(CONFIG_FILENAME))
}

/// Ensure the parent directory of `path` exists, creating it if necessary.
///
/// Returns an error if `path` has no parent (e.g. a bare `config.yaml` with no
/// directory component) or if directory creation fails.
pub fn ensure_parent_dir(path: &Path) -> Result<(), AppError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| {
            AppError::ConfigPath(format!(
                "config path {} has no parent directory",
                path.display()
            ))
        })?;
    std::fs::create_dir_all(parent)
        .map_err(|e| AppError::ConfigPath(format!("create {}: {e}", parent.display())))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vec-backed [`EnvReader`] for parallel-safe tests — no `std::env` access.
    struct MockEnv {
        entries: Vec<(String, String)>,
    }

    impl MockEnv {
        fn empty() -> Self {
            Self { entries: Vec::new() }
        }

        fn with(key: &str, value: &str) -> Self {
            Self {
                entries: vec![(key.to_string(), value.to_string())],
            }
        }
    }

    impl EnvReader for MockEnv {
        fn get(&self, key: &str) -> Option<String> {
            self.entries
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
                .filter(|v| !v.is_empty())
        }
    }

    #[test]
    fn test_resolve_config_path_returns_flag_override_when_provided() {
        // Arrange
        let flag = PathBuf::from("/tmp/custom.yaml");
        let env = MockEnv::with("INVOICE_GENERATOR_CONFIG", "/should/be/ignored.yaml");

        // Act
        let result = resolve_config_path(Some(&flag), &env).unwrap();

        // Assert
        assert_eq!(result, PathBuf::from("/tmp/custom.yaml"));
    }

    #[test]
    fn test_resolve_config_path_returns_env_var_when_no_flag() {
        // Arrange
        let env = MockEnv::with("INVOICE_GENERATOR_CONFIG", "/etc/invoice/config.yaml");

        // Act
        let result = resolve_config_path(None, &env).unwrap();

        // Assert
        assert_eq!(result, PathBuf::from("/etc/invoice/config.yaml"));
    }

    #[test]
    fn test_resolve_config_path_falls_through_to_xdg_default_when_unset() {
        // Arrange
        let env = MockEnv::empty();

        // Act
        let result = resolve_config_path(None, &env).unwrap();

        // Assert — default path should end with `invoice-generator/config.yaml`
        let s = result.to_string_lossy();
        assert!(
            s.ends_with("invoice-generator/config.yaml"),
            "Expected default path to end with 'invoice-generator/config.yaml', got: {s}"
        );
    }

    #[test]
    fn test_resolve_config_path_treats_empty_env_var_as_unset() {
        // Arrange — INVOICE_GENERATOR_CONFIG="" must not be treated as a real path
        let env = MockEnv::with("INVOICE_GENERATOR_CONFIG", "");

        // Act
        let result = resolve_config_path(None, &env).unwrap();

        // Assert — should fall through to XDG default, not yield an empty PathBuf
        let s = result.to_string_lossy();
        assert!(
            s.ends_with("invoice-generator/config.yaml"),
            "Empty env var should fall through to default, got: {s}"
        );
    }

    #[test]
    fn test_resolve_config_path_env_var_relative_path_kept_as_is() {
        // Arrange — relative paths from the env var are not canonicalized
        let env = MockEnv::with("INVOICE_GENERATOR_CONFIG", "foo.yaml");

        // Act
        let result = resolve_config_path(None, &env).unwrap();

        // Assert
        assert_eq!(result, PathBuf::from("foo.yaml"));
    }

    #[test]
    fn test_ensure_parent_dir_creates_missing_directory() {
        // Arrange
        let dir = tempfile::TempDir::new().unwrap();
        let nested = dir.path().join("a").join("b").join("config.yaml");
        assert!(!nested.parent().unwrap().exists());

        // Act
        ensure_parent_dir(&nested).unwrap();

        // Assert
        assert!(nested.parent().unwrap().exists());
    }

    #[test]
    fn test_ensure_parent_dir_succeeds_when_directory_exists() {
        // Arrange — calling twice in a row should be a no-op the second time
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.yaml");

        // Act
        ensure_parent_dir(&path).unwrap();
        let result = ensure_parent_dir(&path);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_ensure_parent_dir_returns_error_when_path_has_no_parent() {
        // Arrange — bare filename has no directory component
        let path = PathBuf::from("config.yaml");

        // Act
        let result = ensure_parent_dir(&path);

        // Assert
        assert!(matches!(result, Err(AppError::ConfigPath(_))));
    }

    #[test]
    #[cfg(unix)]
    fn test_ensure_parent_dir_returns_error_when_create_fails() {
        // Arrange — make a readonly parent so create_dir_all fails inside it
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::TempDir::new().unwrap();
        let readonly = dir.path().join("readonly");
        std::fs::create_dir(&readonly).unwrap();
        std::fs::set_permissions(&readonly, std::fs::Permissions::from_mode(0o000)).unwrap();
        let target = readonly.join("nested").join("config.yaml");

        // Act
        let result = ensure_parent_dir(&target);

        // Assert
        assert!(matches!(result, Err(AppError::ConfigPath(_))));

        // Restore permissions so TempDir cleanup works.
        std::fs::set_permissions(&readonly, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}
