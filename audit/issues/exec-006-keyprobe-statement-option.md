# exec-006: `KeyProbePlan.statement: Option` is U vs M as a hole

- **Severity:** medium
- **Tree:** exec
- **Status:** OPEN
- **Source:** audit/plan-exec.md F9
- **Depends on:** none (dispatch types; parallel-safe)

## The bug

`crates/bumbledb/src/exec/dispatch.rs:44-59`:

```rust
pub struct KeyProbePlan {
    pub statement: Option<StatementId>, // None = full-fact M; Some = U determinant
    pub key: Vec<(FieldId, Const)>,     // projection order for U, declaration order for M
    ...
}
```

`classify.rs:137-166` already computed which arm (`key_probe_candidate` returns `(Option<StatementId>, Vec<FieldId>)`) then stuffed it into the hole. Same vec, two meanings, distinguished by Option.

## Why it's wrong

Option-as-tag (Insight 4). U vs M are two access paths; the type admits `None` with a projection-shaped key and `Some` with a full-fact key. Classify parsed the kind and discarded it.

## The fix

Per `audit/CONTRACT.md` §C1 (trusted layer is a sum; C ABI essential-C is not this type):

```rust
enum KeyProbeKind {
    Uniqueness { statement: StatementId, key: Vec<(FieldId, Const)> },
    Membership { key: Vec<(FieldId, Const)> },
}
```

`key_probe_fact` / `execute_key_probe` match the kind. No `statement.is_some()`.

## Acceptance criteria

- [ ] Gone: `rg -n 'statement: Option<StatementId>' crates/bumbledb/src/exec/dispatch.rs` → no matches.
- [ ] Unchanged tests: `cargo test -p bumbledb --lib exec::dispatch` green; U and M key-probe fixtures still classify and execute identically.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Eligibility rules unchanged (single positive atom, no residuals, no measure, no ParamSet, not Interior, not closed). `classify` still returns `Option<KeyProbePlan>` meaning "this path vs Free Join" — that Option is the eligibility parse, not this issue.
