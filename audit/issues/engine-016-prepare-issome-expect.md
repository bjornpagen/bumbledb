# engine-016: prepare branches `if witness.rec().is_some()` and `expect`s the Option it just tested

- **Severity:** medium
- **Tree:** engine
- **Status:** OPEN
- **Source:** audit/engine.md F16
- **Depends on:** engine-005 (the witness sum is what prepare matches)

## The bug

`crates/bumbledb/src/api/prepared/build.rs:61-66`:

```rust
let rec = if witness.rec().is_some() {
    signatures.push(witness.rec().expect("rec present").predicate());
    Some(prepare_reach(txn, cache, schema, &witness, &signatures)?)
} else {
    None
};
```

and `prepare_reach` re-expects at `build.rs:371`:

```rust
let rec = witness.rec().expect("rec present");
```

## Why it's wrong

After validation, prepare is supposed to be a total function over a witness; instead it interrogates an `Option` twice on one line and once more in the callee — the validate-then-re-check move (Insight 6) one layer up from engine-005. Each `expect` is a runtime restatement of a fact the witness type should carry.

## The fix

Per `audit/CONTRACT.md §C3`: `prepare` matches engine-005's witness sum once:

```rust
match &witness {
    ValidatedQuery::Cq { .. }              => /* build Pipeline::Cq */,
    ValidatedQuery::Reach { rec, .. }      => /* prepare_reach(rec, ...), build Pipeline::Reach */,
}
```

`prepare_reach` takes `&ValidatedRec` (not the whole witness), so no `expect` is expressible. Signature sealing order (interiors then rec) stays as data flow, not as flag-guarded pushes.

## Acceptance criteria

- [ ] Gone: `rg -n 'rec\(\)\.is_some\(\)|expect\("rec present"\)' crates/bumbledb/src/api/prepared/build.rs` → no matches (also covered by engine-005's global grep).
- [ ] Unchanged tests: `cargo test -p bumbledb` green, zero assertion edits.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Lands with/after engine-005 and engine-001 (the two sums this match connects). Pure restructuring; behavior identical.
