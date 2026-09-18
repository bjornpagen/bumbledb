# Event implementation ledger

The complete revision 0.9 proposal is the target. This ledger records native
implementation evidence; it does not replace or reduce the M0–M8 acceptance
gates in `proposal/implementation-plan.md`. No release is authorized by this work.

Base: `d76d31ab` (public v1.3.1 plus removal of scratch allocation-count tests).
Implementation branch: `codex/event-algebra`. Owned core commit: `fca4ac667`.

| Gate | Status | Evidence / remaining work |
| --- | --- | --- |
| M0 contract baseline | Passed before native edits; current proposal audited separately | Historical revision 0.8 readiness record is preserved; revision 0.9 adds native identity and bounded fixed-point semantics |
| M1 owned canonical core | Implemented | Shared `bumbledb-event` crate; completed functions, essential tables, fallible construction, legal counts, saturation, checked alignment/registry, ownership and native cancellation |
| M2 structural type / persistence | Native unmeasured slice implemented; full gate open | Event values/macros, canonical rows, image words, parameters/sets/literals, Free Join equality, spill and owned results; raw Node wire uses checked BEVT bytes. Complete SDK authoring and future source/face descriptor qualification remain |
| M3 dependencies / admission | Pending; explicit safety gate | Event dependency projections, selections and closed rosters refuse `EventContractPending`; ordinary scalar keys with Event payloads work. Replace the guard with pointwise admission, not scalar-key semantics |
| M4 faces / maps / relations | Pending | Support/role/map certificates, shared witnesses, all modalities and residuals |
| M5 query / Pack / Free Join | Pending | Constructive heads, staging, stable participating fault sets, grouping and certified factoring; ordinary Event binding/equality already works |
| M6 sources / inference | Pending | Rational and shared-parameter laws, exact semialgebraic capabilities, TypeSafe adapters and source intents |
| M7 information / fixed points | Pending native implementation; bounded reference proved | Observables, permissions, sealed monotone programs, finite-presentation admission and refinement to the proved iteration |
| M8 qualification | Pending | Full battery, packaged Rust/TypeScript consumers, executable Coup fixtures, malformed descriptors, integrated performance and ARM64 evidence |
| Final commit / push | Pending | Complete implementation and curated evidence committed and pushed; no version bump, tag or release |

## Core and equality

`Value` belongs to the theory crate and must not depend on the storage engine.
The shared Event crate owns values, canonical managers and mathematical
operations; the engine supplies cancellation through `Control`/`WorkContext`.

An owned Event retains its manager. Rust `Eq`/`Hash` is scoped handle identity.
Independent owners require checked `align_to`; it is not an extensional global
comparison. Canonical persistent bytes identify the named source, original support
and supported membership. A query registry aligns equivalent decoded values into
one namespace, so ordinary word equality implements Event equality there. Equal
counts or probabilities do not identify Events. Public order comparisons refuse.

## Native storage and ownership

[BEVT v1](event-value-format.md) is embedded as a length-prefixed Event field in
canonical rows. The field occupies two ordinary resident words, not an interval
endpoint pair. Whole-fact deduplication uses canonical bytes, never resident keys.
Generated facts containing Event are owned and `Clone`; they are not `Copy`.
Event host-newtype syntax is not implemented.

Each typed or dynamic decoded row aligns its own Event fields through a lazy
registry. Independent rows remain independently owned and require explicit
alignment for standalone algebra. Query outputs use the execution registry and
own their returned Events, including pages and spill results after the snapshot,
executor and database are dropped. Missing/stale/unregistered keys refuse.
Database decode paths with a WorkContext retain cancellation; manual
`RowReader::new` and the legacy no-work scan use the uncancellable core control.

Resolver generations reclaim unreferenced text independently. **All registered
Events and canonical descriptors remain strongly retained until the generation
is released.** The default registry limit is 2,000,000 distinct Events; that is
an entry bound, not a byte budget. Images and executions pin their generation.
Cache rotation cannot invalidate retained results. Memory-pressure qualification
and a complete retained-byte policy remain acceptance work; eviction alone does
not reclaim Event history in a live generation.

The raw Node bridge transports canonical BEVT `Uint8Array` values, tagged `event`
where a tag is required. The worker serializes returned Events under its work
context; malformed input refuses in the shared core decoder. This is the native
wire foundation. The high-level TypeScript field/value/operation API and generated
SDK bindings are still pending; bindings generation refuses unsupported Event
schemas explicitly. The log JSON value spelling is `{"event":"<BEVT hex>"}`.
Command result records retain their existing scalar-only contract.

## Lean evidence and remaining correspondence

The proposal suite was rerun: **246 reports / 30 files, 160 axiom-free**.
The [native semantic run](event-evidence/fixed-point-boundaries/check.json) adds
**20 reports / two files**, using only permitted standard Lean axioms.
Persistence proves support-relative identity and the registry-key law under
explicit admission/roundtrip premises. FixedPoint proves bounded stabilization,
least/greatest extremality, uniform shared environments and counterexamples for
negation and environment mixing. No `sorryAx` is accepted.

These prove reference denotations. They do not verify the Rust graph, encoder,
byte parser, mutexes, allocator, source solver or future fixed-point compiler.
In particular, a finite-diagram node count is not a finite-world proof. The M7
implementation must certify an exhaustive stable carrier and monotone operator,
then refine the bounded reference with explicit cancellation/resource refusal.

The old readiness record pins preimplementation production source. Do not
rewrite it to claim its isolation audit verifies the current engine. Revision
0.8 documents are preserved before revision 0.9 changes. The new current-proposal
auditor checks documentation/proof evidence without asserting Rust completion.

## Verification of the native slice

- Event core: 15 substantive tests, including exhaustive four-world supports,
  dense symbolic oracles, projection/counts, codec corruption, allocation/order
  changes, collisions, capacity/cancellation and owner lifetimes.
- Native Event storage: ten tests covering canonical dedup/reopen, cross-owner
  equality joins on resident/cursor paths, literals/parameters/set dedup, rollback,
  concurrent publication, malformed bytes, a pinned row fixture, same-row alignment,
  cancellation and retained pages. A separate internal test covers spill/stale keys.
- Rust query macro Event parameter test and native WorkContext test pass.
- Raw Node bridge: all 103 unit tests pass, including two Event tests for owned
  wire roundtrip, malformed/version refusal and worker cancellation. Node crate
  compilation includes every target. TypeScript checking and the wire-tag fixture
  test pass; this does not claim the complete packaged SDK has been qualified.
- Earlier regression run: 1,341 engine unit tests passed, 18 ignored; 25 API and
  five keyed-get tests passed. These precede the final same-row decoder change;
  the focused Event tests exercise that change. Log JSON: seven tests passed.

Formatting, whitespace checks and strict clippy pass for the core/engine and
the separate Node crate. The [native slice record](event-evidence/native-storage-qualification-final/check.json)
pins the checked source state and retained logs. The
[proposal handoff](event-evidence/implementation-handoff-0-9-final/check.json)
separately records documentation, proof and reference checks.
This evidence qualifies the described slice, not the entire proposal. The final
packaged-consumer battery and integrated performance qualification remain M8.
The local proposal laboratory is preserved and will be curated explicitly for
Git; it is not blanket-added or deleted.
