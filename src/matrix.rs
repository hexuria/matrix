//! The matrix aggregate, its slot layout, and the cycling engine.

use crate::account::Account;
use crate::error::MatrixError;
use flushevents::MatrixCycled;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Fixed matrix size: 7 slots in a 2×3 layout.
pub const MATRIX_SIZE: u8 = 7;

/// Unique identifier for an account in a matrix (wraps a v7 UUID).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AccountId(Uuid);

impl AccountId {
    /// Generate a fresh time-sortable id.
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for AccountId {
    fn default() -> Self {
        Self::generate()
    }
}

impl From<Uuid> for AccountId {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}

impl From<AccountId> for Uuid {
    fn from(id: AccountId) -> Uuid {
        id.0
    }
}

impl std::fmt::Display for AccountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for AccountId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

/// Unique identifier for a matrix (wraps a v7 UUID).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MatrixId(Uuid);

impl MatrixId {
    /// Generate a fresh time-sortable id.
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for MatrixId {
    fn default() -> Self {
        Self::generate()
    }
}

impl From<Uuid> for MatrixId {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}

impl From<MatrixId> for Uuid {
    fn from(id: MatrixId) -> Uuid {
        id.0
    }
}

impl std::fmt::Display for MatrixId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The seven fixed slots in a matrix. Slot 1 is always the owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum SlotNumber {
    S1 = 1,
    S2 = 2,
    S3 = 3,
    S4 = 4,
    S5 = 5,
    S6 = 6,
    S7 = 7,
}

impl SlotNumber {
    /// Parse a u8 into a slot, validating `1..=7`.
    pub fn new(slot: u8) -> Result<Self, MatrixError> {
        match slot {
            1 => Ok(SlotNumber::S1),
            2 => Ok(SlotNumber::S2),
            3 => Ok(SlotNumber::S3),
            4 => Ok(SlotNumber::S4),
            5 => Ok(SlotNumber::S5),
            6 => Ok(SlotNumber::S6),
            7 => Ok(SlotNumber::S7),
            other => Err(MatrixError::InvalidSlotNumber(other)),
        }
    }

    #[inline]
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// All seven slots, lowest first.
    pub fn all() -> [SlotNumber; 7] {
        [
            SlotNumber::S1,
            SlotNumber::S2,
            SlotNumber::S3,
            SlotNumber::S4,
            SlotNumber::S5,
            SlotNumber::S6,
            SlotNumber::S7,
        ]
    }
}

/// Lifecycle state of a matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatrixStatus {
    /// Still accepting accounts.
    Filling,
    /// All 7 slots occupied; eligible for cycling.
    Completed,
}

/// The 2×3 forced-matrix aggregate (7 slots).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Matrix {
    id: MatrixId,
    status: MatrixStatus,
    /// Ordered slot map. BTreeMap keeps slots deterministically ordered by
    /// SlotNumber (the old code used a HashMap, producing non-deterministic
    /// graduate ordering on cycle).
    slots: BTreeMap<SlotNumber, AccountId>,
    owner: AccountId,
}

impl Matrix {
    /// Create a fresh matrix owned by `owner`. The owner occupies slot 1.
    pub fn new(owner: AccountId) -> Self {
        let mut slots = BTreeMap::new();
        slots.insert(SlotNumber::S1, owner);
        Self {
            id: MatrixId::generate(),
            status: MatrixStatus::Filling,
            slots,
            owner,
        }
    }

    /// Construct with a caller-supplied id (for replay/testing).
    pub fn with_id(id: MatrixId, owner: AccountId) -> Self {
        let mut m = Self::new(owner);
        m.id = id;
        m
    }

    // ----- queries -------------------------------------------------------

    pub fn id(&self) -> MatrixId {
        self.id
    }

    pub fn status(&self) -> MatrixStatus {
        self.status
    }

    pub fn owner(&self) -> AccountId {
        self.owner
    }

    /// Number of occupied slots.
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Is every slot occupied?
    pub fn is_full(&self) -> bool {
        self.slots.len() as u8 == MATRIX_SIZE
    }

    /// The account occupying `slot`, if any.
    pub fn account_in_slot(&self, slot: SlotNumber) -> Option<AccountId> {
        self.slots.get(&slot).copied()
    }

