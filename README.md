# flushmatrix

2×3 forced-matrix referral tree for the **Royal Flush Network (RFN)**.

A matrix holds exactly **7 slots** in a fixed 2-deep tree:

```text
            [1]
           /    \
        [2]      [3]
       /  \     /  \
     [4] [5]  [6] [7]
```

The owner account is always placed in slot 1 at construction. New accounts
are placed **sponsor-first** (in the first vacant *direct child* of their
sponsor, left-first) and fall back to the lowest empty slot otherwise.

## Placement rules

1. **Sponsor-preferred.** If the account names a sponsor currently in the
   matrix, it goes into the sponsor's first vacant *direct child* (left-first).
   No recursion into grandchildren — if both children are full, fall back.
2. **Sequential.** Otherwise, the lowest-numbered empty slot (1→7).

## Cycling

When all 7 slots are filled the matrix is *completed*. Calling `cycle()`
returns `(new_matrix, graduates, events)`:

- **`new_matrix`** — a fresh matrix owned by the same owner (slot 1 only).
- **`graduates`** — the 6 non-owner account ids, deterministically ordered by
  slot (cleanup: the old code returned them in non-deterministic HashMap order).
- **`events`** — a single [`flushevents::MatrixCycled`] carrying the owner and
  the **new** matrix id.

Cycling does **not** mutate the original matrix — the caller replaces it with
the spawned one.

## Events (outbox pattern)

There is **no async runtime dependency**. `cycle()` returns the emitted events
directly:

```rust
use flushmatrix::{Account, AccountId, Matrix};

let owner = AccountId::generate();
let mut m = Matrix::new(owner);
for _ in 2..=7 {
    m.add_account(Account::unsponsored(AccountId::generate(), "x")).unwrap();
}
let (new_m, graduates, events) = m.cycle()?;
assert_eq!(graduates.len(), 6);
assert_eq!(events.len(), 1);
```

## Usage

```toml
[dependencies]
flushmatrix = "0.1"
```

## Testing & verification

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run --example matrix_flow
```

## Related crates

- [`flushevents`](../flushevents) — shared event payloads.
- [`flushline`](../flushline) — 5-tier card progression engine.
- [`royalflush`](../royalflush) — weekly 75-25 pot bonus distribution.
