//! Remote template fetching.
//!
//! Built-in templates ship inside the binary; everything else lives on GitHub
//! at `templates/<name>.typ` and is downloaded on demand. Network failures
//! surface as [`PdfError::Remote`] with a human-readable message.

use std::time::Duration;

use ureq::{Agent, AgentBuilder};

use super::error::PdfError;
use super::manifest::Manifest;

/// URL of the canonical manifest in the upstream repository.
pub const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/anton-kochev/invoice-generator/main/templates/manifest.json";

/// Hard upper bound on each HTTP call so an unreachable network can't freeze
/// the interactive prompt.
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

/// Build the raw URL for an upstream template `.typ` file.
pub fn template_url(name: &str) -> String {
    format!(
        "https://raw.githubusercontent.com/anton-kochev/invoice-generator/main/templates/{name}.typ"
    )
}

/// Build a ureq [`Agent`] with a 10-second global timeout. Constructed per
/// call — agents are cheap to build and not worth caching for the handful of
/// HTTP requests this CLI makes.
fn agent() -> Agent {
    AgentBuilder::new().timeout(HTTP_TIMEOUT).build()
}

/// Fetch the upstream manifest over HTTPS and parse it as JSON.
pub fn fetch_manifest() -> Result<Manifest, PdfError> {
    let body = agent()
        .get(MANIFEST_URL)
        .call()
        .map_err(|e| PdfError::Remote(friendly_manifest_error(&e, MANIFEST_URL)))?
        .into_string()
        .map_err(|e| PdfError::Remote(format!("could not read response body: {e}")))?;
    serde_json::from_str(&body).map_err(|e| {
        PdfError::Remote(format!("parse manifest from {MANIFEST_URL}: {e}"))
    })
}

/// Fetch a single template's `.typ` source from the upstream repository.
pub fn fetch_template(name: &str) -> Result<String, PdfError> {
    let url = template_url(name);
    agent()
        .get(&url)
        .call()
        .map_err(|e| PdfError::Remote(friendly_template_error(&e, name)))?
        .into_string()
        .map_err(|e| PdfError::Remote(format!("could not read response body: {e}")))
}

/// Format a manifest-fetch failure as user-friendly text.
fn friendly_manifest_error(err: &ureq::Error, url: &str) -> String {
    match err {
        ureq::Error::Status(404, _) => {
            format!("Manifest not found at {url}; the remote may have moved.")
        }
        ureq::Error::Status(code, _) => format!("Remote returned HTTP {code}."),
        ureq::Error::Transport(t) if is_network_unreachable(t) => {
            "Network unreachable. Using cached templates if available.".to_string()
        }
        ureq::Error::Transport(_) => err.to_string(),
    }
}

/// Format a template-fetch failure as user-friendly text. The 404 message
/// names the missing template instead of the URL.
fn friendly_template_error(err: &ureq::Error, name: &str) -> String {
    match err {
        ureq::Error::Status(404, _) => {
            format!("Template '{name}' not found in remote repo.")
        }
        ureq::Error::Status(code, _) => format!("Remote returned HTTP {code}."),
        ureq::Error::Transport(t) if is_network_unreachable(t) => {
            "Network unreachable. Using cached templates if available.".to_string()
        }
        ureq::Error::Transport(_) => err.to_string(),
    }
}

/// Best-effort classifier: do the transport details point at a connect/DNS/IO
/// failure that we want to surface as "network unreachable"?
fn is_network_unreachable(t: &ureq::Transport) -> bool {
    use ureq::ErrorKind;
    matches!(
        t.kind(),
        ErrorKind::ConnectionFailed | ErrorKind::Dns | ErrorKind::Io
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_url_includes_name() {
        // Arrange & Act
        let url = template_url("callisto");

        // Assert
        assert!(url.ends_with("/callisto.typ"), "got: {url}");
        assert!(url.starts_with("https://"), "got: {url}");
    }

    #[test]
    fn test_manifest_url_targets_main_branch() {
        // Arrange & Act & Assert
        assert!(MANIFEST_URL.contains("/main/templates/manifest.json"));
    }
}
