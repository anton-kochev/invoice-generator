//! `invoice-generator template …` subcommand handlers.
//!
//! For v1 the only action is `refresh`: pull the latest manifest from upstream
//! and atomically replace the cached copy. Listing/installing templates is
//! handled by the interactive flow's "Browse remote…" prompt.

use std::io::Write;

use clap::Subcommand;

use crate::error::AppError;
use crate::pdf::manifest;
use crate::pdf::remote;

/// `template …` subcommand selector.
#[derive(Debug, Subcommand)]
pub enum TemplateAction {
    /// Refresh the cached template manifest from upstream.
    Refresh,
}

/// Dispatch a `template …` invocation.
pub fn handle_template(action: TemplateAction, writer: &mut dyn Write) -> Result<(), AppError> {
    match action {
        TemplateAction::Refresh => handle_refresh(writer),
    }
}

fn handle_refresh(writer: &mut dyn Write) -> Result<(), AppError> {
    let manifest = remote::fetch_manifest()?;
    manifest::write_cache(&manifest)?;
    writeln!(
        writer,
        "Refreshed template manifest. {} template{} available remotely.",
        manifest.templates.len(),
        if manifest.templates.len() == 1 { "" } else { "s" }
    )
    .map_err(crate::cli::CliError::OutputWrite)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_action_refresh_variant_exists() {
        // Arrange & Act
        let action = TemplateAction::Refresh;

        // Assert — match exhaustiveness ensures the variant is present.
        match action {
            TemplateAction::Refresh => {}
        }
    }
}
