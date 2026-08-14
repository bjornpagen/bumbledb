# engine-007: `DeltaVariant` — the k-variant machinery survives as a singleton wrapper

- **Severity:** high
- **Tree:** engine
- **Status:** FIXED(0eb89cae)
- **Source:** audit/engine.md F7
- **Depends on:** engine-002 (RecArm is the replacement type; co-lands)

## The bug

`crates/bumbledb/src/api/prepared.rs:341-357` — one rec arm, one delta occurrence, still wrapped as "the variant":

```rust
pub(crate) struct RecursiveRule { pub(super) variant: DeltaVariant }
pub(crate) struct DeltaVariant { pub(super) delta: OccId, pub(super) rule: FreeJoinRule }
```

Consumers unwrap through the ghost layer everywhere: `rule.variant.rule` (`reach.rs:322,504,603`; `introspect.rs:81,136`; `prepared.rs` accessors), `rule.variant.delta` (`reach.rs:420`; `introspect.rs:84`). Introspection still narrates the k-wide world — `introspect.rs:36-40`:

```rust
// A fixpoint program reports every predicate's plan units in
// predicate order — a recursive rule as its delta variants —
// ... the counted surface is the per-stratum round section
// (`stats.strata`), never per-unit node stats.
```

`stats.strata` does not exist (`ExecutionStats` has `reach: Option<ReachStats>`). `exec/introspection.rs:92-94` documents labels as `predicate p0 rule 1 delta variant 0`; the actual label is `reach rec {i} (delta occ {})` — the comment describes a deleted system.

## Why it's wrong

A newtype around a single value is the residue of k-variant minting (Insight 2: the deleted design's coordinate system outliving it). Every read pays an unwrapping step through a layer that encodes nothing, and the surrounding prose teaches maintainers the k-wide model — comments that cite a nonexistent field (`stats.strata`) are drift already realized (Insight 1).

## The fix

Per `audit/CONTRACT.md §C3`: `RecArm { delta: OccId, rule: FreeJoinRule }` (engine-002) replaces `RecursiveRule`+`DeltaVariant`; every `.variant.` path becomes a direct field read. Sweep the vocabulary:

- Delete the "delta variants" / "per-stratum" / "`stats.strata`" prose at `introspect.rs:36-40` and `exec/introspection.rs:87-94`; describe what exists: rec arms with one marked delta occurrence, counted surface `stats.reach.rounds`.
- `prepare_rule` stops being a wrapper that passes `delta: None` into `prepare_rule_variant`; the rec arm gets its own entry `prepare_rec_arm(..., delta: OccId) -> RecArm`. `delta: Option<OccId>` leaves every signature (absorbs engine.md F39).
- Introspection labels keep `reach rec {i} (delta occ {d})` — already variant-free; only the comments lie.
- `plan/selectivity.rs:89-94` comment "a delta-variant plan's marked occurrence" reworded to "a rec arm's marked delta occurrence".

## Acceptance criteria

- [x] Gone: `DeltaVariant`/`RecursiveRule`/`stats.strata`/`per-stratum` deleted; remaining `variant` uses name real Rust enum variants (Empty, sink, scalar/set). `prepare_rec_arm(..., delta: OccId) -> RecArm` is the rec entry; `delta: Option<OccId>` left the prepare signatures.
- [x] Unchanged tests: introspection snapshot tests pass unchanged (rendered text never contained "variant").
- [x] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb --lib api::prepared` (84 passed). Bridge census already cites `RecArm`.

## Constraints

- Semantics identical; one-delta-per-arm behavior byte-identical.
- Co-lands with engine-002. Bridge census green is part of done.
