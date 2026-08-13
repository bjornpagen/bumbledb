# Spec-vs-docs-vs-Rust audit manifest (ids 200–225)

Findings are one divergence each. Severity and wrong-side as filed.

| file | summary | wrong-side | severity |
|---|---|---|---|
| 200-c20-ray-weight-absent-parent.md | C20 refuses a ray Duration child under an absent parent; Lean `capacity_of_empty_parent` and architecture docs treat that insert as a no-op | split | high |
| 201-argkey-measure-missing-from-lean.md | `ArgKey::Measure` is in Rust/docs/R5; Lean `AggOp.argMax` is VarId-only; conformance fences the shape | spec | high |
| 202-cookbook-claims-disjoint-dedup-elision.md | Cookbook recipe 22 (and TS twin) claims executor elides cross-rule dedup; Lean/40-execution/Rust keep a spanning seen-set | docs | high |
| 203-bridge-abort-fresh-discarded.md | Bridge premise says aborted mint runs are discarded; `Fresh.lean` and the engine persist the high-water | spec | medium |
| 204-abort-never-touched-disk.md | README/70-api claim abort never touched LMDB; abort burn writes `Q` marks | docs | medium |
| 205-dnf-fold-cites-projection-theorem.md | DNF “fold-preserving” cites `dnf_preserves_denotation` (projection); fold law is `dnf_rekey_transparent` | docs | medium |
| 206-fixpoint-budget-incompleteness.md | Engine `FixpointBudgetExceeded` is incomplete vs Lean `evalProgram` / `program_eval_sound` | rust | medium |
| 207-closed-target-key-broader-in-lean.md | `TargetKeyAccepted` is any matching FD; Rust closed targets require synthetic `FieldId(0)` | spec | medium |
| 208-closed-containment-interval-unmodeled.md | Closed+interval containment is a Lean judgment; engine `ClosedContainmentInterval` refuses v0 | spec | medium |
| 209-fixedbytes-word-vs-byte-encoding.md | Lean `bytes<N>` is N Words; Rust/docs store N bytes padded to ⌈N/8⌉×8 | split | medium |
| 210-measure-of-ray-not-the-only-runtime-error.md | Docs call `MeasureOfRay` the one runtime type error; 70-api omits it and other query aborts exist | docs | medium |
| 211-ts-argkey-measure-missing.md | TS `argMax` keys are variables only; Rust/C++/docs admit `Duration` keys | split | medium |
| 212-commitrejected-all-containment-comment.md | `CommitRejected` comment says all-containment; statement phase mixes capacity citations | rust | low |
| 213-multi-interval-fd-lean-scalar-default.md | Two interval fields → Lean scalar `Functionality`; Rust `FunctionalityMultipleIntervals` | spec | medium |
| 214-conformance-fences-shipped-shapes.md | Third oracle excludes negated membership, set membership, measure Arg — shipped elsewhere | unspecified | medium |
| 215-functionality-interval-not-last.md | Non-final interval FD is pointwise in Lean; Rust `FunctionalityIntervalNotLast` | spec | low |
| 216-readme-omits-fixed-width-interval.md | README type table has no `interval<E,w>` row | docs | low |
| 217-closed-roster-cap-unmodeled.md | Engine/docs cap closed axioms at 256; Lean `GroundExtension` is unbounded | unspecified | low |
| 218-api-roster-omits-capacity-ray-measure.md | 70-api write errors omit `CapacityRayMeasure` | docs | medium |
| 219-hash-equality-vs-canonical-bytes.md | Lean identity is canonical bytes; store membership is blake3 with collision axiom | split | medium |
| 220-capacity-ray-junk-zero.md | Lean `durationNat` of a ray is 0; engine `CapacityRayMeasure` (undefined, not false) | spec | medium |
| 221-negated-complement-fold-unmodeled.md | Docs cite Lean grounding theorems for `fold_negated`; Lean leaves the complement fold unmodeled | docs | medium |
| 222-bulk-load-chunking-vs-scanload.md | Lean `scanLoad` is one judgment; `bulk_load` is 4096-fact commit sequence | unspecified | low |
| 223-schema-fingerprint-unmodeled.md | Open identity is blake3 v5; Lean `Theory` has no fingerprint | unspecified | low |
| 224-membership-lowering-excludes-negated.md | Bridge-cited `membership_lowering_preserves` requires membership-free negation; engine runs `AntiProbe` | spec | medium |
| 225-origin-and-result-bytes-overflow.md | `OriginCapacity` / `ResultBytesOverflow` abort queries Lean still denotes | unspecified | low |

## Counts

**By severity:** high 3, medium 16, low 7, critical 0, info 0. **Total 26.**

**By wrong-side:** spec 8, rust 2, docs 7, split 4, unspecified 5.
