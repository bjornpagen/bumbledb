# PRD 10 — The commit pipeline consumes the witness

**Depends on:** 09 (the types exist; the tree is red until 11).
**Modules:** `crates/bumbledb/src/storage/commit/plan.rs`,
`storage/commit/judgment.rs`, `storage/commit/applier.rs` (touch
expected nil — verify), `storage/keys.rs`
(`debug_assert_ordinary` only if it reads statements),
`storage/delta/` (only if it reads `Relation.keys` — verify),
`storage/commit/tests/*`.
**Authority:** PRD 09's decided shape; the audit's site list for this
layer: `plan.rs` (guard derivation, edge/membership split, target
checks, `key_relation`), `judgment.rs` (`Selections::containment`'s
expect, `check_source`'s membership lookup, `check_target`'s
debug_assert + `containment_source` + `closed_source_survivor` + the
affected-walk destructures, `establishing_fact`'s callers).
**Representation move:** every `let Resolved::… else unreachable!` and
every "validated schema: X ids name Y statements" expect in the commit
pipeline becomes a total match or a direct typed accessor call.

## Context (decided shape)

- **`plan.rs`**: `fact_op`'s guard loop iterates
  `relation.keys(): &[KeyId]` → `schema.key(kid)` — `pointwise` read
  directly, `guard_bytes` from `key.projection`; the
  `Resolved::Functionality` destructure dies. The edges/memberships loop
  iterates `relation.outgoing(): &[ContainmentId]` →
  `schema.containment(cid)`, matches `enforcement`:
  `Probe { target_key, key_permutation, coverage }` → `EdgeOp`;
  `Closed { .. }` → `MembershipOp` (referencing word decode unchanged);
  no third arm exists to refuse. `EdgeOp.target_key` becomes `KeyId`
  (the guard probe's typed target); `EdgeOp.statement` /
  `MembershipOp.statement` stay `StatementId` (error payloads, `R`
  keys). `GuardOp.statement` stays `StatementId` (`U` keys embed it —
  storage layout is untouchable), sourced as `schema.key(kid).id`.
- **`target_checks`**: `deleted_guards`/`inserted_guards` key on
  `KeyId`; `GuardCheck.key: KeyId`; `GuardCheck.relation` dies —
  `schema.key(kid).relation` answers it (the `key_relation` helper and
  its unreachable die). `DependentCheck.statement` splits into what its
  consumers need: `cid: ContainmentId` (for `Selections` and the typed
  reads) — the violation payload takes
  `schema.containment(cid).id`. `dependents(kid)` is already typed; the
  dependents-loop destructure and the PRD-04-era debug_assert die.
- **`judgment.rs`**: `Selections.checks` becomes
  `Box<[SideChecks]>` indexed by `ContainmentId` densely — the `Option`
  and the `containment()` expect die; `encode_with` walks
  `schema.containments()` directly. `check_source`'s membership arm
  reads `Enforcement::Closed`'s members via
  `schema.containment(cid).enforcement` — the plan can now carry `cid`
  on `MembershipOp` (add it beside the payload `StatementId`, or carry
  only `cid` and derive the id at error construction — choose the
  latter: one field, id derived where the error is built).
  `containment_source` and `closed_source_survivor` take
  `ContainmentId` and become field reads — their unreachables die.
  `check_target`'s affected-walk parses `R` keys to `StatementId`
  (storage bytes — stays), then maps to `ContainmentId` through
  `schema.statement(sid)`: this is the ONE place a stored id meets the
  arena, and a non-containment there is CORRUPTION (stored bytes lie),
  not a programmer invariant — it becomes a
  `CorruptionError::MalformedValue("R key statement")` path, mirroring
  `verify_store/reverse.rs`'s convention. `Probe.target_key: KeyId`.
  `establishing_fact` takes `KeyId`.
- **`applier.rs`** consumes only plan-derived byte material and
  `StatementId`s — expected zero changes; verify and record.
- **`keys.rs`**: signatures keep `StatementId` (storage layout);
  `debug_assert_ordinary` unchanged unless it reads statements (it reads
  relations — verify).

## Technical direction

Work top-down `plan.rs` → `judgment.rs` → tests; the compiler is the
worklist once 09 landed. Every deleted `unreachable!`/`expect` in this
layer is counted and named in the commit body (the audit counted:
plan.rs ×5, judgment.rs ×6 — the commit body reconciles the actual
count). Commit tests re-anchor: the fixtures' `Resolved::…` assertions
(e.g. `closed.rs`'s `the_domain_statement_resolved_the_handler_key`)
become `Enforcement`/witness assertions; `sealed_checks.rs`'s
`checks.as_ref().expect("containment")` chains become direct field
reads. No test's VERDICT assertions change — same ops, same typed
errors, same statement ids.

## Passing criteria

- `[shape]` `grep -rn "unreachable!\|\.expect(" crates/bumbledb/src/storage/commit/plan.rs
  crates/bumbledb/src/storage/commit/judgment.rs` → zero
  variant-agreement asserts (remaining hits only: fixed-width slice
  expects and the corruption conversion of the R-key mapping).
- `[shape]` `Selections` carries no `Option`; `grep -n "validated schema:"
  crates/bumbledb/src/storage/commit` → zero hits.
- `[shape]` `GuardOp`/`U`-key and `R`-key byte layouts byte-identical
  (the key-layout golden tests in `storage/keys.rs` unmodified and
  green at PRD 11's close).
- `[test]` The commit test suites (`apply`, `commit`, `functionality`,
  `judgment`, `plan`, `target`, `closed`, `sealed_checks`) green in
  re-anchored form with unchanged verdict assertions.
- `[gate]` Workspace gates green at campaign close (post-11).

## Doc amendments (rule 5)

`50-storage.md` § write path: the plan/judgment prose swaps "resolved
enforcement data" vocabulary for the witness accessors (two sentences);
key layout section states explicitly that storage keys embed
`StatementId` and never arena ids.
