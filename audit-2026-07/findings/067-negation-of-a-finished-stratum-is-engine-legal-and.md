## Negation of a finished stratum is engine-legal and Rust-spellable but unwritable in the TS SDK

category: missing-free-feature | severity: medium | verdict: CONFIRMED | finder: cross:free-features
outcome: fixed 6e88d33c

### Summary

The engine deliberately keeps negation *of* lower strata legal — the strata judge refuses only negation *through* a cycle — and the Rust `query!` macro spells it with `!pred(...)`. The TS SDK has no spelling for it at all: `not()` accepts only stored relations and closed vocabularies, the runtime path crashes on a rec handle, and no negated-idb constructor exists anywhere in `ts/src/query/`. The classic complement query ("nodes NOT reachable from root") is a one-liner on the Rust surface and unrepresentable on the TS surface, even though the wire IR the TS SDK emits can already carry the shape and the engine machinery (stratification witness, anti-probe filters, per-stratum finished images) is fully paid for.

### Evidence

Engine legality (checked against the architecture spec, `docs/architecture/20-query-ir.md:131-142` § engine recursion — "`NegationThroughCycle` (negation *of* lower strata stays legal — a finished set is what keeps the operator monotone)"):

- `crates/bumbledb/src/ir/validate/strata.rs:128-135` — the safety roster refuses a negated atom only when its idb target shares the atom's own SCC (`scc[via] == scc[index]` → `NegationThroughCycle`); a lower-stratum target passes.
- `crates/bumbledb/src/ir/validate/strata.rs:28-30` (module doc): "Negation *of* lower strata is legal: a lower stratum is a finished set before this stratum's operator runs." Restated at `crates/bumbledb/src/error.rs:840` and `crates/bumbledb/src/error/display.rs:799`.
- Execution machinery is real: `crates/bumbledb/src/exec/run/anti_probe.rs` (anti-probe per surviving binding), and `crates/bumbledb/src/api/prepared/fixpoint.rs:630-634` — `fill_images` hands every idb occurrence of a lower stratum its finished image; negated occurrences are plan occurrences ("joins no node, probed through its anti-probe", `exec/introspection/tests.rs:129`).

Rust spelling:

- `crates/bumbledb-query-macros/src/lib.rs:866-871` — a leading `!` parses the following atom as `Item::Negated`.
- `crates/bumbledb-query-macros/src/lib.rs:1532-1533` — `Item::Negated` lowers through the same `atom()` path as positive atoms, and `atom()` at lib.rs:1391-1398 resolves macro-local predicate names to `idb_atom` → `AtomSource::Idb` (lib.rs:1383-1385). No polarity restriction exists, so `(n) | Node(id: n), !reach(n);` emits a negated Idb atom the strata judge accepts.

TS wall:

- `ts/src/query/atom.ts:470-476` — `function not<R extends MatchOwner, ...>`; `ts/src/query/scope.ts:63` — `type MatchOwner = AnyRelation | AnyClosed`. No RecRef arm.
- Untyped escape crashes: the negated path (`ts/src/query/lower.ts:690-699`) calls `resolveBindings` (lower.ts:486), which calls `sealedFieldsOf(relation)` (`ts/src/closed.ts:218-222`) — that reads `member.data.fields`, but a rec's shared data is `{ name, rules }` (`ts/src/query/predicate.ts:230`), so a rec passed to `not()` fails at construction.
- `advanceIdb` (`ts/src/query/lower.ts:714-731`) builds only positive `kind: "idb"` items, and `lowerRule` (`ts/src/query/lower.ts:1826-1832`) routes them into the positive `atoms` bucket exclusively — the `negated` bucket is fed only by EDB/closed atoms.
- The wire already carries the shape: `ts/src/native.ts:96-99` (`RuleIr.negated: readonly AtomIr[]`) with `AtomSourceIr = { kind: "edb", ... } | { kind: "idb", pred }` (native.ts:122-124). Only the surface constructor is missing.

### Failure scenario

A TS host porting the Rust complement query finds no spelling. The workaround is executing the closure program, collecting its answer set host-side, and folding a set difference against a full scan — an O(n) host loop plus a full result marshal, where the engine already runs the same anti-probe in-plan over the finished stratum's transient image. This also violates the SDK-parity expectation the surface-pair work established: the two surfaces are supposed to spell the same IR.

### Suggested fix

Add a rec-accepting negation to the output-rule scope — either `r.notIdb(rec, { c })` or widen `not()` with a RecRef arm dispatched at runtime by the rec-handle tag — lowering to an `AtomIr` in the `negated` bucket with `source: { kind: "idb", pred }` over the rec's head-keyed named record (the same named-record wall `idb()` already enforces at `advanceIdb`, plus the existing negated-atom boundness wall at lower.ts:1040-1046). No engine change is needed: the strata judge, anti-probe execution, and finished-image plumbing already accept and run the shape.
