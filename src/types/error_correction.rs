//! QR code error correction levels.

use fast_qr::ECL as FastQrECL;

/// Error correction level for QR codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ErrorCorrectionLevel {
    /// Low error correction (~7% recovery).
    L,
    /// Medium error correction (~15% recovery).
    M,
    /// Quartile error correction (~25% recovery, default).
    #[default]
    Q,
    /// High error correction (~30% recovery).
    H,
}

impl ErrorCorrectionLevel {
    /// Returns the error correction percentage (0.0 to 1.0).
    pub fn percentage(&self) -> f64 {
        match self {
            ErrorCorrectionLevel::L => 0.07,
            ErrorCorrectionLevel::M => 0.15,
            ErrorCorrectionLevel::Q => 0.25,
            ErrorCorrectionLevel::H => 0.30,
        }
    }

    /// Converts to the fast_qr crate's ECL.
    pub fn to_fast_qr_ecl(&self) -> FastQrECL {
        match self {
            ErrorCorrectionLevel::L => FastQrECL::L,
            ErrorCorrectionLevel::M => FastQrECL::M,
            ErrorCorrectionLevel::Q => FastQrECL::Q,
            ErrorCorrectionLevel::H => FastQrECL::H,
        }
    }
}
