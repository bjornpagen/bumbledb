# 90 — Proposal provenance, review and evidence boundaries

Date: 2026-09-04. This file records the scope of the successor proposal; [audit/90-evidence.md](../audit/90-evidence.md) separately preserves the preceding code audit and executed reproductions.

## Inputs

- The repository working tree at base commit `dc078489c98777600e92cbc49590219e0e2f6122`, including existing owner changes. The audit identifies that working tree as 0.20.3 even though the committed base is an earlier packaging release.
- All 19 Markdown files in `audit/`, including complete reproduction harnesses and their limitations. Each of the three subsystem reviewers reread the entire audit before writing the successor chapters.
- The owner's current brief and refinements: breaking 1.0, floats including sum/mean and parameterized intervals, no arbitrary LMDB/RAM ceiling, aggressive casework elimination, minimal core, log-layer backup/migration, useful nightly, justified multiwriter, all required tests, TypeScript-only public log, hard deletion of the entire public C product, schema-generated migrations, and a Next.js/Alchemy/Vercel experience for per-student/per-user applications.
- Both supplied representation-first attachments, read in full and byte-identical: SHA-256 `a931bb20a66d732fa66961fac6e1e249f1fee1166f920f313ce46b943fd663c3`. The quoted historical attributions are supplied material, not independently verified scholarship.
- The original July 10 Nessie context, [“Bumbledb: The Database Is a Theory”](https://nessielabs.com/n/f9c6a22d-ddf8-4228-aee9-669d3f564d05), read in full for intent. The Nessie skill was used read-only; no external context was created or updated.
- The in-repo README read in full, benchmark storage/SQL mapping and relevant executor/key/encoding sources, and the sibling `../bumblebench` README/charter plus the evidence and antagonists identified in chapter 40. Historical timing/space claims were not rerun. The README's literal `night-2026-08-22/` raw artifact path is absent in this checkout, so its results remain attributed records rather than a newly verified campaign.
- TigerBeetle primary source at commit `47aeb2212a255273dda508288412e537d11e4b7c`: checksum implementation, vendored AEGIS and checksum benchmark; BLAKE3 official implementation description and SQLite's file-format documentation. Chapter 41 records exact source links, role/width decisions and collision arithmetic. No new hash-speed result is claimed.

The historical material helped recover the set/theory-first thesis. It did not veto the owner's newer requirements. Old no-floats, single-process-only, permanent tooling bans and other accidental historical constraints are not preserved when they conflict with the selected successor. Neither Claude's authorship of old code nor a different model's review is evidence of correctness by itself.

## Parallel review and integration

| Reviewer workstream | Owned chapters and principal scope |
| --- | --- |
| Engine | 10–13 and 40; canonical values/admission, intervals, LMDB growth/snapshots, deterministic floats, Free Join/fallback, Lean/Rust/nightly, in-repo and M2 Max performance evidence/constants |
| Replication | 20–22; authority, receipts/unknown outcomes, checkpoints/GC, LocalHistory specialization, backup/restore/migration |
| SDK | 30–33; ownership, clients/native ABI, tenant/runtime/packaging, TypeScript migration and app workflow |
| Primary integration | 00–02, 41, 50, 60, 70, 90 and README; philosophy, scope, concurrency judgment, storage/hash accounting, complete issue/gate inventory and release plan |

Engine, replication and SDK reviewers cross-read peer chapters. The primary reviewer read the detailed chapters and reconciled conflicts rather than concatenating independent proposals. The work used three concurrent subsystem agents, followed by targeted follow-up review; agents changed only their Markdown contributions and explicitly assigned review documents.

### Important corrections made during review

- Chose a single tenant publication order, but explicitly preserved many concurrent hosted writers. Semilattice union does not certify keys/capacity or delete/read-dependent command intent.
- Distinguished LocalHistory's atomic LMDB authority from HostedHistory's object/tail/epoch machinery. Local embedded history does not need remote recovery scaffolding.
- Retained compact integer interval endpoints while adding dense numeric `Interval<F64>` with the same two-word shape; NaN is not a bound, infinities are bounds only, and float length is not discrete point counting. Fixed-width float intervals and approximate capacity weights are not added.
- Replaced multiple float nonfinite flags with a canonical sum type; exact accumulator merge requires disjoint deduplicated binding partitions and is not idempotent.
- Separated head revision, durable decision identity and application-state revision. Maintenance/no-op/rejection do not spuriously invalidate application-state witnesses.
- Resolved the admission/receipt-hash cycle with one narrow post-judgment host-record seal. It can mutate opaque host records but cannot amend already judged application facts.
- Removed FreshRef and 28-byte issued-ID machinery entirely. Commands carry application-owned 128-bit IDs before sealing; one concrete payload needs no symbolic resolver or generated-ID result map. Restore preserves those values; their bits are not a current-lineage permission token.
- Replaced shared result/cursor ownership with a consuming transfer; separated owned results from mapped snapshot lifetimes and legal Rust borrows from revocable managed tokens.
- Required exact stamp ancestry evidence for `AtLeast`; pruned evidence yields `WitnessUnavailable`, not a sequence-only pretense.
- Narrowed retention to named roots, separated independent backups, and accounted for late old uploads as eventual orphans instead of claiming instantaneous perfect reclamation.
- Replaced handwritten migration modules, helper-closure hashes and JavaScript purity restrictions with schema-generated canonical plan/history data. The native log executor preserves necessary intermediate semantics but publishes one final target per pending batch; users supply declarative intent for ambiguity, not imperative migrations.
- Hard-deleted C from the successor product contract, preserving corresponding Rust/Node ownership regressions. Actual source deletion is an implementation step, not a change made in this documentation revision.
- Restored Free Join/selective indexed application paths as primary; bounded disk execution remains ordinary fallback. Apple Silicon is the initial optimization target; Graviton and x86 Vercel portable/runtime correctness are required without pretending M2 constants describe those CPUs.
- Split local fingerprints from authoritative commitments. Selected exact-checked 16-byte local fingerprints and retained 32-byte remote/schema/command identities; investigated TigerBeetle AEGIS as a measured candidate rather than assuming a universal speed win. Explained current physical amplification separately from RAM-only query structures.
- Separated prepublication artifact qualification from the public registry checks that can only happen after authorized promotion.

These decisions are reflected in the normative chapters and root contract. They are not independent optional alternatives to be recombined arbitrarily during implementation.

## What was actually checked in this proposal phase

The team inspected current source, examples, toolchain/runtime definitions, installed LMDB wrapper documentation, relevant CI/scripts and the audit. The primary reviewer read the existing workspace/log/C workflows and battery/check/Miri scripts to ground the release gaps. Nightly feature availability was inspected; chapter 13 distinguishes that from compiling/benchmarking a successor prototype. Follow-up read-only Nessie research revisited original intent; the current typed schema/query constructors and direct IR lowering, together with the owner's latest instructions, ground the generated migration design. Historical model suggestions were not treated as owner decisions.

Documentation QA checks local links, code fences, whitespace, the complete finding-ID roster, the detailed gate cross-index, current scope terminology and the staged-file boundary. Cross-review challenged specific state/proof/API seams rather than supplying an approval based on document volume. Git inspection separates preexisting committed/uncommitted work from this documentation-only change.

The revised documentation inventory has **19 audit files and 21 proposal files**. Mechanical checks cover local file links, fenced blocks, final newlines and trailing whitespace. All **68 indexed audit IDs** (47 implementation observations plus 21 architecture/operations/performance/assurance concerns) appear in the closure matrix. The **220 detailed test families** have matching chapter definitions and release cross-index entries; `PKG-07B` is explicitly post-promotion distribution verification. The added families cover float intervals, hosted workload qualification, application performance/constants and storage/hash sizing. These are document completeness checks, not execution of those future tests.

No successor implementation or release tests were run in this phase. The previous audit's executed baseline remains:

| Prior evidence | Recorded result | Important limit |
| --- | --- | --- |
| Locked workspace nextest | 2,049 passed; 30 skipped | Existing 0.x tree; excludes separately built Node/C bridges |
| Lean/conformance | Build completed; 277 cases, zero disagreements | Finite semantic bridge, not Rust/LMDB/S3 verification |
| Selected TS/Node suites | 209 passed | Existing native artifact, not a fresh 1.0 build |
| Workspace format/Clippy | Passed | Not the complete feature/platform/release matrix |
| Targeted old counterexamples | Preserved in audit 11/22/32 | See exact static/reproduced labels and artifact caveats |

These results are not inherited passes for any new child gate. No AWS operation, deployment, publication, production data migration, physical power cut, new performance benchmark, fresh successor native build or new Lean proof completion is claimed by this proposal.

## Required evidence still ahead

The chosen protocol needs independent model and real S3 qualification. The F64 specification needs completed numerical proofs, a genuinely independent bit/rational oracle and hardware/compiler/FPU evidence on every supported target. The map/cursor/scratch design needs crash, lifetime, resource and large-data qualification. The TypeScript migration runner needs actual packaged history/crash/deployment tests. The performance costs need measured application workloads.

The proposal's safety arguments are conditional on explicit substrate/identity/crypto/host premises, not formal proofs of all Rust, LLVM, LMDB, AWS and hardware behavior. Chapter 13 owns the exact assurance boundaries; chapter 70 owns the release result manifest. Unknown future defects remain possible. Every **known** supported-behavior defect must receive actual closure before release.

## Change hygiene and handoff

The original documentation commit included only `audit/` and `final-solution/` Markdown. This follow-up changes only `final-solution/`. Preexisting Cargo/manifests/lockfiles, log grammar/store/tests, packaging scripts, TypeScript source/build support and deleted old grammar files are left untouched and unstaged. Existing `audit/` evidence remains dated rather than rewritten to describe the successor.

The proposal is committed and pushed on a dedicated `codex/` proposal branch so publishing the documents does not advance the remote main branch through unrelated preexisting local commits. No force push, source reset, release tag or registry publication is part of this handoff. The final response supplies the verified documentation commit and branch; the repository history records the exact file contents without a self-referential commit hash in this file.

Next work begins only with implementation authorization. Start from [60](60-implementation-and-release-plan.md), preserve [50](50-audit-closure-matrix.md), and treat missing evidence in [70](70-test-and-release-gates.md) as work, never as a prose waiver.
