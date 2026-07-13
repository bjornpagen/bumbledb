# PRD 08 — determinant: the guard vocabulary says what dependency theory says

**Depends on:** 07 (serialize the two giant sweeps). Runs SOLO.
**Modules:** `crates/bumbledb/src/storage/keys.rs` (57 hits:
`guard_key`, `parse_guard_key`, `guard_bytes`, `permuted_guard_bytes`,
`MAX_GUARD_WIDTH`), `storage/delta/guards.rs`, `verify_store/guards.rs`,
`storage/read/guard_row.rs`, `storage/commit/{judgment,plan,applier}.rs`
(43/35/18 hits), `exec/dispatch/{guard_probe_fact.rs,execute_guard.rs}`
+ `GuardPlan`/`GuardVar`, `api/prepared.rs` (`PreparedRule::Guard`,
`GuardRule`, `guard_finds`), `api/prepared/tests/guard.rs`, docs
(50-storage 22, 40-execution 14, 20-query-ir 10, 30-dependencies 10,
70-api 9, 60-validation 6, 10-data-model 5).
**Authority:** audit deep issue #7 + the census: "guard" carries four
meanings — the materialized FD index entry (dominant), its width cap,
the point-probe access path, and a prepared-rule variant — while
suggesting a runtime check or lock it is not. The materialized entry is
an INDEX on a key; say so.
**Representation move:** one real one rides the rename — raw guard
bytes become `DeterminantImage`.

## Context (decided shape) — the ledger

The one decision that makes this rename coherent: **the stored artifact
materializes the DETERMINANT — the left side X of the functionality
`R(X) -> R`, dependency theory's own word; the bytes of one entry are
a `DeterminantImage`; the execution path that reads it is a
`key_probe` (a point lookup by key — equally academic).** Three
derived names, one concept family, zero invented vocabulary.

- Storage: `guard_key(…)` → `determinant_key(…)` (the U-keyspace
  composer), `parse_guard_key` → `parse_determinant_key`, `guard_bytes` /
  `permuted_guard_bytes` → return `DeterminantImage`; `MAX_GUARD_WIDTH` →
  `MAX_DETERMINANT_WIDTH` (same value, same test); file
  `storage/delta/guards.rs` → `delta/determinants.rs`;
  `verify_store/guards.rs` → `verify_store/determinants.rs`;
  `storage/read/guard_row.rs` → `read/determinant_row.rs`.
- `DeterminantImage` newtype:

```rust
/// The encoded projection of one fact onto one key, in key order —
/// the bytes stored in (and probed against) the U determinant index. Width is
/// bounded by MAX_DETERMINANT_WIDTH at declaration; construction sites
/// are the two encoders in storage/keys.rs, nowhere else.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct DeterminantImage(/* SmallVec or Box<[u8]> per current repr */);
```

  Internal repr matches whatever the current byte carrier is (do not
  change allocation behavior — the guard scratch reuse in judgment.rs
  must survive as-is; if the scratch pattern resists the newtype
  without allocation churn, wrap at the API seams and record the seam
  list).
- Execution: `guard_probe_fact` → `key_probe_fact`; `GuardPlan` →
  `KeyProbePlan`; `GuardVar` → `KeyProbeVar`;
  `exec/dispatch/execute_guard.rs` → `execute_key_probe.rs`.
- Prepared: `PreparedRule::Guard(GuardRule)` →
  `PreparedRule::KeyProbe(KeyProbeRule)`; `guard_finds` →
  `key_probe_finds`; test file `tests/guard.rs` → `tests/key_probe.rs`.
- Error/display/EXPLAIN strings saying "guard" for this concept follow;
  goldens update (recorded churn).
- NOT renamed: prose "guard" in the generic Rust-idiom sense (RAII,
  let-else) — each surviving occurrence must read unambiguously as the
  idiom, not the domain term; the sweep rewrites any that don't.
- Storage FORMAT untouched: the `U` keyspace prefix byte and all
  on-disk bytes are identical — this is names, not encoding. The
  fingerprint and every storage pin stay byte-identical.

## Technical direction

Order: keys.rs core → storage consumers → exec dispatch → prepared →
tests → docs. The docs sweep rewrites "guard" to "determinant (the
materialized left side of the functionality)" on first use per chapter, then uses determinant
consistently. Run the store-level pins (verify_store suite, judgment
tests, `the_fingerprint_is_pinned`) after the storage step before
proceeding — a rename that moves bytes fails there, immediately.

## Passing criteria

- `[shape]` `grep -rni "guard" crates fuzz scripts` → only the
  documented Rust-idiom survivors (each listed in the commit body with
  its line); zero domain-sense hits. Docs: same battery over
  `docs/architecture/ docs/cookbook.md README.md`.
- `[shape]` `DeterminantImage` construction confined to storage/keys.rs (grep
  constructor count).
- `[test]` Full workspace suite green with unchanged assertion VALUES
  (mechanical re-anchors only); verify_store suite green; fingerprint
  pin byte-untouched.
- `[gate]` Bounded fuzz smoke (ops — it exercises commit judgment);
  clippy; fmt.

## Doc amendments (rule 6)

`50-storage.md`'s U-keyspace section retitled to the determinant index;
`30-dependencies.md` enforcement prose follows; glossary line in
`10-data-model.md`.
