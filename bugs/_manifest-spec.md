# Spec-vs-docs-vs-Rust audit manifest (ids 200–225)

Verified 2026-08-12 against Lean, normative docs, and Rust. All 26 survivors confirmed. None deleted; none newly filed.

| file | summary | wrong-side | severity | status |
|---|---|---|---|---|
| 200-c20-ray-weight-absent-parent.md | C20 refuses a ray Duration child under an absent parent; Lean `capacity_of_empty_parent` and architecture docs treat that insert as a no-op | split | high | confirmed |
| 201-argkey-measure-missing-from-lean.md | `ArgKey::Measure` is in Rust/docs/R5; Lean `AggOp.argMax` is VarId-only; conformance fences the shape | spec | high | confirmed |
| 202-cookbook-claims-disjoint-dedup-elision.md | Cookbook recipe 22 (and TS twin) claims executor elides cross-rule dedup; Lean/40-execution/Rust keep a spanning seen-set | docs | high | confirmed |
| 203-bridge-abort-fresh-discarded.md | Bridge premise says aborted mint runs are discarded; `Fresh.lean` and the engine persist the high-water | spec | medium | confirmed |
| 204-abort-never-touched-disk.md | README/70-api claim abort never touched LMDB; abort burn writes `Q` marks | docs | medium | confirmed |
| 205-dnf-fold-cites-projection-theorem.md | DNF “fold-preserving” cites `dnf_preserves_denotation` (projection); fold law is `dnf_rekey_transparent` | docs | medium | confirmed |
| 206-fixpoint-budget-incompleteness.md | Engine `FixpointBudgetExceeded` is incomplete vs Lean `evalProgram` / `program_eval_sound` | rust | medium | confirmed |
| 207-closed-target-key-broader-in-lean.md | `TargetKeyAccepted` is any matching FD; Rust closed targets require synthetic `FieldId(0)` | spec | medium | confirmed |
| 208-closed-containment-interval-unmodeled.md | Closed+interval containment is a Lean judgment; engine `ClosedContainmentInterval` refuses v0 | spec | medium | confirmed |
| 209-fixedbytes-word-vs-byte-encoding.md | Lean `bytes<N>` is N Words; Rust/docs store N bytes padded to ⌈N/8⌉×8 | split | medium | confirmed |
| 210-measure-of-ray-not-the-only-runtime-error.md | 70-api omits `MeasureOfRay`; “one runtime type error” slogan hides write-path `CapacityRayMeasure` | docs | medium | rewritten |
| 211-ts-argkey-measure-missing.md | TS `argMax` keys are variables only; Rust/C++/docs admit `Duration` keys | split | medium | rewritten |
| 212-commitrejected-all-containment-comment.md | `CommitRejected` comment says all-containment; statement phase mixes capacity citations | rust | low | confirmed |
| 213-multi-interval-fd-lean-scalar-default.md | Two interval fields → Lean scalar `Functionality`; Rust `FunctionalityMultipleIntervals` | spec | medium | confirmed |
| 214-conformance-fences-shipped-shapes.md | Third oracle excludes negated membership, set membership, measure Arg — shipped elsewhere | unspecified | medium | confirmed |
| 215-functionality-interval-not-last.md | Non-final interval FD is pointwise in Lean; Rust `FunctionalityIntervalNotLast` | spec | low | confirmed |
| 216-readme-omits-fixed-width-interval.md | README type table has no `interval<E,w>` row | docs | low | confirmed |
| 217-closed-roster-cap-unmodeled.md | Engine/docs cap closed axioms at 256; Lean `GroundExtension` is unbounded | spec | low | confirmed |
| 218-api-roster-omits-capacity-ray-measure.md | 70-api write errors omit `CapacityRayMeasure` | docs | medium | confirmed |
| 219-hash-equality-vs-canonical-bytes.md | Lean identity is canonical bytes; store membership is blake3 with collision axiom | split | medium | confirmed |
| 220-capacity-ray-junk-zero.md | Lean `durationNat` of a ray is 0; engine `CapacityRayMeasure` (undefined, not false) | spec | medium | confirmed |
| 221-negated-complement-fold-unmodeled.md | Grounding overview cites Lean theorems for evaluation; Lean leaves `fold_negated` unmodeled | docs | medium | confirmed |
| 222-bulk-load-chunking-vs-scanload.md | Lean `scanLoad` is one judgment; `bulk_load` is 4096-fact commit sequence | unspecified | low | confirmed |
| 223-schema-fingerprint-unmodeled.md | Open identity is blake3 v5; Lean `Theory` has no fingerprint | unspecified | low | confirmed |
| 224-membership-lowering-excludes-negated.md | Named `membership_lowering_preserves` requires membership-free negation; engine runs `AntiProbe` | spec | medium | rewritten |
| 225-origin-and-result-bytes-overflow.md | `OriginCapacity` / `ResultBytesOverflow` abort queries Lean still denotes; 70-api/40-execution understate | split | low | rewritten |

## Counts

**By severity:** high 3, medium 16, low 7, critical 0, info 0. **Total 26.**

**By wrong-side:** spec 9, rust 2, docs 7, split 5, unspecified 3.

**By verification status:** confirmed 22, rewritten (still confirmed, narrower) 4, deleted 0, new 0.

Wrong-side corrections vs original filing: 217 unspecified→spec; 225 unspecified→split.

## Deleted

None. No `bugs/_rebuttals-spec.md`.

## New

None (40-execution “resource limits: none except fixpoint” folded into 225).
