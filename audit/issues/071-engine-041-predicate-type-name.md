# engine-041: the sealed signature type is named `Predicate` — Datalog vocabulary in the type name

- **Severity:** low
- **Tree:** engine
- **Status:** FIXED(4c21fafd)
- **Source:** adversarial pass (not in audit/engine.md; traced from docs findings that teach "the predicate the query defines")
- **Depends on:** engine-005/engine-006 (witness/sealing restructure first, to avoid renaming a type mid-churn)

## The bug

`crates/bumbledb/src/ir/validate.rs:111` — the sealed main/derived signature type:

```rust
pub struct Predicate {
```

with `pub fn predicate(&self) -> &Predicate` accessors at `validate.rs:359,384,413,483` and a re-export on the prepared surface (`api/prepared/introspect.rs:460`). It is consumed across `api/prepared/build.rs` and ~20 bench call sites. Downstream docs then teach "the predicate the query defines" because the code offers no other word.

## Why it's wrong

Names are representation (Insight 1): a sealed answer/interior *signature* named `Predicate` is the Program coordinate system living in the type name, and it propagates outward — every doc and SDK that touches the surface inherits the Datalog vocabulary the cut deleted.

## The fix

Per `audit/CONTRACT.md §C3` (Signature naming amendment): mechanical rename of the **sealed signature type** `ir/validate::Predicate` → `Signature`, `PredicateColumn` → `SignatureColumn` (`validate.rs:152`), accessors `predicate()` → `signature()` including the public `PreparedQuery::predicate()` (`introspect.rs:460`) and `IntrospectionHeader.predicate: String` (rendered signature). Across `crates/bumbledb` and `crates/bumbledb-bench` (and any SDK crate re-exporting the symbol, in the same commit).

Do **not** rename: `FilterPredicate`, DNF/condition "predicate" English, SQL WHERE "predicate columns", C ABI / wire types / boundary `ir.rs`, locked error/config names. `AtomSource::Interior` is out of scope.

## Acceptance criteria

- [ ] Gone: `pub struct Predicate` / `pub struct PredicateColumn` / `fn predicate(` on the signature type and `PreparedQuery`. `rg -n 'struct Predicate\b|struct PredicateColumn\b|fn predicate\(' crates/bumbledb/src crates/bumbledb-bench/src` → no signature-type hits (`FilterPredicate` must still exist).
- [ ] Unchanged tests: pure rename — all suites green with zero assertion edits; ABI and boundary IR byte-identical.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb && cargo test -p bumbledb-bench`; `./scripts/check.sh`; `./scripts/lean.sh` (any Bridge census token citing the signature type moves in the same change — check `rg -n 'Predicate' lean/Bumbledb/Bridge.lean` and list non-signature leftovers in the commit).

## Constraints

- Rename-only; zero behavior change. Boundary/ABI untouched; locked names stay. Coordinate with docs issues that quote the old name (they cite this id).
