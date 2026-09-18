//! SVG renderer for QR codes.

use std::f64::consts::PI;
use fixedbitset::FixedBitSet;

use crate::config::{Color, Gradient, QRCodeStylingOptions};
use crate::core::QRMatrix;
use crate::error::Result;
use crate::figures::{QRCornerDot, QRCornerSquare, QRDot};
use crate::types::{GradientType, ShapeType};

/// SVG renderer for QR codes.
pub struct SvgRenderer<'a> {
    options: &'a QRCodeStylingOptions,
    instance_id: u64,
}

/// Square mask for corner squares (7x7 pattern, flattened to 1D).
/// 1 = part of outer square border, 0 = not part of border
const SQUARE_MASK: [u8; 49] = [
    1, 1, 1, 1, 1, 1, 1,
    1, 0, 0, 0, 0, 0, 1,
    1, 0, 0, 0, 0, 0, 1,
    1, 0, 0, 0, 0, 0, 1,
    1, 0, 0, 0, 0, 0, 1,
    1, 0, 0, 0, 0, 0, 1,
    1, 1, 1, 1, 1, 1, 1,
];

/// Dot mask for corner dots (7x7 pattern, flattened to 1D).
/// 1 = part of inner 3x3 dot, 0 = not part of dot
const DOT_MASK: [u8; 49] = [
    0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0,
    0, 0, 1, 1, 1, 0, 0,
    0, 0, 1, 1, 1, 0, 0,
    0, 0, 1, 1, 1, 0, 0,
    0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0,
];

