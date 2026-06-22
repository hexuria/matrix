//! Matrix geometry: the fixed 2x3 / 7-slot layout and parent->child mapping.

use flushmatrix::{Matrix, SlotNumber};

#[test]
fn matrix_size_is_seven() {
    assert_eq!(flushmatrix::MATRIX_SIZE, 7);
}

#[test]
fn slot_number_validates_one_through_seven() {
    assert!(SlotNumber::new(0).is_err());
    assert!(SlotNumber::new(8).is_err());
    assert!(SlotNumber::new(1).is_ok());
    assert!(SlotNumber::new(7).is_ok());
    assert_eq!(SlotNumber::new(3).unwrap().as_u8(), 3);
}

#[test]
fn children_of_slot_one_are_two_and_three() {
    let kids = Matrix::children_of(SlotNumber::S1);
    assert_eq!(kids.len(), 2);
    assert!(kids.contains(&SlotNumber::S2));
    assert!(kids.contains(&SlotNumber::S3));
}

#[test]
fn children_of_slot_two_are_four_and_five() {
    let kids = Matrix::children_of(SlotNumber::S2);
    assert_eq!(kids.len(), 2);
    assert!(kids.contains(&SlotNumber::S4));
    assert!(kids.contains(&SlotNumber::S5));
}

#[test]
fn children_of_slot_three_are_six_and_seven() {
    let kids = Matrix::children_of(SlotNumber::S3);
    assert_eq!(kids.len(), 2);
    assert!(kids.contains(&SlotNumber::S6));
    assert!(kids.contains(&SlotNumber::S7));
}

#[test]
fn leaves_have_no_children() {
    assert!(Matrix::children_of(SlotNumber::S4).is_empty());
    assert!(Matrix::children_of(SlotNumber::S5).is_empty());
    assert!(Matrix::children_of(SlotNumber::S6).is_empty());
    assert!(Matrix::children_of(SlotNumber::S7).is_empty());
}
