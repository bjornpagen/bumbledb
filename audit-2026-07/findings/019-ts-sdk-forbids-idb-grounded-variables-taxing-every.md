## TS SDK forbids idb-grounded variables, taxing every recursive query with a spurious re-grounding join the engine never asked for

category: missing-free-feature | severity: high | verdict: CONFIRMED | finder: cross:free-features

### Summary

The Rust engine treats a positive `Idb` (recursive-predicate) atom exactly like a positive EDB atom for variable boundness: a variable bound at an idb position enters `atom_vars`, may appear in the rule head, and satisfies negation safety. The idb-only output rule `(c) | reach(c);` is legal, compiled, and executed by the engine — the Rust cookbook test proves it end-to-end. The TS SDK alone walls this off: `advanceIdb` deliberately does not add idb binding refs to the rule's `bound` set, so `validateIdb` (and the find-column boundness check) reject any rule whose variable is grounded only through an idb atom. Consequence: the identity projection of a finished stratum is unwritable in TS, and every recursive TS program's output rule must carry a redundant `.match` over a domain relation — one extra join atom per execution. The two hosts' spellings of cookbook recipes 24/25 therefore lower to different ProgramIr despite the cross-host parity doctrine, and docs/feature-register.md mislabels the tax as "engine law".

### Evidence (all verified against the working tree, 2026-07-23)

Engine grounds via Idb atoms:
- `crates/bumbledb/src/ir/validate/context.rs:471-536` — `check_atoms` walks Idb atoms in the same loop as Edb: the source screen at line 501 (`AtomSource::Idb(pred) => idb.screen(occ_idx, pred)?`), per-binding column typing at line 521 (`idb.column(occ_idx, pred, *field)?`).
- `crates/bumbledb/src/ir/validate/context.rs:600-607` (`check_scalar_binding`) and `:552-558` (`check_interval_binding`) — `Term::Var` on any positive atom does `self.atom_vars.insert(*var)`; Idb and Edb are indistinguishable here. Head boundness and negation safety read `atom_vars`.
- **Executed proof**: `cargo test -p bumbledb-query --test cookbook r24_closure` passes. The test (`crates/bumbledb-query/tests/cookbook.rs:1763-1766`) builds `(c) | reach(c);` as the output rule — a rule whose ONLY atom is the Idb atom — prepares it, executes it with `execute_collect`, and asserts result equality with the host-loop dialect. Same form in `docs/cookbook.md:1161`.
- `docs/architecture/20-query-ir.md` § engine recursion: the only Idb refusals recorded as law are the two grounding rewrites (statement elimination and statistics, lines 156-163) and value creation in recursive heads — nothing forbids idb-bound head variables; line 107 states an Idb atom's bindings address head columns typed field-for-field.

TS-only wall:
- `ts/src/query/lower.ts:714-735` — `advanceIdb` returns `bound: state.bound`: an idb atom never grounds. Only `advanceMatch` (`:580-596`) extends `bound`.
- `ts/src/query/lower.ts:1001-1006` — `validateIdb` throws "idb ... names the variable ..., but no relation atom of the rule binds it — an idb atom is a join position; bind the variable through the theory's own relation first".
- `ts/src/query/lower.ts:1013-1018` — the class wall joins `mintSlotOf(context, binding.ref)` (the variable's mint, available with or without a relation atom) against the head column's classed slot; it does not depend on the grounding precondition.
- `ts/src/query/lower.ts:883` (`validateColumn` → `assertBound` at `:843`) — the find column of an idb-only rule would also fail boundness; extending `bound` in `advanceIdb` fixes both walls at once.
- No TS test asserts the restriction (the error string appears only in lower.ts).

Drift and mislabel:
- `ts/COOKBOOK.md:1209-1235` — recipe 24's engine-native output rule is `r.match(Node, { id: c }).idb(seeded, { c }).find({ c })` with the prose "an `idb` atom is a join position, so the head rides the `Node` atom", vs the Rust cookbook's bare `(c) | reach(c);`. Recipe 25 (`:1246+`) carries the same pattern.
- `docs/feature-register.md:138-139` — "The idb re-grounding tax (an idb atom is a join position) — engine law, documented, ~6 recursive queries carry one extra `.match`." The passing engine test refutes the "engine law" label; the tax is SDK conservatism.

### Failure scenario / Bench impact

- Expressiveness: any TS rule that reads a finished stratum without a domain re-join is unwritable (identity projection of a predicate, or joining two predicates of the program against each other without an EDB anchor).
- Perf: every recursive TS program pays one extra join atom (a full scan-and-join over the domain relation, e.g. `Node` or `Account`) in its output rule, on every execution, purely to launder boundness the engine already grants. This is a spurious hidden join in a hot path — exactly the class the allocation-control/representation-first doctrine treats as a finding.
- Parity: the Rust and TS spellings of cookbook recipes 24/25 lower to DIFFERENT ProgramIr (extra `Node`/`Account` atom TS-side) despite the cross-host parity doctrine; the schema-fingerprint pins hash theories, not queries, so nothing catches the drift.

### Suggested fix

1. In `ts/src/query/lower.ts`, make `advanceIdb` extend the rule's `bound` set with its binding refs (idb atoms are positive occurrences — this is exactly the engine's representation in `check_atoms`). This simultaneously satisfies `validateColumn`'s find-boundness check.
2. Delete `validateIdb`'s relation-atom precondition (lines 1001-1006), now vacuous; keep the mint-slot/head-slot class check (lines 1007-1018), which is grounding-independent.
3. Rewrite `ts/COOKBOOK.md` recipes 24/25's output rules to the idb-only form matching `docs/cookbook.md:1161`, and add a TS test executing it against the engine.
4. Correct `docs/feature-register.md:138-139`: the tax was SDK-only, now removed — not engine law.
