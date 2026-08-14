# engine-030: dead `normalize()` encodes the false "witness has no Interior occurrences" invariant

- **Severity:** medium
- **Tree:** engine
- **Status:** FIXED(1af537e5)
- **Source:** audit/engine.md F30
- **Depends on:** none (deletion; parallel-safe)

## The bug

`crates/bumbledb/src/ir/normalize/normalize.rs:14-28`:

```rust
/// ... The query path: no `Interior`
/// occurrence exists in a sealed [`ValidatedQuery`] (the query boundary
/// has no predicate address space), so the signature surface is empty.
#[must_use]
#[allow(dead_code)]
pub fn normalize(schema: &Schema, query: &ValidatedQuery) -> Vec<NormalizedQuery> {
    normalize_rules(schema, &[], query.rules())
}
```

The claim is false (interiors/rec/main all carry Interior occurrences; production routes through `normalize_rules`/`normalize_predicate` with signatures). `plan/fj/validate.rs:198-200,216-217` repeats the same false sentence around the `#[cfg(test)]` `validate` entry that passes `&[]`.

## Why it's wrong

A dead function is a load-bearing lie waiting for a caller: it compiles, it's `pub`, its doc states an invariant that stopped being true when the cut landed, and `#[allow(dead_code)]` is the hush flag keeping it alive (Insight 15: unspent code has negative value; Insight 1: the doc trains readers on a deleted model).

## The fix

- DELETE `normalize()` and its `#[allow(dead_code)]`. Production keeps `normalize_rules`/`normalize_predicate` (signatures always).
- `plan/fj/validate.rs`: the `#[cfg(test)]` `validate` entry either takes signatures like production (empty slice passed BY ITS CALLERS as fixture data — fine) or keeps the convenience wrapper with an honest doc ("test convenience: EDB-only fixtures pass no derived signatures"); the "a sealed ValidatedQuery carries no Interior occurrence" sentence deletes in both places (also tracked by engine-011).

## Acceptance criteria

- [ ] Gone: `rg -n 'fn normalize\(' crates/bumbledb/src/ir/normalize/normalize.rs` → no matches; `rg -n 'allow\(dead_code\)' crates/bumbledb/src/ir/normalize` → no matches; the false sentence gone from both files (engine-011's grep).
- [ ] Unchanged tests: `cargo test -p bumbledb` green (test-only `fj::validate` callers updated mechanically).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Pure deletion + doc honesty; zero behavior change.