    /// Direct children of a slot, left-first. Slot 1 → slots 2 and 3,
    /// slot 2 → slots 4 and 5, slot 3 → slots 6 and 7; leaves (4-7) return
    /// an empty vec.
    pub fn children_of(parent: SlotNumber) -> Vec<SlotNumber> {
        match parent {
            SlotNumber::S1 => vec![SlotNumber::S2, SlotNumber::S3],
            SlotNumber::S2 => vec![SlotNumber::S4, SlotNumber::S5],
            SlotNumber::S3 => vec![SlotNumber::S6, SlotNumber::S7],
            _ => Vec::new(),
        }
    }

    // ----- placement -----------------------------------------------------

    /// Place `account` into the matrix.
    ///
    /// Order: if the account names a sponsor currently in the matrix, place
    /// under the sponsor's first vacant **direct child** (left-first, no
    /// recursion into grandchildren). Otherwise place in the lowest empty slot.
    /// Returns the chosen slot, or an error if the matrix is full.
    pub fn add_account(&mut self, account: Account) -> Result<SlotNumber, MatrixError> {
        if self.is_full() {
            return Err(MatrixError::MatrixFull);
        }

        let id = account.id;
        let mut placed = None;

        // 1. Sponsor-preferred placement under a direct child.
        if let Some(sponsor) = account.sponsor_id {
            if let Some(sponsor_slot) = self.slot_of(sponsor) {
                for child in Matrix::children_of(sponsor_slot) {
                    use std::collections::btree_map::Entry;
                    if let Entry::Vacant(e) = self.slots.entry(child) {
                        e.insert(id);
                        placed = Some(child);
                        break;
                    }
                }
            }
        }

        // 2. Sequential fallback: lowest empty slot.
        if placed.is_none() {
            for slot in SlotNumber::all() {
                use std::collections::btree_map::Entry;
                if let Entry::Vacant(e) = self.slots.entry(slot) {
                    e.insert(id);
                    placed = Some(slot);
                    break;
                }
            }
        }

        let placed = placed.expect("a slot is always free when not full");

        if self.is_full() {
            self.status = MatrixStatus::Completed;
        }
        Ok(placed)
    }

    // ----- cycling -------------------------------------------------------

    /// Cycle a full matrix.
    ///
    /// Returns `(new_matrix, graduates, events)` where:
    /// - `new_matrix` is a fresh matrix owned by the same owner (slot 1 only);
    /// - `graduates` are the 6 non-owner account ids, **deterministically
    ///   ordered by slot** (cleanup vs the old HashMap-nondeterministic order);
    /// - `events` contains a single [`MatrixCycled`] carrying the owner and the
    ///   **new** matrix id.
    ///
    /// Does NOT mutate `self` (the old matrix stays full/completed).
    pub fn cycle(&self) -> Result<(Matrix, Vec<AccountId>, Vec<MatrixCycled>), MatrixError> {
        if !self.is_full() {
            return Err(MatrixError::MatrixNotFull);
        }

        let graduates: Vec<AccountId> = self
            .slots
            .iter()
            .filter(|(slot, _)| **slot != SlotNumber::S1)
            .map(|(_, id)| *id)
            .collect();

        let new_matrix = Matrix::new(self.owner);
        let events = vec![MatrixCycled {
            account_id: self.owner.into(),
            matrix_id: new_matrix.id.into(),
        }];
        Ok((new_matrix, graduates, events))
    }

    // ----- internals -----------------------------------------------------

    fn slot_of(&self, account: AccountId) -> Option<SlotNumber> {
        self.slots
            .iter()
            .find_map(|(slot, id)| (*id == account).then_some(*slot))
    }
}

impl Default for Matrix {
    fn default() -> Self {
        Self::new(AccountId::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn children_of_matches_fixed_layout() {
        assert_eq!(
            Matrix::children_of(SlotNumber::S1),
            vec![SlotNumber::S2, SlotNumber::S3]
        );
        assert_eq!(
            Matrix::children_of(SlotNumber::S2),
            vec![SlotNumber::S4, SlotNumber::S5]
        );
        assert_eq!(
            Matrix::children_of(SlotNumber::S3),
            vec![SlotNumber::S6, SlotNumber::S7]
        );
        assert!(Matrix::children_of(SlotNumber::S4).is_empty());
        assert!(Matrix::children_of(SlotNumber::S7).is_empty());
    }
}