static INSTANCE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl<'a> SvgRenderer<'a> {
    /// Create a new SVG renderer.
    pub fn new(options: &'a QRCodeStylingOptions) -> Self {
        let instance_id = INSTANCE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self {
            options,
            instance_id,
        }
    }

    /// Render the QR code as SVG string.
    pub fn render(&self, matrix: &QRMatrix) -> Result<String> {
        let count = matrix.module_count();
        let min_size = self.options.width.min(self.options.height) - self.options.margin * 2;
        let real_qr_size = if self.options.shape == ShapeType::Circle {
            min_size as f64 / 2.0_f64.sqrt()
        } else {
            min_size as f64
        };
        let dot_size = self.round_size(real_qr_size / count as f64);

        // Calculate image hiding area if there's an image
        let (hide_x_dots, hide_y_dots) = if self.options.image.is_some() {
            self.calculate_image_hide_area(count, dot_size)
        } else {
            (0, 0)
        };

        let mut svg_content = String::with_capacity(10000);
        let mut defs_content = String::new();
        let mut elements_content = String::new();

        // Draw background
        let (bg_defs, bg_elements) = self.render_background();
        defs_content.push_str(&bg_defs);
        elements_content.push_str(&bg_elements);

        // Draw dots
        let (dots_defs, dots_elements) = self.render_dots(
            matrix,
            count,
            dot_size,
            hide_x_dots,
            hide_y_dots,
        );
        defs_content.push_str(&dots_defs);
        elements_content.push_str(&dots_elements);

        // Draw corners
        let (corners_defs, corners_elements) = self.render_corners(count, dot_size);
        defs_content.push_str(&corners_defs);
        elements_content.push_str(&corners_elements);

        // Draw image if present
        if let Some(ref image_data) = self.options.image {
            let image_svg = self.render_image(count, dot_size, hide_x_dots, hide_y_dots, image_data);
            elements_content.push_str(&image_svg);
        }

        // Build final SVG
        let shape_rendering = if self.options.dots_options.round_size {
            ""
        } else {
            r#" shape-rendering="crispEdges""#
        };

        svg_content.push_str(&format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{}" height="{}" viewBox="0 0 {} {}"{}>
<defs>
{}
</defs>
{}
</svg>"#,
            self.options.width,
            self.options.height,
            self.options.width,
            self.options.height,
            shape_rendering,
            defs_content,
            elements_content
        ));

        Ok(svg_content)
    }

    fn render_background(&self) -> (String, String) {
        let mut defs = String::new();
        let mut elements = String::new();

        let bg = &self.options.background_options;
        let name = format!("background-color-{}", self.instance_id);

        let (width, height) = if bg.round > 0.0 {
            let size = self.options.width.min(self.options.height);
            (size, size)
        } else {
            (self.options.width, self.options.height)
        };

        let x = self.round_size((self.options.width - width) as f64 / 2.0);
        let y = self.round_size((self.options.height - height) as f64 / 2.0);

        // Create clip path
        let rx = if bg.round > 0.0 {
            (height as f64 / 2.0) * bg.round
        } else {
            0.0
        };

        defs.push_str(&format!(
            r#"<clipPath id="clip-path-{}"><rect x="{}" y="{}" width="{}" height="{}"{}/></clipPath>
"#,
            name,
            x,
            y,
            width,
            height,
            if rx > 0.0 {
                format!(r#" rx="{}""#, rx)
            } else {
                String::new()
            }
        ));

        // Create color/gradient rect
        let (grad_defs, fill) = self.create_color(
            bg.gradient.as_ref(),
            &bg.color,
            0.0,
            0.0,
            0.0,
            self.options.height as f64,
            self.options.width as f64,
            &name,
        );
        defs.push_str(&grad_defs);

        elements.push_str(&format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" clip-path="url(#clip-path-{})"/>
"#,
            0, 0, self.options.width, self.options.height, fill, name
        ));

        (defs, elements)
    }

    fn render_dots(
        &self,
        matrix: &QRMatrix,
        count: usize,
        dot_size: f64,
        hide_x_dots: usize,
        hide_y_dots: usize,
    ) -> (String, String) {
        let mut defs = String::new();
        let mut clip_path_elements = String::new();

        let x_beginning = self.round_size((self.options.width as f64 - count as f64 * dot_size) / 2.0);
        let y_beginning = self.round_size((self.options.height as f64 - count as f64 * dot_size) / 2.0);

        let dot_drawer = QRDot::new(self.options.dots_options.dot_type);
        let name = format!("dot-color-{}", self.instance_id);

        let mut drawable = FixedBitSet::with_capacity(count * count);

        for row in 0..count {
            for col in 0..count {
                if self.should_draw_dot(row, col, count, hide_x_dots, hide_y_dots)
                    && matrix.is_dark(row, col)
                {
                    drawable.insert(row * count + col);
                }
            }
        }

        for row in 0..count {
            for col in 0..count {
                if !drawable.contains(row * count + col) {
                    continue;
                }

                let x = x_beginning + col as f64 * dot_size;
                let y = y_beginning + row as f64 * dot_size;

                let neighbor_fn = |x_offset: i32, y_offset: i32| -> bool {
                    let new_col = col as i32 + x_offset;
                    let new_row = row as i32 + y_offset;
                    if new_col < 0 || new_row < 0 || new_col >= count as i32 || new_row >= count as i32 {
                        return false;
                    }
                    drawable.contains(new_row as usize * count + new_col as usize)
                };

                let svg = dot_drawer.draw(x, y, dot_size, Some(&neighbor_fn));
                clip_path_elements.push_str(&svg);
                clip_path_elements.push('\n');
            }
        }

        // Handle circle shape with fake edge dots
        if self.options.shape == ShapeType::Circle {
            let circle_dots = self.render_circle_edge_dots(matrix, count, dot_size, x_beginning, y_beginning, &dot_drawer);
            clip_path_elements.push_str(&circle_dots);
        }

        defs.push_str(&format!(
            r#"<clipPath id="clip-path-{}">
{}
</clipPath>
"#,
            name, clip_path_elements
        ));

        // Create color rect
        let (grad_defs, fill) = self.create_color(
            self.options.dots_options.gradient.as_ref(),
            &self.options.dots_options.color,
            0.0,
            0.0,
            0.0,
            self.options.height as f64,
            self.options.width as f64,
            &name,
        );
        defs.push_str(&grad_defs);

        let elements = format!(
            r#"<rect x="0" y="0" width="{}" height="{}" fill="{}" clip-path="url(#clip-path-{})"/>
"#,
            self.options.width, self.options.height, fill, name
        );

        (defs, elements)
    }

    fn render_circle_edge_dots(
        &self,
        matrix: &QRMatrix,
        count: usize,
        dot_size: f64,
        x_beginning: f64,
        y_beginning: f64,
        dot_drawer: &QRDot,
    ) -> String {
        let mut result = String::new();
        let min_size = (self.options.width.min(self.options.height) - self.options.margin * 2) as f64;
        let additional_dots = self.round_size((min_size / dot_size - count as f64) / 2.0) as usize;
        let fake_count = count + additional_dots * 2;
        let x_fake_beginning = x_beginning - additional_dots as f64 * dot_size;
        let y_fake_beginning = y_beginning - additional_dots as f64 * dot_size;
        let center = fake_count as f64 / 2.0;

        let mut fake_matrix = vec![0u8; fake_count * fake_count];

        for row in 0..fake_count {
            for col in 0..fake_count {
                // Skip inner area
                if row >= additional_dots.saturating_sub(1)
                    && row <= fake_count - additional_dots
                    && col >= additional_dots.saturating_sub(1)
                    && col <= fake_count - additional_dots
                {
                    continue;
                }

                // Skip outside circle
                let dist = ((row as f64 - center).powi(2) + (col as f64 - center).powi(2)).sqrt();
                if dist > center {
                    continue;
                }

                // Get random dots from QR code
                let source_col = if col < 2 * additional_dots {
                    col
                } else if col >= count {
                    col.wrapping_sub(2 * additional_dots)
                } else {
                    col.wrapping_sub(additional_dots)
                };
                let source_row = if row < 2 * additional_dots {
                    row
                } else if row >= count {
                    row.wrapping_sub(2 * additional_dots)
                } else {
                    row.wrapping_sub(additional_dots)
                };

                if source_row < count && source_col < count && matrix.is_dark(source_row, source_col) {
                    fake_matrix[row * fake_count + col] = 1;
                }
            }
        }

        for row in 0..fake_count {
            for col in 0..fake_count {
                if fake_matrix[row * fake_count + col] == 0 {
                    continue;
                }

                let x = x_fake_beginning + col as f64 * dot_size;
                let y = y_fake_beginning + row as f64 * dot_size;

                let neighbor_fn = |x_offset: i32, y_offset: i32| -> bool {
                    let new_col = col as i32 + x_offset;
                    let new_row = row as i32 + y_offset;
                    if new_col < 0 || new_row < 0 || new_col >= fake_count as i32 || new_row >= fake_count as i32 {
                        return false;
                    }
                    fake_matrix[row * fake_count + col] == 1
                };

                let svg = dot_drawer.draw(x, y, dot_size, Some(&neighbor_fn));
                result.push_str(&svg);
                result.push('\n');
            }
        }

        result
    }

    fn render_corners(&self, count: usize, dot_size: f64) -> (String, String) {
        let mut defs = String::new();
        let mut elements = String::new();

        let x_beginning = self.round_size((self.options.width as f64 - count as f64 * dot_size) / 2.0);
        let y_beginning = self.round_size((self.options.height as f64 - count as f64 * dot_size) / 2.0);

        let corners_square_size = dot_size * 7.0;
        let corners_dot_size = dot_size * 3.0;

        // Three corners: top-left, top-right, bottom-left
        let corner_positions = [
            (0, 0, 0.0),
            (1, 0, PI / 2.0),
            (0, 1, -PI / 2.0),
        ];

        for (column, row, rotation) in corner_positions {
            let x = x_beginning + column as f64 * dot_size * (count - 7) as f64;
            let y = y_beginning + row as f64 * dot_size * (count - 7) as f64;

            // Render corner square
            let (sq_defs, sq_elements) = self.render_corner_square(
                x, y, corners_square_size, dot_size, rotation, column, row,
            );
            defs.push_str(&sq_defs);
            elements.push_str(&sq_elements);

            // Render corner dot
            let (dot_defs, dot_elements) = self.render_corner_dot(
                x + dot_size * 2.0,
                y + dot_size * 2.0,
                corners_dot_size,
                dot_size,
                rotation,
                column,
                row,
            );
            defs.push_str(&dot_defs);
            elements.push_str(&dot_elements);
        }

        (defs, elements)
    }

    fn render_corner_square(
        &self,
        x: f64,
        y: f64,
        size: f64,
        _dot_size: f64,
        rotation: f64,
        column: usize,
        row: usize,
    ) -> (String, String) {
        let mut defs = String::new();
        let mut clip_path_content = String::new();

        let name = format!("corners-square-color-{}-{}-{}", column, row, self.instance_id);

        let sq_options = &self.options.corners_square_options;

        // Use corner square drawer if specific type is set
        let drawer = QRCornerSquare::new(sq_options.square_type);
        let svg = drawer.draw(x, y, size, rotation);
        clip_path_content.push_str(&svg);

        defs.push_str(&format!(
            r#"<clipPath id="clip-path-{}">
{}
</clipPath>
"#,
            name, clip_path_content
        ));

        // Create color
        let (grad_defs, fill) = self.create_color(
            sq_options.gradient.as_ref(),
            &sq_options.color,
            rotation,
            x,
            y,
            size,
            size,
            &name,
        );
        defs.push_str(&grad_defs);

        let elements = format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" clip-path="url(#clip-path-{})"/>
"#,
            x, y, size, size, fill, name
        );

        (defs, elements)
    }

    fn render_corner_dot(
        &self,
        x: f64,
        y: f64,
        size: f64,
        _dot_size: f64,
        rotation: f64,
        column: usize,
        row: usize,
    ) -> (String, String) {
        let mut defs = String::new();
        let mut clip_path_content = String::new();

        let name = format!("corners-dot-color-{}-{}-{}", column, row, self.instance_id);

        let dot_options = &self.options.corners_dot_options;

        // Use corner dot drawer
        let drawer = QRCornerDot::new(dot_options.dot_type);
        let svg = drawer.draw(x, y, size, rotation);
        clip_path_content.push_str(&svg);

        defs.push_str(&format!(
            r#"<clipPath id="clip-path-{}">
{}
</clipPath>
"#,
            name, clip_path_content
        ));

        // Create color
        let (grad_defs, fill) = self.create_color(
            dot_options.gradient.as_ref(),
            &dot_options.color,
            rotation,
            x,
            y,
            size,
            size,
            &name,
        );
        defs.push_str(&grad_defs);

        let elements = format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" clip-path="url(#clip-path-{})"/>
"#,
            x, y, size, size, fill, name
        );

        (defs, elements)
    }

    fn render_image(
        &self,
        count: usize,
        dot_size: f64,
        hide_x_dots: usize,
        hide_y_dots: usize,
        image_data: &[u8],
    ) -> String {
        let x_beginning = self.round_size((self.options.width as f64 - count as f64 * dot_size) / 2.0);
        let y_beginning = self.round_size((self.options.height as f64 - count as f64 * dot_size) / 2.0);

        let width = hide_x_dots as f64 * dot_size;
        let height = hide_y_dots as f64 * dot_size;

        let margin = self.options.image_options.margin as f64;
        let dx = x_beginning + self.round_size(margin + (count as f64 * dot_size - width) / 2.0);
        let dy = y_beginning + self.round_size(margin + (count as f64 * dot_size - height) / 2.0);
        let dw = width - margin * 2.0;
        let dh = height - margin * 2.0;

        // Encode image as base64 data URL
        let base64_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, image_data);

        // Detect mime type from image data
        let mime_type = if image_data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            "image/png"
        } else if image_data.starts_with(&[0xFF, 0xD8]) {
            "image/jpeg"
        } else if image_data.starts_with(b"RIFF") && image_data.len() > 12 && &image_data[8..12] == b"WEBP" {
            "image/webp"
        } else {
            "image/png" // Default
        };

        let data_url = format!("data:{};base64,{}", mime_type, base64_data);

        format!(
            r#"<image href="{}" xlink:href="{}" x="{}" y="{}" width="{}px" height="{}px"/>
"#,
            data_url, data_url, dx, dy, dw, dh
        )
    }

    fn create_color(
        &self,
        gradient: Option<&Gradient>,
        color: &Color,
        additional_rotation: f64,
        x: f64,
        y: f64,
        height: f64,
        width: f64,
        name: &str,
    ) -> (String, String) {
        let mut defs = String::new();

        if let Some(grad) = gradient {
            let size = width.max(height);

            match grad.gradient_type {
                GradientType::Radial => {
                    let cx = x + width / 2.0;
                    let cy = y + height / 2.0;
                    let r = size / 2.0;

                    defs.push_str(&format!(
                        r#"<radialGradient id="{}" gradientUnits="userSpaceOnUse" fx="{}" fy="{}" cx="{}" cy="{}" r="{}">
"#,
                        name, cx, cy, cx, cy, r
                    ));

                    for stop in &grad.color_stops {
                        defs.push_str(&format!(
                            r#"<stop offset="{}%" stop-color="{}"/>
"#,
                            stop.offset * 100.0,
                            stop.color.to_hex()
                        ));
                    }

                    defs.push_str("</radialGradient>\n");
                }
                GradientType::Linear => {
                    let rotation = (grad.rotation + additional_rotation) % (2.0 * PI);
                    let positive_rotation = (rotation + 2.0 * PI) % (2.0 * PI);

                    let (mut x0, mut y0, mut x1, mut y1) = (
                        x + width / 2.0,
                        y + height / 2.0,
                        x + width / 2.0,
                        y + height / 2.0,
                    );

                    if (positive_rotation >= 0.0 && positive_rotation <= 0.25 * PI)
                        || (positive_rotation > 1.75 * PI && positive_rotation <= 2.0 * PI)
                    {
                        x0 -= width / 2.0;
                        y0 -= (height / 2.0) * rotation.tan();
                        x1 += width / 2.0;
                        y1 += (height / 2.0) * rotation.tan();
                    } else if positive_rotation > 0.25 * PI && positive_rotation <= 0.75 * PI {
                        y0 -= height / 2.0;
                        x0 -= (width / 2.0) / rotation.tan();
                        y1 += height / 2.0;
                        x1 += (width / 2.0) / rotation.tan();
                    } else if positive_rotation > 0.75 * PI && positive_rotation <= 1.25 * PI {
                        x0 += width / 2.0;
                        y0 += (height / 2.0) * rotation.tan();
                        x1 -= width / 2.0;
                        y1 -= (height / 2.0) * rotation.tan();
                    } else if positive_rotation > 1.25 * PI && positive_rotation <= 1.75 * PI {
                        y0 += height / 2.0;
                        x0 += (width / 2.0) / rotation.tan();
                        y1 -= height / 2.0;
                        x1 -= (width / 2.0) / rotation.tan();
                    }

                    defs.push_str(&format!(
                        r#"<linearGradient id="{}" gradientUnits="userSpaceOnUse" x1="{}" y1="{}" x2="{}" y2="{}">
"#,
                        name,
                        x0.round(),
                        y0.round(),
                        x1.round(),
                        y1.round()
                    ));

                    for stop in &grad.color_stops {
                        defs.push_str(&format!(
                            r#"<stop offset="{}%" stop-color="{}"/>
"#,
                            stop.offset * 100.0,
                            stop.color.to_hex()
                        ));
                    }

                    defs.push_str("</linearGradient>\n");
                }
            }

            (defs, format!("url(#{})", name))
        } else {
            (String::new(), color.to_hex())
        }
    }

    fn should_draw_dot(
        &self,
        row: usize,
        col: usize,
        count: usize,
        hide_x_dots: usize,
        hide_y_dots: usize,
    ) -> bool {
        // Hide dots behind image
        if self.options.image_options.hide_background_dots && self.options.image.is_some() {
            let x_start = (count - hide_x_dots) / 2;
            let x_end = (count + hide_x_dots) / 2;
            let y_start = (count - hide_y_dots) / 2;
            let y_end = (count + hide_y_dots) / 2;

            if row >= y_start && row < y_end && col >= x_start && col < x_end {
                return false;
            }
        }

        // Skip corner squares (finder patterns)
        // Top-left
        if row < 7 && col < 7 {
            if SQUARE_MASK[row * 7 + col] == 1 || DOT_MASK[row * 7 + col] == 1 {
                return false;
            }
        }

        // Top-right
        if row < 7 && col >= count - 7 {
            let local_col = col - (count - 7);
            if SQUARE_MASK[row * 7 + local_col] == 1 || DOT_MASK[row * 7 + local_col] == 1 {
                return false;
            }
        }

        // Bottom-left
        if row >= count - 7 && col < 7 {
            let local_row = row - (count - 7);
            if SQUARE_MASK[local_row * 7 + col] == 1 || DOT_MASK[local_row * 7 + col] == 1 {
                return false;
            }
        }

        true
    }

    fn calculate_image_hide_area(&self, count: usize, _dot_size: f64) -> (usize, usize) {
        // Calculate based on error correction level and image size
        let error_correction_percent = self.options.qr_options.error_correction_level.percentage();
        let cover_level = self.options.image_options.image_size * error_correction_percent;
        let max_hidden_dots = (cover_level * (count * count) as f64).floor() as usize;
        let max_hidden_axis_dots = count.saturating_sub(14);

        // Simple calculation for image area
        // Use aspect ratio 1:1 for simplicity (can be enhanced with actual image dimensions)
        let mut hide_dots = (max_hidden_dots as f64).sqrt().floor() as usize;

        // Ensure odd number for center alignment
        if hide_dots % 2 == 0 {
            hide_dots = hide_dots.saturating_sub(1);
        }

        // Clamp to max
        hide_dots = hide_dots.min(max_hidden_axis_dots);

        (hide_dots, hide_dots)
    }

    fn round_size(&self, value: f64) -> f64 {
        if self.options.dots_options.round_size {
            value.floor()
        } else {
            value
        }
    }
}
