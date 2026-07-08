# PRD set — reconciling the code against the 2026-07-08 architecture

This directory is the complete, ordered work plan that takes the codebase from the
pre-redesign engine to the architecture documented in `docs/architecture/`. When a
PRD and an architecture chapter disagree, **the chapter wins** and the PRD is
amended.

## Policy (read before executing any PRD)

1. **A PRD is a work-organizational unit, not an atomic passing-code state.** The
   tree is *expected* not to compile between PRDs. Never write a transitional shim,
   a compatibility alias, a deprecated re-export, or a feature flag to keep old and
   new worlds coexisting. Rip the old thing out and cut directly to the end state;
   downstream breakage is the next PRD's job.
2. **Passing criteria are evaluated when the criterion's dependencies exist**, not
   necessarily at the PRD's own completion. Criteria marked `[shape]` are checkable
   immediately (the code exists and has the stated form — verifiable by reading or
   grep). Criteria marked `[test]` are unit tests written *in this PRD* that must
   pass once the tree compiles again. Criteria marked `[gate]` hold at the end of
   the whole plan (`cargo fmt --check`, `clippy -D warnings`, `cargo test`).
3. **No migrations, ever.** Pre-redesign stores do not open (format version bump,
   PRD 06). Do not write conversion code. ETL is a human decision.
4. **No smoke-test or end-to-end-test PRDs.** Unit tests co-located with the code
   they pin are in scope and required where a PRD says so. Running the verify/bench
   harness, wiring CI, and judging results is human work.
5. **Vocabulary discipline is a requirement, not a style preference.** New code
   never introduces the deleted words (`unique`, `foreign key`, `fk`, `primary key`,
   `constraint`, `cascade`, `restrict`) as identifiers or doc-comment concepts;
   PRD 25 purges the survivors. The replacement vocabulary: *statement*,
   *functionality / key (FD)*, *containment (IND)*, *judgment*, *guard*,
   *reverse edge*.
6. **Deviation handling:** if executing a PRD reveals the architecture docs are
   wrong or silent, stop, record the conflict in the PRD file under a `## Conflict`
   heading, and leave the decision to the owner. Do not improvise semantics.

## Execution order

Strict order within phases; phases are ordered. Do not start a PRD whose
dependencies are unfinished.

| Phase | PRDs | What exists at the end |
|---|---|---|
| A — type & schema foundation | 01 02 03 04 05 | Interval type; statement descriptors; validation + acceptance gate; fingerprint; the new `schema!` |
| B — storage & judgments | 06 07 08 09 10 | New key layout + format bump; FD enforcement (scalar + pointwise); containment both sides; WriteTx point reads |
| C — query surface | 11 12 13 14 15 16 17 18 19 | New IR; validation; lowering; interval images; planner; anti-probes; param sets; new sinks; point-lookup path |
| D — API boundary | 20 | Error taxonomy, statement rendering, param-set binding |
| E — oracle & bench infrastructure | 21 22 23 24 | The naive model; SQL translator extensions; generator coverage; new ledger |
| F — closure | 25 | Vocabulary sweep; root README example updated |

## The PRDs

- [01 — Interval value type and encoding](01-interval-value-type.md)
- [02 — Statement descriptors replace constraints](02-statement-descriptors.md)
- [03 — Schema validation: the roster and the acceptance gate](03-schema-validation.md)
- [04 — Fingerprint over statements](04-fingerprint.md)
- [05 — The `schema!` macro: statement grammar](05-schema-macro.md)
- [06 — Storage keys and format version](06-storage-keys-format.md)
- [07 — Commit: functionality enforcement](07-commit-functionality.md)
- [08 — Commit: containment, source side](08-commit-containment-source.md)
- [09 — Commit: containment, target side](09-commit-containment-target.md)
- [10 — WriteTx point reads](10-writetx-point-reads.md)
- [11 — IR shape](11-ir-shape.md)
- [12 — IR validation roster](12-ir-validation.md)
- [13 — Normalization and lowering](13-ir-normalization.md)
- [14 — Images: interval columns](14-image-interval-columns.md)
- [15 — Planner over statements](15-planner-statements.md)
- [16 — Executor: anti-probes](16-executor-anti-probe.md)
- [17 — Executor: param sets and membership](17-executor-paramset-membership.md)
- [18 — Sinks: CountDistinct and Arg-restriction](18-sinks-aggregates.md)
- [19 — Guard-probe point lookups over statements](19-guard-point-lookups.md)
- [20 — API: errors, rendering, binding](20-api-errors-render-bind.md)
- [21 — The naive model](21-naive-model.md)
- [22 — IR→SQL translator extensions](22-sql-translator.md)
- [23 — Query generator coverage](23-querygen-coverage.md)
- [24 — The new ledger benchmark schema and families](24-bench-ledger.md)
- [25 — Vocabulary sweep](25-vocabulary-sweep.md)
