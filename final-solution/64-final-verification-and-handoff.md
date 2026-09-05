# 64 — One final verification campaign, honest release evidence

This chapter controls **when** to execute [70](70-test-and-release-gates.md),
not whether to execute it. The current proposal refactor runs no tests, builds,
typechecks, linters or performance probes. Neither does future implementation
until the F2 barrier in [61](61-orchestration-and-dependency-graph.md).

## Before the barrier: write the suite, do not run it

P01–P14 author their permanent tests and P12 maps every audit/gate to an execution
lane. P13 fixes packaging/runner declarations as source edits. P14 preserves
baseline revisions, records historical results as historical, and authors cost
probes. Reads of source, dependency docs, Git diff and manifests are allowed.
Do not sneak in a build with `pnpm test`, package prepack, a generator, a probe,
CI push or “just a quick compiler check.” Dependency installation with lifecycle
hooks is also deferred to F3. No verification is needed to prepare this proposal.

An implemented packet has all selected behavior and authored regression source,
not just a declaration/TODO. It stays unqualified until actual execution. F2
source review must check the stopped boundaries explicitly: computed module and
sink wiring, numerical guard, general interiors, owned snapshots, worker affinity,
complete Effect cutover, one FS CAS authority and real generated migration flow.

## Final campaign order

1. **Record candidate and environment.** P00 freezes the integrated source,
   dependency/toolchain/platform/backend matrix, obligation inventory and exact
   commands. Inventory current hardware/credentials/disk before allocating
   large fixtures or requesting external resources. Do not silently narrow a
   required platform because the current machine cannot run it.
2. **Fresh builds and structural gates.** Clean pinned installs; Rust workspace
   and separate ts/crate; Lean; TS core/log types/lint/tests; docs, feature matrix,
   forbidden surface/dependency checks. Do not use old dist/.node artifacts.
   Repair all integration failures, then rebuild; no test waiver.
3. **Local semantic and lifecycle matrix.** All unit/property/independent oracle,
   negative compile, conformance, resource, process/crash, ownership and exact
   cross-language tests. Exercise both native and actual public Effect paths.
   Run real parallel suites; do not suppress stdio LEAK or serialize to hide it.
4. **Pre-format measurements.** Run authored physical-layout/long-key/hash
   probes and matched baseline comparisons. Default selected roles remain
   16-byte exact-checked local fingerprints and 32-byte authoritative BLAKE3.
   If evidence warrants a different choice, write the decision, notify all C12
   consumers, finish changes and repeat affected semantic/canonical tests.
   Freeze one physical encoding and golden corpus only after this decision.
5. **Whole product and large-data performance.** Warm/cold/after-write/forced-
   disk, full hosted decisions/checkpoint pressure, tenant churn, Effect/V8 and
   event-loop costs. Physically populated >40 GiB and separately memory-enforced
   >RAM workloads, actual disk and failure handling. Performance runs serialize
   per machine; unrelated benchmark agents must not compete for the same host.
6. **Fresh staged artifacts and actual backends/targets.** Tarball-isolated
   consumers, exact native handshake, supported Apple Silicon/Graviton/x86
   Node matrix, real S3 conditions, backup/restore/migration and deployment
   drills. External mutation requires an explicitly authorized disposable
   scope. No cloud provisioning/publication/production tenant changes by inference.
7. **Full requalification and review.** Every required chapter 70 child and
   chapter 50 audit has exact revision/artifact evidence and independent review.
   Rerun impacted lanes after fixes; source/codec/toolchain changes invalidate
   affected old evidence. A final fresh full local battery and full configured
   CI matrix cannot be replaced by path-filtered smoke tests.
8. **Git handoff, not release promotion.** Commit/push the integrated result and
   evidence under the rules below. Missing external gates are reported as
   blocking release, never Passed. Tag/version/publication require separate
   authorization; PKG-07B is explicitly post-publication only.

This ordering moves former “early probes” to the beginning of final
qualification. It does not freeze an unmeasured physical design merely because
the other implementation is written. Prototype/review effort can be wasted by
late measurements; that is the accepted cost of final-only execution.

## Reuse real repository machinery

