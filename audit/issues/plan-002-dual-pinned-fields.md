# plan-002: two `pinned_fields` functions; they disagree on sets

- **Severity:** medium
- **Tree:** plan
- **Status:** OPEN
- **Source:** audit/plan-exec.md F4
- **Depends on:** none (parallel-safe)

## The bug

`crates/bumbledb/src/plan.rs:13-35` documents "**the one** pinned-field vocabulary, shared by the distinctness witness and the DP's key-coverage translation so the two coverage predicates cannot diverge." It yields `FieldId` for Eq against scalar constants only — `ParamSet`/`WordSet` excluded (a set matches any element).

`plan/fj/provably_disjoint.rs:117-126` has a second `fn pinned_fields` that yields `(FieldId, &Const)` for **every** `Eq` compare, sets included, then relies on `provably_different` returning false for them.

## Why it's wrong

Two functions named for one fact, already diverging on sets (Insight 1: the dual *is* the flowchart). Distinctness says sets pin nothing; disjointness treats set Eqs as candidate pins. If `provably_different` ever grows a set case, disjointness fires on pins that distinctness refuses — a future bug the alias made easy (the same shape as engine-018's third floor name).

## The fix

Per `audit/CONTRACT.md` §C1 (one trusted vocabulary, not two):

- One function, the `plan.rs` filter, returning `(FieldId, &Const)` for scalar Eq only.
- `provably_distinct` / `densify` map to `FieldId`; `provably_disjoint` uses the pair.
- Delete the private copy in `provably_disjoint.rs`.

## Acceptance criteria

- [ ] Gone: exactly one `fn pinned_fields` under `crates/bumbledb/src/plan` (`rg -n 'fn pinned_fields' crates/bumbledb/src/plan` → one match, in `plan.rs`).
- [ ] Unchanged: distinctness and disjointness verdicts identical on the whole test corpus (`cargo test -p bumbledb` green, zero assertion edits).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Rename-and-dedup only. Sets still pin nothing. No change to the distinct-witness or disjoint-witness denotations.
