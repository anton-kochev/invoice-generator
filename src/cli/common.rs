use std::path::{Path, PathBuf};

use crate::invoice::types::InvoicePeriod;

/// Build the standardized PDF output path: `Invoice_<Name>_<MonAbbrev><Year>.pdf`.
///
/// Spaces in `sender_name` are replaced with underscores.
pub fn pdf_output_path(sender_name: &str, period: &InvoicePeriod, dir: &Path) -> PathBuf {
    let name = sender_name.replace(' ', "_");
    let filename = format!(
        "Invoice_{}_{}{}.pdf",
        name,
        period.month_abbrev(),
        period.year()
    );
    dir.join(filename)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_pdf_output_path_replaces_spaces_in_name() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();

        // Act
        let path = pdf_output_path("Alice Smith", &period, Path::new("/tmp"));

        // Assert
        assert_eq!(path, PathBuf::from("/tmp/Invoice_Alice_Smith_Mar2026.pdf"));
    }

    #[test]
    fn test_pdf_output_path_no_spaces() {
        // Arrange
        let period = InvoicePeriod::new(12, 2025).unwrap();

        // Act
        let path = pdf_output_path("Acme", &period, Path::new("/out"));

        // Assert
        assert_eq!(path, PathBuf::from("/out/Invoice_Acme_Dec2025.pdf"));
    }
}