At the frozen checkpoint, `scripts/battery.sh` includes workspace formatting/
clippy/nextest, the dependency-lean internal log feature lane, `scripts/check.sh`,
Lean/census, separate native bridge checks/tests/build, both TS package tests/
types/lint and `scripts/packed-import.sh`. It is the starting integration spine,
**not all of chapter 70**. P13/P00 update it for deleted APIs and new fixtures
without removing selected properties. `scripts/miri.sh`, measurement/assembly
scripts and backend/platform/deployment lanes supplement it as chapter 70 says.

P12's final execution manifest records actual commands, paths, required runner,
environment, fixture size, expected nonzero test inventory, artifact input,
output/report path and the covered IDs. Derive commands from the implemented
harness; do not invent flags or claim existing batteries cover new tests merely
because they have broad names. Inspect package scripts: both current TS test
commands invoke builds. No part of the spine runs before F3.

The existing release checker command is:

```sh
node scripts/release-results.mjs pre-promotion implementation/release-results.json <exact-staged-source-revision>
```

Replace the placeholder with the recorded commit, never a convenient unrelated
HEAD. It checks evidence; it does not execute qualification or manufacture it.
The current inventory is 68 audits, 17 parents and 220 child families. Preserve
all rows when routing work. Newly found defects add obligations; counts are a
floor, not a reason to delete new failures. No run is performed in this refactor.

## Candidate identity without checkpoint churn

No agent commits or pushes, and no implementation-phase checkpoint commits are
required. At F3, P00 can make the single integrated **local candidate commit** so
fresh artifact builds and evidence name a real source revision. Do not push it
until local final verification has succeeded or an explicitly incomplete handoff
is requested. If final verification needs fixes, amend only this unpublished,
campaign-owned candidate and rerun affected qualification against its new SHA.
Never amend another person's commit or force-push an already published revision.

Push the qualified code candidate without `[skip ci]`; await the actual full CI
outcome. If CI fails, repair and requalify, preserving the failure evidence; a
normal corrective commit is preferable to rewriting published history. One
final outcome matters, not an artificial exactly-one-commit constraint that
would conceal remote failures.

Evidence must identify the code commit it tested. An evidence-only documentation
commit may follow to add reports, links and specification/ledger records; it
does not pretend to be the source of earlier binaries. Use the checker's explicit
source-revision argument. Keep artifact inputs unchanged and record source tree,
toolchain/locks, platform/features and digests. No circular attempt to embed a
commit's own SHA inside files that determine that SHA.

The two current preservation/proposal commits use `[skip ci]` specifically because
the owner stopped execution and asked for an unverified handoff. That exemption
does not carry into a claimed completed implementation.

## Evidence and failure rules

- One `implementation/release-results.json`, with immutable referenced reports
  and artifact hashes. Per-packet notes link it rather than asserting their own
  competing green status.
- Passed requires actual applicable executions, nonzero cases and reviewed
  outputs on the exact candidate. Missing credentials/disk/runner, test skips,
  zero selection, stale binaries, timeout or unavailable reports are not a pass.
- Model equality, F64 canonical bits and exact complete outcomes matter; no
  tolerance that erases numerical or certainty bugs. Do not regenerate expected
  output from the same production helper being tested.
- Close success means real reclamation. A defect in finalizer Cause after a known
  receipt is not rejected publication. Test both truths independently.
- Keep actual baseline failures and new counterexamples. Do not “fix” a flaky
  suite by disabling, broad timeout inflation, weakening fixtures or changing
  expected output to match wrong behavior.
- Provisioning/credentials that are not available create an external blocker.
  Finish safe local implementation/evidence, describe exactly what is missing,
  and request only that missing authority. Never label 1.0 ready with required
  backend/platform/large-data/consumer evidence absent.

## Proposal retirement is an implementation dependency

The current `scripts/release-results.mjs` reads audit and gate inventories from
`final-solution/50-...` and `70-...`, and scans the other chapters for missing
children. Deleting this folder today would break that qualification machinery
and destroy the remaining-work contract.

Keep the folder for this handoff. If the future campaign completes the selected
work and retires the proposal, first move the normative design and complete
gate/audit inventory to permanent documentation, update the checker and its
tests to those actual paths, update all links, and include those changes before
final qualification. Preserve chapter 90's historical evidence and `audit/`.
No empty ledger, deleted check or lost proof obligation may make retirement
look like completion. If anything remains unimplemented/unqualified, retain the
proposal and report the remaining work.
