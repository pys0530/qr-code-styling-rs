//! Main QRCodeStyling struct.

use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::config::{QRCodeStylingBuilder, QRCodeStylingOptions};
use crate::core::QRMatrix;
use crate::error::Result;
use crate::rendering::{PdfRenderer, RasterRenderer, SvgRenderer};
use crate::types::OutputFormat;

/// Main QR code styling struct.
///
/// This is the primary entry point for creating styled QR codes.
///
/// # Example
///
/// ```rust
/// use qr_code_styling::{QRCodeStyling, OutputFormat};
///
/// let qr = QRCodeStyling::builder()
///     .data("https://example.com")
///     .width(300)
///     .height(300)
///     .build()
///     .unwrap();
///
/// let svg = qr.render_svg().unwrap();
/// ```
pub struct QRCodeStyling {
    options: QRCodeStylingOptions,
    matrix: QRMatrix,
}

impl QRCodeStylingBuilder {
    /// Build the QRCodeStyling with the configured options.
    pub fn build(self) -> Result<QRCodeStyling> {
        let options = self.build_options()?;
        QRCodeStyling::new(options)
    }
}

impl QRCodeStyling {
    /// Create a new QRCodeStyling builder.
    pub fn builder() -> QRCodeStylingBuilder {
        QRCodeStylingBuilder::new()
    }

    /// Create a new QRCodeStyling with the given options.
    pub fn new(options: QRCodeStylingOptions) -> Result<Self> {
        let matrix = QRMatrix::new(&options.data, &options.qr_options)?;

        Ok(Self { options, matrix })
    }

    /// Update the data and regenerate the QR code.
    pub fn update(&mut self, data: &str) -> Result<&mut Self> {
        self.options.data = data.to_string();
        self.matrix = QRMatrix::new(&self.options.data, &self.options.qr_options)?;
        Ok(self)
    }

    /// Render the QR code as an SVG string.
    pub fn render_svg(&self) -> Result<String> {
        let renderer = SvgRenderer::new(&self.options);
        renderer.render(&self.matrix)
    }

    /// Render the QR code in the specified format.
    pub fn render(&self, format: OutputFormat) -> Result<Vec<u8>> {
        match format {
            OutputFormat::Svg => {
                let svg = self.render_svg()?;
                Ok(svg.into_bytes())
            }
            OutputFormat::Png | OutputFormat::Jpeg | OutputFormat::WebP => {
                let svg = self.render_svg()?;
                RasterRenderer::render(&svg, self.options.width, self.options.height, format)
            }
            OutputFormat::Pdf => {
                // Convert SVG directly to PDF (vector quality preserved)
                let svg = self.render_svg()?;
                PdfRenderer::render_from_svg(&svg, self.options.width, self.options.height)
            }
        }
    }

    /// Save the QR code to a file.
    pub fn save<P: AsRef<Path>>(&self, path: P, format: OutputFormat) -> Result<()> {
        let data = self.render(format)?;
        let mut file = File::create(path)?;
        file.write_all(&data)?;
        Ok(())
    }

    /// Get the QR code module count.
    pub fn module_count(&self) -> usize {
        self.matrix.module_count()
    }

    /// Get the current options.
    pub fn options(&self) -> &QRCodeStylingOptions {
        &self.options
    }

    /// Get mutable reference to options (requires regeneration after).
    pub fn options_mut(&mut self) -> &mut QRCodeStylingOptions {
        &mut self.options
    }

    /// Regenerate the QR matrix (call after modifying options).
    pub fn regenerate(&mut self) -> Result<()> {
        self.matrix = QRMatrix::new(&self.options.data, &self.options.qr_options)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DotType;
    use crate::config::DotsOptions;

    #[test]
    fn test_basic_creation() {
        let qr = QRCodeStyling::builder()
            .data("https://example.com")
            .build()
            .unwrap();

        assert!(qr.module_count() >= 21);
    }

    #[test]
    fn test_render_svg() {
        let qr = QRCodeStyling::builder()
            .data("Test")
            .width(200)
            .height(200)
            .build()
            .unwrap();

        let svg = qr.render_svg().unwrap();
        assert!(svg.contains("<?xml"));
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_update() {
        let mut qr = QRCodeStyling::builder()
            .data("First")
            .build()
            .unwrap();

        let count1 = qr.module_count();

        qr.update("This is a much longer string that should result in a larger QR code")
            .unwrap();

        let count2 = qr.module_count();

        // Longer data should result in larger QR code
        assert!(count2 >= count1);
    }

    #[test]
    fn test_with_dot_options() {
        let qr = QRCodeStyling::builder()
            .data("Test")
            .dots_options(DotsOptions::new(DotType::Dots))
            .build()
            .unwrap();

        let svg = qr.render_svg().unwrap();
        assert!(svg.contains("circle"));
    }

    #[test]
    fn test_render_png() {
        let qr = QRCodeStyling::builder()
            .data("Test")
            .width(100)
            .height(100)
            .build()
            .unwrap();

        let png = qr.render(OutputFormat::Png).unwrap();
        // PNG magic bytes
        assert_eq!(&png[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }
}
