//! Placement rules: sequential fill, sponsor-preferred, and fallback.
//! Ported from rfn/matrix/src/domain/tests.rs.

use matrix::{Account, AccountId, Matrix, MatrixStatus, SlotNumber, MATRIX_SIZE};

fn owner_matrix() -> (AccountId, Matrix) {
    let owner = AccountId::generate();
    (owner, Matrix::new(owner))
}

#[test]
fn new_matrix_initializes_with_owner_in_slot_one() {
    let (owner, m) = owner_matrix();
    assert_eq!(m.slot_count(), 1);
    assert_eq!(m.account_in_slot(SlotNumber::S1), Some(owner));
    assert_eq!(m.status(), MatrixStatus::Filling);
    assert_eq!(m.owner(), owner);
    assert!(!m.is_full());
}

#[test]
fn unsponsored_accounts_fill_slots_two_through_seven_in_order() {
    let (_owner, mut m) = owner_matrix();
    for slot_u8 in 2..=MATRIX_SIZE {
        let id = AccountId::generate();
        let slot = m.add_account(Account::unsponsored(id, "test")).unwrap();
        assert_eq!(slot, SlotNumber::new(slot_u8).unwrap());
        assert_eq!(
            m.account_in_slot(SlotNumber::new(slot_u8).unwrap()),
            Some(id)
        );
    }
    assert!(m.is_full());
    assert_eq!(m.status(), MatrixStatus::Completed);
}

#[test]
fn adding_to_full_matrix_errors() {
    let (_owner, mut m) = owner_matrix();
    for _ in 2..=MATRIX_SIZE {
        m.add_account(Account::unsponsored(AccountId::generate(), "x"))
            .unwrap();
    }
    let result = m.add_account(Account::unsponsored(AccountId::generate(), "overflow"));
    assert!(matches!(result, Err(matrix::MatrixError::MatrixFull)));
}

#[test]
fn sponsored_by_owner_with_slot_two_filled_goes_to_slot_three() {
    let (owner, mut m) = owner_matrix();
    m.add_account(Account::unsponsored(AccountId::generate(), "a"))
        .unwrap(); // slot 2

    let sponsored = AccountId::generate();
    let slot = m
        .add_account(Account::sponsored(sponsored, owner, "s"))
        .unwrap();
    assert_eq!(slot, SlotNumber::S3);
    assert_eq!(m.account_in_slot(SlotNumber::S3), Some(sponsored));
}

#[test]
fn sponsored_by_owner_both_children_open_takes_slot_two_first() {
    let (owner, mut m) = owner_matrix();
    let a = AccountId::generate();
    let s1 = m.add_account(Account::sponsored(a, owner, "s1")).unwrap();
    assert_eq!(s1, SlotNumber::S2);

    let b = AccountId::generate();
    let s2 = m.add_account(Account::sponsored(b, owner, "s2")).unwrap();
    assert_eq!(s2, SlotNumber::S3);
}

#[test]
fn sponsor_children_full_falls_back_to_sequential_slot_four() {
    let (owner, mut m) = owner_matrix();
    m.add_account(Account::unsponsored(AccountId::generate(), "c2"))
        .unwrap(); // slot 2
    m.add_account(Account::unsponsored(AccountId::generate(), "c3"))
        .unwrap(); // slot 3

    // Owner's direct children (2,3) full -> no recursion into grandchildren,
    // fall back to lowest empty slot (4).
    let s = AccountId::generate();
    let slot = m
        .add_account(Account::sponsored(s, owner, "fallback"))
        .unwrap();
    assert_eq!(slot, SlotNumber::S4);
    assert_eq!(m.account_in_slot(SlotNumber::S4), Some(s));
}

#[test]
fn sponsor_not_in_matrix_falls_back_to_sequential() {
    let (_owner, mut m) = owner_matrix();
    let ghost_sponsor = AccountId::generate();
    let s = AccountId::generate();
    let slot = m
        .add_account(Account::sponsored(s, ghost_sponsor, "ghost"))
        .unwrap();
    assert_eq!(slot, SlotNumber::S2);
    assert_eq!(m.account_in_slot(SlotNumber::S2), Some(s));
}

#[test]
fn sponsored_by_non_owner_goes_under_their_child_slot() {
    let (owner, mut m) = owner_matrix();
    let sponsor = AccountId::generate();
    m.add_account(Account::unsponsored(sponsor, "non_owner"))
        .unwrap(); // slot 2

    let s = AccountId::generate();
    let slot = m
        .add_account(Account::sponsored(s, sponsor, "child_of_2"))
        .unwrap();
    assert_eq!(slot, SlotNumber::S4); // child of slot 2
    assert_eq!(m.account_in_slot(SlotNumber::S4), Some(s));
    let _ = owner; // owner unused beyond construction
}

#[test]
fn mixed_sequential_and_sponsored_placement_matches_expected_slots() {
    let (owner, mut m) = owner_matrix();

    let acc2 = AccountId::generate();
    let s = m.add_account(Account::unsponsored(acc2, "seq2")).unwrap();
    assert_eq!(s, SlotNumber::S2);

    let acc3 = AccountId::generate();
    let s = m
        .add_account(Account::sponsored(acc3, owner, "sp"))
        .unwrap();
    assert_eq!(s, SlotNumber::S3); // sponsored by owner -> slot 3

    let acc4 = AccountId::generate();
    let s = m.add_account(Account::unsponsored(acc4, "seq3")).unwrap();
    assert_eq!(s, SlotNumber::S4);

    let acc5 = AccountId::generate();
    let s = m
        .add_account(Account::sponsored(acc5, acc2, "sp2"))
        .unwrap();
    assert_eq!(s, SlotNumber::S5); // child of slot 2

    let acc6 = AccountId::generate();
    let s = m.add_account(Account::unsponsored(acc6, "seq4")).unwrap();
    assert_eq!(s, SlotNumber::S6);

    let acc7 = AccountId::generate();
    let s = m.add_account(Account::unsponsored(acc7, "seq5")).unwrap();
    assert_eq!(s, SlotNumber::S7);

    assert!(m.is_full());
    assert_eq!(m.status(), MatrixStatus::Completed);
}
