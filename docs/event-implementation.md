# Event implementation ledger

The complete revision 0.8 proposal is the target. This ledger records native
implementation evidence; it does not replace or reduce the M0–M8 acceptance
gates in `proposal/implementation-plan.md`. No release is authorized by this work.

Base: `d76d31ab` (public v1.3.1 plus removal of scratch allocation-count tests).
Implementation branch: `codex/event-algebra`.

| Gate | Status | Evidence / remaining work |
| --- | --- | --- |
| M0 contract baseline | Passed before native edits | `proposal/semantics/results/implementation-ready.json`: 142 documents, 246 Lean reports (160 axiom-free), seven semantic suites; readiness auditor rerun successfully before source changes |
| M1 owned canonical core | Core implemented; image bindings continue in M2 | Shared `bumbledb-event` crate; completed functions, essential tables, fallible construction, cardinality, legal-fibre saturation, checked binding registry, ownership and native WorkContext cancellation; exhaustive and dense-oracle checks pass |
| M2 structural type / persistence | In progress | [Version 1 unmeasured codec](event-value-format.md) implemented with cross-order/decoder tests and checked owner alignment; native values/macros, measured descriptors, compatibility and publication/lifetime gates remain |
| M3 dependencies / admission | Pending | Pointwise conflicts, union coverage, final-state writes, scalar constraints on empty facts, planner premises |
| M4 faces / maps / relations | Pending | Support/role/map certificates, shared witnesses, all modalities and residuals |
| M5 query / Pack / Free Join | Pending | Heads, staging, fault sets, grouping, certified factoring, spill and lifetime |
| M6 sources / inference | Pending | Rational and shared-parameter laws, exact semialgebraic capabilities, TypeSafe adapters and source intents |
| M7 information / fixed points | Pending | Observables, permissions, sealed monotone programs, termination/refinement evidence |
| M8 qualification | Pending | Full battery, packaged consumers, Coup application, malformed inputs, integrated performance and ARM64 evidence |
| Commit / push | Pending | Completed implementation and evidence committed and pushed; no version bump, tag or release |

The proposal readiness record pins the pre-implementation production source.
Changes to that source are expected now; do not rewrite historical evidence to
claim that the old isolation audit verifies the implemented engine.

## Core integration

`Value` belongs to the theory crate and must not depend on the storage engine.
The shared Event crate therefore owns values, canonical managers and mathematical
operations; the engine supplies cancellation through a small control interface.
Schema admission, query execution and source constructors consume that same core.
Resident keys are process-local handles. They never determine persistent fact
identity. The codec gate remains mandatory before Event facts are persisted.

## Current verification

- `cargo test -p bumbledb-event`: 15 tests, including exhaustive four-world
  supports/truth functions, dense symbolic oracles, projection, cardinality,
  adversarial physical orders, codec corruption, colliding fingerprints,
  cancellation/capacity, result lifetimes and opposite-direction alignment.
- `cargo test -p bumbledb --test event_core`: native WorkContext integration passes.
- `cargo test -p bumbledb-theory`: 39 unit, five integration and three doc tests pass.
- `cargo clippy -p bumbledb-event --all-targets -- -D warnings` and
  `cargo clippy -p bumbledb --all-targets -- -D warnings` pass.
- `cargo fmt --all -- --check` passes.

These verify the current core change, not completion of the remaining milestones.
The repository battery and integrated performance qualification remain M8 work.
The 2.8 GB local proposal laboratory remains preserved and is not blanket-added
to Git; curating the reproducible proposal/proof sources for the final push is
still required. No retained experimental artifact has been deleted or rewritten.
