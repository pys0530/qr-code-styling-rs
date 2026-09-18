//! QR code matrix wrapper providing neighbor lookup functionality.

use crate::config::QROptions;
use crate::error::{QRError, Result};
use fast_qr::{QRBuilder, Version};

fn type_number_to_version(type_number: u8) -> Option<Version> {
    match type_number {
        1 => Some(Version::V01),
        2 => Some(Version::V02),
        3 => Some(Version::V03),
        4 => Some(Version::V04),
        5 => Some(Version::V05),
        6 => Some(Version::V06),
        7 => Some(Version::V07),
        8 => Some(Version::V08),
        9 => Some(Version::V09),
        10 => Some(Version::V10),
        11 => Some(Version::V11),
        12 => Some(Version::V12),
        13 => Some(Version::V13),
        14 => Some(Version::V14),
        15 => Some(Version::V15),
        16 => Some(Version::V16),
        17 => Some(Version::V17),
        18 => Some(Version::V18),
        19 => Some(Version::V19),
        20 => Some(Version::V20),
        21 => Some(Version::V21),
        22 => Some(Version::V22),
        23 => Some(Version::V23),
        24 => Some(Version::V24),
        25 => Some(Version::V25),
        26 => Some(Version::V26),
        27 => Some(Version::V27),
        28 => Some(Version::V28),
        29 => Some(Version::V29),
        30 => Some(Version::V30),
        31 => Some(Version::V31),
        32 => Some(Version::V32),
        33 => Some(Version::V33),
        34 => Some(Version::V34),
        35 => Some(Version::V35),
        36 => Some(Version::V36),
        37 => Some(Version::V37),
        38 => Some(Version::V38),
        39 => Some(Version::V39),
        40 => Some(Version::V40),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FinderRegion {
    None,
    Outer,
    Inner,
    Body,
}

/// Wrapper around the QR code matrix providing efficient module access.
#[derive(Debug, Clone)]
pub struct QRMatrix {
    /// Flat array of module values (true = dark, false = light).
    modules: Vec<bool>,
    /// Size of the QR code (number of modules per side).
    size: usize,
}

impl QRMatrix {
    /// Create a new QR matrix from data with the specified options.
    pub fn new(data: &str, options: &QROptions) -> Result<Self> {
        let ecl = options.error_correction_level.to_fast_qr_ecl();

        // Determine the version
        let version = if options.type_number == 0 {
            None // Auto-detect
        } else {
            Some(
                type_number_to_version(options.type_number)
                    .ok_or_else(|| QRError::QRGenerationError(format!("Invalid QR version: {}", options.type_number)))?
            )
        };

        // Build the QR code
        let mut builder = QRBuilder::new(data.as_bytes());
        builder.ecl(ecl);
        if let Some(v) = version {
            builder.version(v);
        }
        let qr = builder
            .build()
            .map_err(|e| QRError::QRGenerationError(e.to_string()))?;

        let size = qr.size;
        let mut modules = Vec::with_capacity(size * size);

        // Convert to flat array for O(1) access
        // fast_qr stores data as [Module; 31329] (max 177x177), only first size*size are valid
        for y in 0..size {
            for x in 0..size {
                modules.push(qr.data[y * size + x].value());
            }
        }

        Ok(Self { modules, size })
    }

    /// Get the size (width/height) of the QR code in modules.
    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get the module count (same as size for compatibility).
    #[inline]
    pub fn module_count(&self) -> usize {
        self.size
    }

    /// Check if a module at (row, col) is dark.
    #[inline]
    pub fn is_dark(&self, row: usize, col: usize) -> bool {
        if row >= self.size || col >= self.size {
            return false;
        }
        self.modules[row * self.size + col]
    }

    /// Check if a module at (row, col) is dark, with signed coordinates.
    /// Returns false for out-of-bounds coordinates.
    #[inline]
    pub fn is_dark_signed(&self, row: i32, col: i32) -> bool {
        if row < 0 || col < 0 {
            return false;
        }
        self.is_dark(row as usize, col as usize)
    }

    /// Get neighbor state relative to a position.
    #[inline]
    pub fn get_neighbor(&self, row: i32, col: i32, offset_x: i32, offset_y: i32) -> bool {
        self.is_dark_signed(row + offset_y, col + offset_x)
    }
    
    pub fn finder_region(&self, row: usize, col: usize) -> FinderRegion {
        let size = self.size;

        let corner_origin = if row < 7 && col < 7 {
            Some((0, 0))
        } else if row < 7 && col >= size - 7 {
            Some((0, size - 7))
        } else if row >= size - 7 && col < 7 {
            Some((size - 7, 0))
        } else {
            None
        };

        let Some((start_r, start_c)) = corner_origin else {
            return FinderRegion::None;
        };

        let local_r = row - start_r;
        let local_c = col - start_c;

        if local_r == 0 || local_r == 6 || local_c == 0 || local_c == 6 {
            FinderRegion::Outer
        } else if (2..=4).contains(&local_r) && (2..=4).contains(&local_c) {
            FinderRegion::Inner
        } else {
            FinderRegion::Body
        }
    }

    /// Check if a position is part of a finder pattern (corner square).
    /// Finder patterns are 7x7 and located at:
    /// - Top-left: (0, 0)
    /// - Top-right: (0, size-7)
    /// - Bottom-left: (size-7, 0)
    pub fn is_finder_pattern(&self, row: usize, col: usize) -> bool {
        self.finder_region(row, col) != FinderRegion::None
    }
    pub fn is_finder_pattern_outer(&self, row: usize, col: usize) -> bool {
        self.finder_region(row, col) == FinderRegion::Outer
    }
    pub fn is_finder_pattern_inner(&self, row: usize, col: usize) -> bool {
        self.finder_region(row, col) == FinderRegion::Inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_matrix_creation() {
        let options = QROptions::default();
        let matrix = QRMatrix::new("Hello", &options).unwrap();
        assert!(matrix.size() >= 21); // Minimum QR code size
    }

    #[test]
    fn test_is_dark() {
        let options = QROptions::default();
        let matrix = QRMatrix::new("Test", &options).unwrap();

        // Finder pattern top-left corner should be dark
        assert!(matrix.is_dark(0, 0));
    }

    #[test]
    fn test_neighbor_lookup() {
        let options = QROptions::default();
        let matrix = QRMatrix::new("Test", &options).unwrap();

        // Test that neighbor lookup works
        let dark = matrix.is_dark(0, 0);
        let neighbor = matrix.get_neighbor(0, 1, -1, 0);
        assert_eq!(dark, neighbor);
    }

    #[test]
    fn test_finder_pattern_detection() {
        let options = QROptions::default();
        let matrix = QRMatrix::new("Test", &options).unwrap();

        // Top-left corner should be finder pattern
        assert!(matrix.is_finder_pattern(0, 0));
        assert!(matrix.is_finder_pattern(3, 3));
        assert!(matrix.is_finder_pattern(6, 6));

        // Middle of QR code should not be finder pattern
        let mid = matrix.size() / 2;
        assert!(!matrix.is_finder_pattern(mid, mid));
    }
}