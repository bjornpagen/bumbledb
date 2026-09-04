# Assurance: proofs, tests, specifications, and their actual boundaries

## What is unusually strong

The project already has several independent kinds of evidence: Rust and TypeScript tests; compile-time type/refusal tests; SQLite differential query checks; a separate naive model; Lean denotations and abstract algorithm proofs; a three-way conformance lane; protocol goldens; an independently spelled grammar generator; allocation and assembly checks; and cross-language filesystem tests.

These are valuable precisely because they answer different questions. Keep the separation. A golden proves stability against an expected byte string; a differential model checks denotation; a lifecycle history checks ordering and visibility; an assembly gate checks a code-generation property. None subsumes the others.

`lean/README.md:67` expressly refuses verified Rust and describes the Rust↔Lean link as empirical. It also excludes durability/crash mechanism from the formal scope. This is an honest boundary and should appear prominently wherever “verified” is used to describe the product.

## ASS-001 — The braid theorem's closed-relation premise does not directly match the implementation

**Priority:** P2 proof/implementation coverage gap. **Confidence:** confirmed static counterexample to the stated mapping; not a discovered unsound partition.

**Evidence:** `lean/Bumbledb/Txn/Braids.lean:119-131` defines `stmtRels` to include both source and target of every containment/capacity statement, and `ComponentClosed` requires all such relations to have the same component. L9 assumes that premise (`:175`). Rust braid derivation deliberately omits edges to closed relations (`crates/bumbledb-log/src/braids.rs:148-160`).

Consider two ordinary relations R and S each referencing one shared closed vocabulary V. Rust can put R and S in different braids. The literal formal premise demands `comp(R)=comp(V)` and `comp(S)=comp(V)`, which forces R and S together. Therefore this Rust decomposition cannot directly instantiate that premise as written.

The intended independence remains sensible: V is immutable, so changing R cannot alter S's lookup target. The missing piece is a theorem/premise that explicitly ignores immutable consulted relations, or a proof that works with each closed denotation held constant independently of component membership.

**Recommendation:** align the model with mutable relation support, and test the derivation independently for shared closed targets, closed sources in capacity statements, isolated relations, and multiple statement types. Do not respond by unnecessarily connecting all relations through closed vocabularies; that would throw away the useful optimization rather than explain it.

**Acceptance:** the supported decomposition can be mapped into the theorem's actual hypotheses, and the bridge points to tests checking the mapping—not only a matching symbol name.

## ASS-002 — Existing gates are stronger on bytes and final states than on complete histories

**Priority:** P1 assurance work for hosted release. **Classification:** cross-cutting test gap, not an additional independent data-loss defect.

The new probes reproduce failures while selected existing suites pass. In particular, a restart-focused pending test can miss a subsequent commit on the same failed live writer; final-state replica equality can miss a dirty read that escaped before rejection; monotonically growing scalar checkpoint tests can miss component regression; freshly created lock directories can miss lifetime acquisition cost; and simulated post-step crashes can miss a process paused inside a filesystem mutation.

Required extension: a small independent history model. Track commands, accepted/rejected/unresolved outcomes, published slots, observed reads, checkpoint roots, retired floors, lease owners, and local borrows. Generate schedules, not just data values.

Properties to assert:

- Every published success has a recoverable authoritative effect/receipt.
- A read never exposes a rejected or uncommitted candidate through the committed-read API.
- No live publication occurs in a retired slot namespace.
- Every new recovery floor contains the old floor componentwise.
- An ID allocation is owned by at most one caller, even if counter bytes match.
- A lost/closed capability cannot mutate its resource.
- Deletion cannot remove a live or retained recovery dependency.
- Limits bound work/resources rather than only detect excess afterward.

Run the same abstract schedule against Rust, TS and a deliberately simple model. Add a small real-filesystem subprocess suite for death/pause/rename boundaries. Test actual adapter ambiguity independently of the driver; a mock that resolves ambiguity correctly can hide an incorrect production adapter.

## ASS-003 — Several human-facing specifications describe retired behavior

**Priority:** P2 documentation/operability defect. **Confidence:** confirmed by source comparison.

Examples:

- `ts-log/README.md` describes pid-liveness-based filesystem locks, old peer/version vocabulary and `generation` receipts. Current code uses expiring `LEASE/1` tokens and `slot` receipts.
- `docs/research/replication-prior-art/THESIS.md` and `IMPOSSIBLE.md` describe a carried/recomputed footprint algorithm and L6–L8 proofs that are not the current writer path.
- `crates/bumbledb-log/src/braids.rs:7` says braid derivation is implemented twice; the one-reader campaign moved TS derivation through Rust.
- `crates/bumbledb/src/api/prepared.rs:300` calls budgets host-settable, but the setters are absent (QRY-003).
- Root README installation examples name older release tags; this working tree's manifests are at 0.20.3. This does not invalidate historical benchmarks, which properly name their measured version.

Some source comments contain disconnected fragments after prior cleanup. A passing name census cannot establish that a sentence is complete or that an operator runbook describes the right algorithm.

**Recommendation:** designate current operational documents, label historical research as historical, and test public examples against the packaged API. Retain accessible explanations of why state transitions are safe. Keep theorem/implementation links, but do not replace explanations with symbol-presence checks.

## ASS-004 — The evidence must survive a completed audit campaign

**Priority:** P2 process/maintainability decision. **Classification:** prevention of recurrence, not a runtime defect.

Prior history records an audit/proposal retirement campaign. Current research and proposal files still refer to reports, normative chapters, and algorithms no longer present. Repeatedly deleting the evidence after implementing fixes makes subsequent reviewers reconstruct the same assumptions from git archaeology and persuasive comments.

Keep this audit immutable as a dated baseline. Add resolutions with:

1. Finding ID and original failure trace.
2. Accepted scope and severity, including any narrowed contract.
3. Fix commit and changed invariant/representation.
4. Regression test names and which schedule they cover.
5. Remaining unsupported environments or cases.
6. Reviewer who challenged the closure.

A disproved finding should remain as “dismissed, with evidence,” not silently disappear. A repaired finding is not a reason to delete its counterexample.

## Limits of this review

This is a broad multi-reviewer source audit with focused runtime falsification, not a formal verification of the implementation or an exhaustive inspection of every line. The repository contains approximately 920 tracked Rust/TypeScript/Lean files and 276,374 lines in those files, including tests, benchmark code and formal sources. No claim is made that all of them were read line by line.

The team prioritized the current log, engine admission/durability boundaries, query output/resources, TypeScript lifecycle/visibility, native ownership, hosted deployment, and their architectural interactions. Testing records distinguish existing suites, new external probes, static interleavings, and proposed acceptance tests.

No AWS deployment, cloud failure experiment, physical power-cut test, full benchmark rerun, exhaustive C ABI sanitizer campaign, or independent validation of the cited academic literature was performed. Historical context guided questions; current code and test observations grounded findings. The complete release battery was not invoked wholesale because package build/pack steps can mutate manifests and generated artifacts; individual non-source-mutating gates were selected instead.
