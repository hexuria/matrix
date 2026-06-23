//! Cycling & the event outbox. Ported from rfn cycle tests.
//!
//! Cleanup note: the old `DomainError::MatrixFull` was misused for the "not
//! full" guard. We expose a dedicated `MatrixNotFull` variant.

use matrix::{Account, AccountId, Matrix, MatrixError, MatrixStatus, SlotNumber, MATRIX_SIZE};

fn full_matrix(owner: AccountId) -> (Matrix, Vec<AccountId>) {
    let mut m = Matrix::new(owner);
    let mut others = Vec::new();
    for _ in 2..=MATRIX_SIZE {
        let id = AccountId::generate();
        m.add_account(Account::unsponsored(id, "x")).unwrap();
        others.push(id);
    }
    (m, others)
}

#[test]
fn cycle_requires_a_full_matrix() {
    let owner = AccountId::generate();
    let mut m = Matrix::new(owner);
    m.add_account(Account::unsponsored(AccountId::generate(), "x"))
        .unwrap();
    assert!(!m.is_full());

    let result = m.cycle();
    // Cleanup: dedicated error, not the reused MatrixFull.
    assert!(matches!(result, Err(MatrixError::MatrixNotFull)));
}

#[test]
fn cycle_spawns_fresh_matrix_for_same_owner() {
    let owner = AccountId::generate();
    let (m, _) = full_matrix(owner);
    let old_id = m.id();
    let (new_m, _, events) = m.cycle().unwrap();

    assert_ne!(new_m.id(), old_id);
    assert_eq!(new_m.owner(), owner);
    assert_eq!(new_m.slot_count(), 1); // only owner in slot 1
    assert_eq!(new_m.status(), MatrixStatus::Filling);
    let _ = events;
}

#[test]
fn cycle_graduates_six_non_owner_accounts() {
    let owner = AccountId::generate();
    let (m, mut others) = full_matrix(owner);
    let (new_m, graduated, _) = m.cycle().unwrap();

    assert_eq!(graduated.len() as u8, MATRIX_SIZE - 1);
    assert!(!graduated.contains(&owner));
    for id in others.drain(..) {
        assert!(graduated.contains(&id), "{id:?} should be in graduates");
    }
    // New matrix owner is the only account in slot 1.
    assert_eq!(new_m.account_in_slot(SlotNumber::S1), Some(owner));
}

#[test]
fn cycle_emits_matrix_cycled_outbox_event() {
    let owner = AccountId::generate();
    let (m, _) = full_matrix(owner);
    let (_new_m, _grad, events) = m.cycle().unwrap();
    assert_eq!(events.len(), 1);
    let owner_uuid: uuid::Uuid = owner.into();
    assert_eq!(
        events[0].account_id, owner_uuid,
        "event carries the owner (cycled) account"
    );
    // matrix_id is the FRESH matrix id — that's the spawn we want to track.
    let _ = events;
}

#[test]
fn cycle_does_not_mutate_the_original_matrix() {
    let owner = AccountId::generate();
    let (m, _) = full_matrix(owner);
    let original_status = m.status();
    let _ = m.cycle().unwrap();
    // The old matrix is untouched (rfn behavior: cycle is &self).
    assert_eq!(m.status(), original_status);
    assert!(m.is_full());
}
