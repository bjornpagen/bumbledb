# PRD-U2 — The engine kills

Wave 1 · Repo: bumbledb (`crates/`) · depends on: U1 in shared files
(`storage/env/*`) — coordinate, don't collide · executes ruling 2 (the kill
class, blanket-approved) and records ruling 4's gravestone

## Objective

Execute every engine-census KILL. Each is a representation collapse: two
spellings of one meaning become one, a guard whose case the loop already
handles dies, a duplicated body gets one home. Nothing here changes shipped
behavior; the referees are the existing pins.

## Work — the ten kills (census `cleanup-engine.md`, sites re-verified here)

1. **Trace-counter call-site cfg duals → a `not(trace)` ZST counters facade.**
   Restores obs.rs's own law ("call sites never write `#[cfg]`"). Sites: dual
   `retain` arms in `image/cache/advance.rs` and `evict_older_than.rs`, the
   `cfg_attr(unused_self)` blocks in `get_or_build.rs`, the scattered
   `fetch_add` cfgs (~15 sites). Shipped binary unchanged (inline empty
   bodies off).
2. **`api/prepared/execute.rs` timing dual → the same ZST facade.**
3. **Bench `obs` cfg'd struct fields + paired report arms → always-present
   `Option` fields** (`None` when off): `harness/measure.rs`,
   `driver/bench.rs`, `closure.rs`, `displaced.rs`, `read_family.rs` (~10 cfg
   sites), zero runtime cost.
4. **Violation message bodies rendered twice → one `violation_body` helper**
   called by both `Display for Violation` and `display_with`
   (`error/display.rs`).
5. **Format-version check triplicated → one `check_format_version`** beside
   `read_u32` (`env/open.rs`, `env/ephemeral.rs`, `env/exhume.rs`). The
   doc-lawed check ORDER is unaffected — pin it if no test already does.
   (Lands after U1's edits in these files.)
6. **`scan` ≡ `scan_from(rel, 0)` → delegate** (`storage/read/scan.rs`); pin
   prefix_iter↔range equivalence with one test.
   **DEFERRED-TO-RECONCILIATION (recorded 2026-07-19, the bug-bash pass):
   impossible on this fork.** `scan_from` is PR #10's copy-on-append read
   path and this branch forked from main BEFORE that merge — no
   `storage/read/scan_from` target exists here (grep zero;
   `b0ddb330` touches no `storage/read/scan.rs`).
   **EXECUTED ON THE RECONCILED TREE (2026-07-19, the reconciliation
   pass): ABORTED-WITH-REASON — the census's ≡ is false at the real
   site.** The delegation was attempted against the merged code and is
   refuted by an existing audit pin: `read/tests.rs:
   a_short_f_key_is_typed_corruption_from_scan` plants a bare 5-byte
   `F | rel` prefix key and requires `scan` to convict it, but a proper
   prefix sorts strictly BEFORE `fact_key(rel, 0)` in LMDB byte order,
   so `scan_from(rel, 0)`'s `Included(fact_key(rel, 0))` range cursor
   skips it silently — delegating would weaken that pin (forbidden).
   The two cursor-opens (prefix_iter vs range) encode DIFFERENT
   corruption envelopes: two meanings, not two spellings; the genuinely
   shared meaning — the per-entry parse, width check, and error fuse —
   already has one home (`parse_facts`). The limit clause governs:
   aborted, recorded here and at the site (`scan`'s doc). The honest
   half of the sketched pin landed instead:
   `scan_from_zero_yields_exactly_scan_over_live_facts` pins the
   row-level agreement over well-formed keys, from zero and from a mid
   cut.
7. **`TransientImage::refill` ≡ append-from-0 with a capacity-policy
   parameter** (`image/build.rs`; call sites in `api/prepared/fixpoint.rs`).
8. **Third copy of the probe hash → `swar::hash_words` widened to
   `pub(crate)`** and called from `image/cardinality.rs` (the file swar.rs
   was created to prevent).
9. **`apply_infallible` survivors-only early return → delete the
   `predicates.len() == 1` guard** (`image/view/apply.rs`) — the loop is
   already a semantic no-op there; the pivot mechanism stays.
10. **Bench SQL builder single-test parenthesization → always emit parens**
    (`bumbledb-bench/src/translate/builder.rs`), SQLite-identical.

## Work — the gravestone (ruling 4)

`lineage-off` never merged into this tree; the ruling is recorded so the merge
of PR #10 cannot resurrect it. Add the gravestone where the house keeps them
(the bench manifest's PRD-C1 gravestone block, `crates/bumbledb-bench/Cargo.toml`):
measurement twins die once the number is banked; the cold-lineage knob and the
Wave-M A/B twin are ruled dead — if PR #10 merges carrying them, they are
deleted in the same commit.

## Explicitly NOT in scope

- The measure-or-merge trio (leaf elision, all-words finalize,
  permuted-identity determinant) — M owns them; touching them here violates
  ruling 6–8's protocol.
  **Amendment (recorded deviation, 2026-07-19):** the U2 commit
  (`b0ddb330`) prepared the trio's A/B scaffolding early —
  `Executor::disable_leaf_elision`/`leaf_elision_engaged` (`cfg(test)`
  only), `api/prepared/tests/measure_twins.rs`, and the ignored
  `permuted_identity_determinant_twin` in `storage/keys.rs`. The fast
  paths themselves are unmodified and no runtime mode exists; M still
  owns the measurement and the verdict, and the switches die with it
  (prd-M's "never ships" clause now reads: never ships PAST the
  verdict). The scope wall stands for everything else.
- Every KEEP-AS-LAW item, including the kind system (U1's remit), the
  closed-relation carve-outs, the sanctioned-unsafe kernels, all platform
  branches, and the copy-on-append fork.
- The limit clause governs each kill: if the collapse turns out to add more
  machinery than it removes at the real site (census sketches are sketches),
  abort that kill and record why in the PR — do not force it.

## Passing criteria

- All ten kills landed or individually aborted-with-reason; no half-kills.
- `grep -rn "#\[cfg" crates/bumbledb/src/image/cache crates/bumbledb/src/api/prepared/execute.rs`
  shows no trace-conditional call sites (the facade owns the fork).
- `scripts/check.sh` green including the full feature matrix (the trace lanes
  prove the facade's on/off twin behavior); `scripts/lean.sh` green.
- No pin weakened; the two new/updated pins (scan delegation equivalence,
  format-check order if newly pinned) green.
- The ruling-4 gravestone exists and names this packet.
- `scripts/spec-census.sh` green (renamed helpers sweep their citations).
