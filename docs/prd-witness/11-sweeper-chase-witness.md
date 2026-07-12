# PRD 11 — Sweeper, chase, and surface consume the witness

**Depends on:** 10. **This PRD restores the tree** — it cuts every
remaining consumer of the dead `Resolved`/`Statement` shapes and closes
the 09→11 spine's gates.
**Modules:** `crates/bumbledb/src/verify_store/` (`facts.rs`,
`guards.rs`, `reverse.rs`, tests), `crates/bumbledb/src/plan/chase.rs` +
`plan/chase/evaluate.rs`, `crates/bumbledb/src/exec/dispatch/classify.rs`,
`crates/bumbledb/src/api/db/` (`get.rs` point reads, `plumbing.rs` if it
names statements), `crates/bumbledb/src/schema/render.rs`,
`crates/bumbledb/src/error/display.rs` (`display_with` statement
rendering), `crates/bumbledb/src/ir/render.rs` (the closed-refs table
construction), `crates/bumbledb/src/plan/selectivity.rs` (containment
bounds), remaining test suites.
**Authority:** PRD 09's decided shape; the audit's remaining site list:
`verify_store/guards.rs` (pointwise read), `facts.rs` (outgoing walk ×2
loops, extension sources), `reverse.rs` (the R-key mapping — already the
corruption convention, keep), `chase.rs` (`scalar_positions_only`,
containment reads), `evaluate.rs` (containment-into-id inference),
`classify.rs` (key lookup), `get.rs` (`NotAKeyStatement` dynamic check).
**Representation move:** same as 10, applied to the read-side consumers;
plus the one *dynamic-surface* mapping this layer owns gets its honest
type: a host-supplied `StatementId` parses into a `KeyId` once, at the
boundary, and the point-read path is total thereafter.

## Context (decided shape)

- **`verify_store/facts.rs`**: the F→U loop iterates
  `relation.keys(): &[KeyId]` (guard re-derivation via
  `schema.key(kid)`; the `key_projection` call and its panic doc die).
  `check_outgoing` iterates typed `ContainmentId`s and matches
  `enforcement` — the three-arm match with the Functionality
  unreachable becomes two arms. `check_extension_sources` likewise; its
  `debug_assert!(!coverage)` stays (it asserts a VALIDATE-refused shape,
  not variant agreement — the honest keep). `Sweep.selections` indexes
  by `ContainmentId` (mirrors PRD 10's `Selections` change).
- **`verify_store/guards.rs`**: the U-pass parses `StatementId` from
  stored bytes and maps through `schema.statement(sid)` — a non-key is
  already a `Malformed` finding path there; the surviving destructure
  reads `schema.key(kid).pointwise` directly.
- **`verify_store/reverse.rs`**: already the corruption convention
  (stored R key → statement → non-Probe = `ClosedRelationEntry`
  finding); re-anchor to `StatementRef`/`Enforcement` with the SAME
  finding semantics — the `else` arm is a finding, never unreachable.
- **`plan/chase.rs`**: `scalar_positions_only(resolved)` dies; condition
  4 reads `enforcement` via the typed accessor
  (`matches!(c.enforcement, Enforcement::Probe { coverage: false, .. })`);
  the eliminate-path's descriptor destructures become field reads on
  `ContainmentStatement` (`source`/`target` are direct fields now — the
  `StatementDescriptor::Containment` matches die).
  **`chase/evaluate.rs`**: `domain_within_ids`' containment-into-id
  inference walks `schema.containments()` directly (its
  descriptor+resolved double-destructure dies).
- **`exec/dispatch/classify.rs`**: key candidate enumeration via
  `relation.keys()` → `schema.key(kid)` — total; the WordSet/ParamSet
  refusal arms are untouched (different concern).
- **`api/db/get.rs`** (+ `get_dyn`): the typed path takes the macro's
  emitted `StatementId` and maps once at entry:
  `schema.statement(sid)` → `Key(kid)` or `FactShapeError::NotAKeyStatement`
  (the dynamic surface's existing typed error — unchanged semantics, now
  the single parse point); everything downstream carries `KeyId`.
- **Render/display**: `schema/render.rs` renders statements from the
  sealed sum (sides are direct fields); the REJECTED-declaration
  diagnostic path (`display_with` over `SchemaDescriptor`) is untouched
  — it renders descriptors, which still exist. `ir/render.rs`'s
  closed-refs table builds from `schema.containments()` +
  `Enforcement::Closed` — its containment walk simplifies.
- **`plan/selectivity.rs`**: the containment-domain rung reads
  enforcement through the typed accessor; its destructure dies.

## Technical direction

Compiler-driven, module by module in the order above; then the test
sweep: verify_store tests, chase/evaluate tests, dispatch tests, prepared
tests, integration tests (`schema_macro.rs` asserts materialized
statements — descriptor-level, expected untouched; `api.rs` point reads).
Close the spine's deferred gates: the fingerprint pins (bench constant
unchanged), the full engine suite, trace suite, and the deleted-assert
reconciliation — the commit body lists every deleted
`unreachable!`/`expect` across 09–11 against the audit's count of 19 for
this family, explaining any delta.

## Passing criteria

- `[shape]` `grep -rn "Resolved::" crates` → zero hits;
  `grep -rn "validated schema:" crates/bumbledb/src` → zero hits (the
  phrase was the family's signature).
- `[shape]` `grep -c "unreachable!" crates/bumbledb/src -r` is reduced
  by ≥19 from the audit baseline (201); the commit body reconciles the
  exact ledger.
- `[shape]` `verify_store/reverse.rs` and `guards.rs` convict stored-byte
  mismatches as findings/corruption — zero programmer-invariant panics on
  stored input anywhere in the sweeper.
- `[test]` Full engine suite green (`cargo test -p bumbledb`, all
  targets, plus `--features trace`): every re-anchored test keeps its
  original verdict/finding assertions.
- `[shape]` The bench fingerprint pin (`63e3b480…`) and the corpus digest
  are UNTOUCHED by the whole 09–11 spine (`git diff` clean on
  `bumbledb-bench/src/schema.rs`'s pin test and `bench-data` keys).
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

None beyond PRD 09's (already written when the spine closes) — verify
30-dependencies' new witness paragraph names the accessors that actually
shipped.
