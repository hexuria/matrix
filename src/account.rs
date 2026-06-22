//! The account placed into a matrix — identity + optional sponsor + label.

use crate::matrix::AccountId;
use serde::{Deserialize, Serialize};

/// An account seeking placement in a matrix.
///
/// Carries the optional sponsor id used for sponsor-preferred placement and a
/// human-readable label (the matrix stores only the id; the label is for
/// diagnostics).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: AccountId,
    pub sponsor_id: Option<AccountId>,
    pub label: String,
}

impl Account {
    /// Sponsor-less account (sequential placement).
    pub fn unsponsored(id: AccountId, label: impl Into<String>) -> Self {
        Self {
            id,
            sponsor_id: None,
            label: label.into(),
        }
    }

    /// Account with a sponsor (sponsor-preferred placement).
    pub fn sponsored(id: AccountId, sponsor_id: AccountId, label: impl Into<String>) -> Self {
        Self {
            id,
            sponsor_id: Some(sponsor_id),
            label: label.into(),
        }
    }
}
