# exec-009: `SelectionLevel.set: bool`, then `Colt` stores a parallel `set_levels: Vec<bool>`

- **Severity:** medium
- **Tree:** exec
- **Status:** OPEN
- **Source:** audit/plan-exec.md F12
- **Depends on:** none (COLT shape; `api/prepared/build.rs` is a writer of `SelectionLevel`)

## The bug

`crates/bumbledb/src/exec/colt.rs:290-293,324-337`:

```rust
pub struct SelectionLevel {
    pub columns: Vec<usize>,
    pub set: bool,  // point-probe vs set-union
}
// Colt:
set_levels: Vec<bool>,  // projected from selections.iter().map(|l| l.set)
selected: bool,         // "always true for selection-free tries"
```

`colt/new.rs:19` flattens `set` into `set_levels`. `colt/select.rs:32` branches `if self.set_levels[level]`. Two arrays that must stay aligned with `schema_columns`'s selection prefix. Selection-free tries pretend `select` already ran (`selected = selection_levels == 0`).

## Why it's wrong

Insight 6: `SelectionLevel` was the structured form; construction projected a bool strip and threw the rest of the alignment into a length invariant. Insight 4: `set: bool` is point vs set as a flag; `selected: bool` encodes vacuous success as a pretend-ran bit.

## The fix

Per `audit/CONTRACT.md` §C1:

```rust
enum SelectionLevel {
    Point { columns: Vec<usize> },
    Set { columns: Vec<usize> },
}
```

Colt stores the levels (or a per-level enum next to columns), not a bool strip. `select` matches Point vs Set. `selected` becomes `enum SelectState { Vacuous, Pending, Done }` — selection-free is Vacuous, not `true`.

## Acceptance criteria

- [ ] Gone: `rg -n 'set: bool' crates/bumbledb/src/exec/colt.rs` → no matches; `rg -nw 'set_levels' crates/bumbledb/src/exec` → no matches.
- [ ] Unchanged tests: `cargo test -p bumbledb --lib exec::colt` green; set-bound selection union still concatenates disjoint position lists.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Set-ness remains a plan fact (a `ParamId` is scalar or set, never both) — only the spelling changes. `build.rs`'s `SelectionLevel { columns, set }` construction becomes the enum. Probe inlining / `scripts/check-asm.sh` still the gate for the select path.
