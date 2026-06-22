//! # flushmatrix
//!
//! 2×3 forced-matrix referral tree for the Royal Flush Network.
//!
//! A matrix holds exactly **7 slots** in a fixed 2-deep tree:
//!
//! ```text
//!            [1]
//!           /    \
//!        [2]      [3]
//!       /  \     /  \
//!     [4] [5]  [6] [7]
//! ```
//!
//! The owner account is always placed in slot 1 at construction. New accounts
//! are placed **sponsor-first** (in the first vacant direct child of their
//! sponsor, left-first) and fall back to the lowest empty slot otherwise. When
//! all 7 slots are filled the matrix is *completed* and may be **cycled**:
//! spawning a fresh matrix for the same owner and graduating the 6 non-owner
//! accounts out.
//!
//! ## Events
//!
//! Cycling emits a [`flushevents::MatrixCycled`] event via the outbox pattern
//! (no async runtime coupling): `cycle` returns `(new_matrix, graduates,
//! Vec<MatrixCycled>)`.

mod account;
mod error;
mod matrix;

pub use account::Account;
pub use error::MatrixError;
pub use matrix::{AccountId, Matrix, MatrixId, MatrixStatus, SlotNumber, MATRIX_SIZE};

pub use flushevents;
