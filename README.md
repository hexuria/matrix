# matrix

**2×3 forced-matrix referral tree** for the Royal Flush Network (RFN).

A matrix holds exactly **7 slots** in a fixed 2-deep tree:

```text
            [1]
           /    \
        [2]      [3]
       /  \     /  \
     [4] [5]  [6] [7]
```

The owner account is always placed in slot 1 at construction. New accounts are
placed **sponsor-first** (in the first vacant *direct child* of their sponsor,
left-first) and fall back to the lowest empty slot otherwise.

This crate is pure domain logic — **no database, no async runtime, no network**.

## Placement rules

1. **Sponsor-preferred.** If the account names a sponsor currently in the
   matrix, it goes into the sponsor's first vacant *direct child* (left-first).
   No recursion into grandchildren — if both children are full, fall back to
   sequential.
2. **Sequential.** Otherwise, the lowest-numbered empty slot (1→7).

**Example:** with only the owner in slot 1, an account sponsored by the owner
lands in slot 2 (the owner's first vacant child). If slots 2 and 3 are both
full, the next owner-sponsored account falls back to slot 4.

## Cycling

When all 7 slots are filled the matrix is *completed*. Calling `cycle()` returns
`(new_matrix, graduates, events)`:

- **`new_matrix`** — a fresh matrix owned by the same owner (slot 1 only).
- **`graduates`** — the 6 non-owner account ids, deterministically ordered by
  slot number (S2, S3, S4, S5, S6, S7).
- **`events`** — a single `MatrixCycled { account_id, matrix_id }` carrying the
  owner and the **new** matrix id.

Cycling does **not** mutate the original matrix — the caller replaces it with
the spawned one.

## Events (outbox pattern)

There is **no async runtime dependency**. `cycle()` returns the emitted events
directly so the caller decides what to do with them (push to a channel, persist,
log, ignore).

```rust
use matrix::{Account, AccountId, Matrix};

let owner = AccountId::generate();
let mut m = Matrix::new(owner);

// Sequential fill: accounts land in slots 2..=7 in order.
for _ in 2..=7 {
    m.add_account(Account::unsponsored(AccountId::generate(), "x")).unwrap();
}
assert!(m.is_full());

// Cycling spawns a fresh matrix + returns the 6 graduates + 1 event.
let (new_m, graduates, events) = m.cycle().unwrap();
assert_eq!(graduates.len(), 6);
assert_eq!(events.len(), 1);
assert_eq!(new_m.owner(), owner);
assert!(!graduates.contains(&owner));
```

## Sponsor-preferred placement

```rust
use matrix::{Account, AccountId, Matrix, SlotNumber};

let owner = AccountId::generate();
let mut m = Matrix::new(owner);
m.add_account(Account::unsponsored(AccountId::generate(), "A")).unwrap(); // slot 2

// Sponsored by the owner -> goes under the owner's free child (slot 3).
let sponsored = AccountId::generate();
let slot = m.add_account(Account::sponsored(sponsored, owner, "B")).unwrap();
assert_eq!(slot, SlotNumber::S3);
```

## Quick start

```toml
[dependencies]
matrix = { path = "../matrix" }
```

```bash
cargo run --example matrix_flow   # full fill + cycle + sponsor demo
cargo doc --no-deps --open        # browse the rustdoc
```

## Testing & verification

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test                        # 21 tests
```

## WebAssembly (WASM) & WASI Support

`matrix` is fully compatible with WebAssembly **out of the box**. It supports compilation for both browser environments (Leptos frontend clients) and server-side WASM sandboxes (such as **Leptos Spin** or **Leptos Wasmtime**).

### 1. Browser-Side WebAssembly (`wasm32-unknown-unknown`)
Pre-configured with `uuid/js` feature enabled, so generating secure `v7` UUIDs requests secure entropy from browser-native JavaScript APIs (`window.crypto.getRandomValues`).
```bash
cargo check --target wasm32-unknown-unknown
```

### 2. Server-Side WASM / WASI (`wasm32-wasip1`)
Compiles seamlessly to WASI for deployments like Spin and Wasmtime. WASI system calls provide entropy natively.
```bash
cargo check --target wasm32-wasip1
```

## License

MIT.

