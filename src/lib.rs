//! # matrix
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
//! ## Placement rules
//!
//! 1. **Sponsor-preferred.** If the account names a sponsor currently in the
//!    matrix, it goes into the sponsor's first vacant *direct child*
//!    (left-first). No recursion into grandchildren — if both children are
//!    full, fall back to sequential.
//! 2. **Sequential.** Otherwise, the lowest-numbered empty slot (1→7).
//!
//! ## Events
//!
//! Cycling emits a [`events::MatrixCycled`] event via the outbox pattern
//! (no async runtime coupling): `cycle` returns `(new_matrix, graduates,
//! Vec<MatrixCycled>)`.
//!
//! # Quick start
//!
//! ```
//! use matrix::{Account, AccountId, Matrix};
//!
//! let owner = AccountId::generate();
//! let mut m = Matrix::new(owner);
//!
//! // Sequential fill: accounts land in slots 2..=7 in order.
//! for _ in 2..=7 {
//!     m.add_account(Account::unsponsored(AccountId::generate(), "x")).unwrap();
//! }
//! assert!(m.is_full());
//!
//! // Cycling spawns a fresh matrix for the same owner + returns 6 graduates.
//! let (new_m, graduates, events) = m.cycle().unwrap();
//! assert_eq!(graduates.len(), 6);
//! assert_eq!(events.len(), 1);
//! assert_eq!(new_m.owner(), owner);
//! ```

mod account;
mod error;
pub mod events;
mod matrix;
#[cfg(feature = "db")]
mod repository;

pub use account::Account;
pub use error::MatrixError;
pub use events::MatrixCycled;
pub use matrix::{AccountId, Matrix, MatrixId, MatrixStatus, SlotNumber, MATRIX_SIZE};
#[cfg(feature = "db")]
pub use repository::{MatrixRepository, PgMatrixRepository};
