//! # Matrix Flow Example
//!
//! Mirrors rfn's `matrix/examples/flushline_integration.rs`: builds a matrix,
//! fills it sequentially, cycles it, then demonstrates sponsor-preferred
//! placement.
//!
//! Run with: `cargo run --example matrix_flow`

use flushmatrix::{Account, AccountId, Matrix, MatrixStatus, SlotNumber};

fn main() {
    println!("=== flushmatrix flow ===\n");

    // Scenario 1: sequential fill + cycle.
    let owner = AccountId::generate();
    let mut m = Matrix::new(owner);
    println!("created matrix {} owned by {owner}", m.id());

    for slot in 2..=7 {
        let id = AccountId::generate();
        let placed = m
            .add_account(Account::unsponsored(id, format!("acct-{slot}")))
            .unwrap();
        assert_eq!(placed.as_u8(), slot);
    }
    println!("\nafter 6 sequential adds:");
    print_matrix(&m);
    println!("status: {:?}, full: {}", m.status(), m.is_full());

    let (new_m, graduates, events) = m.cycle().unwrap();
    println!(
        "\ncycled -> {} graduates, {} event(s)",
        graduates.len(),
        events.len()
    );
    println!("new matrix {} (owner {owner})", new_m.id());
    assert_eq!(graduates.len(), 6);
    assert_eq!(new_m.status(), MatrixStatus::Filling);
    assert_eq!(new_m.account_in_slot(SlotNumber::S1), Some(owner));

    // Scenario 2: sponsor-preferred placement.
    println!("\n--- sponsor placement ---");
    let sponsor = AccountId::generate();
    let mut m2 = Matrix::new(sponsor);

    // Sequential first: slot 2.
    let a = AccountId::generate();
    let s = m2.add_account(Account::unsponsored(a, "A")).unwrap();
    assert_eq!(s, SlotNumber::S2);

    // Sponsored by the owner (slot 1) -> its free child is slot 3.
    let b = AccountId::generate();
    let s = m2.add_account(Account::sponsored(b, sponsor, "B")).unwrap();
    assert_eq!(s, SlotNumber::S3);

    // Sponsored by A (slot 2) -> its free child is slot 4.
    let c = AccountId::generate();
    let s = m2.add_account(Account::sponsored(c, a, "C")).unwrap();
    assert_eq!(s, SlotNumber::S4);

    println!("final slot map: ");
    print_matrix(&m2);
    println!("\nmatrix accepts more placements until all 7 slots fill.");
}

fn print_matrix(m: &Matrix) {
    let slot = |n| {
        m.account_in_slot(SlotNumber::new(n).unwrap())
            .map(|id| short(&id))
            .unwrap_or_else(|| "------".to_string())
    };
    println!(
        "           [{}]\n          /        \\ \n    [{}]        [{}]\n   /        \\      /        \\ \n[{}][{}]  [{}][{}]",
        slot(1),
        slot(2),
        slot(3),
        slot(4),
        slot(5),
        slot(6),
        slot(7)
    );
}

fn short(id: &AccountId) -> String {
    let s = id.to_string();
    s.split('-').next().unwrap_or(&s).to_string()
}
