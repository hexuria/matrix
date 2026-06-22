//! Errors raised by the matrix domain.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MatrixError {
    #[error("matrix is full (all 7 slots occupied)")]
    MatrixFull,
    /// Cleanup: the old code reused `MatrixFull` for the not-full guard on
    /// `cycle`. We expose a dedicated variant so the failure mode is legible.
    #[error("matrix is not full; cannot cycle")]
    MatrixNotFull,
    #[error("invalid slot number {0} (must be 1..=7)")]
    InvalidSlotNumber(u8),
}
