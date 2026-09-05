# 02 — Whole-tree checkpoint, not a 1.0 release

Date: 2026-09-04. Branch: `codex/bumbledb-1-0`. Parent: `57f939d34ba1a14e84982eb18fb4611d6af9458b`. The owner requested final review plus committing/pushing all current work. This saves incomplete implementation and existing owner edits; it does **not** authorize or claim a version bump, release tag, package publication, data migration or completed rewrite. The final review was solo.

## Included work and limits

| Packet | Included | Still not established |
| --- | --- | --- |
| C removal | Public crate/header/export implementation, C smoke/tests/workflow removed; live workspace/release references updated. [Coverage transfer](01-c-surface-removal.md) retained. | Shared Rust/Node owner safety is not closed by deleting C. Removed source is recoverable from Git; the old ignored build cache was moved to temporary recovery storage, not committed. |
| Canonical values | Core/theory `F64` and `Id128`, private canonical scalar representations, 12 primitive regressions, partial type/schema/macro/query/encoding integration. | End-to-end F64 integration is unfinished; exact float sum/mean, float intervals, independent numerical/Lean/platform qualification remain required. Id128 is a host nominal role over 16 fixed bytes, not a new allocator. |
| Log floats | Canonical raw float payload bytes in the old log codec, schema-file literals, inspect, refusal identities and eight codec/replay tests. | This extends the existing v3 format as an implementation packet; it is not the successor format freeze/reset or protocol. Core storage remains the old v8 line. Do not publish this checkpoint. |
| Local ownership | Kernel-held lock, stable sibling lock namespace, ownership before protected cleanup, explicit unlock, multiprocess/rename/pause tests. | This does not replace the old braided protocol with the proposed tenant-wide LocalHistory/HostedHistory or establish full native-owner closure. Filesystem/platform/S3 qualification remains. |
| Existing owner edits | 0.20.3 source/version alignment, retired tilde-key grammar, log store/conformance changes, ts-log declaration/distribution build support. | Preserved as requested, without a new version bump. Package/build changes are not freshly packed/published qualification. |
| Test plumbing | Parallel nextest coverage, no retry-to-green, finite CI stuck-test termination, JUnit, fuller feature/Node bridge checks and branch CI trigger. | Workflow coverage is not evidence of passing tests. Missing S3 credentials still allow an old skip-success path; that is not a release-qualified lane. |
| Effect proposal | Exact Effect 4.0.0-rc.112 hard cut for both packages; core primitives/runtime reused by log; simpler draft rerun contract; final docs/type/runtime review. | Neither TS package implements the proposed Effect dependency/API yet. Scope, native cancellation, bounded workers/conversion, page streams and generated migrations remain implementation work. |

## Current checks

Environment: Apple Silicon macOS, Node `v26.4.0`, Rust `1.99.0-nightly (d453bdd8f 2026-08-14)`. Effect probes used the sibling consumer's installed Effect `4.0.0-rc.112` and TypeScript `7.0.2`; no dependency installation or Edullm edit was made.

| Command/check | Result | Evidence boundary |
| --- | --- | --- |
| `cargo check --workspace --all-targets --locked` | **Failed**, exit 101 | Missing F64 cases in the query notation fixture and 21 benchmark/oracle matches. No all-workspace successor run was possible. |
| `cargo check --manifest-path ts/crate/Cargo.toml --locked` | **Failed**, exit 101 | Eight missing F64 matches in the excluded Node bridge; workspace-only checks would miss them. |
| `cargo nextest run --locked -p bumbledb --test value_primitives --profile ci` | **12 passed, 0 skipped** | Run `2ed50b2e-99c9-4907-a711-af722a55ca6e`; scalar payload/ordering/ID evidence, not engine-wide float qualification. |
| `cargo nextest run --locked -p bumbledb-log --test float_codec --test local_ownership --test lane_e_lease --profile ci` | **21 passed, 0 skipped; 1 LEAK diagnostic** | Run `5281ade7-716b-4004-b795-9f4120a1e7c9`. Includes the formerly failing multiprocess lease test. This is not an unqualified green run. |
| Isolated `float_codec` test selected by `-E 'test(every_truncated_float_payload_refuses_and_legacy_tags_keep_their_meaning)'` | **1 passed, 7 filtered out**, no LEAK | Run `d070cbd8-edc4-42a8-982f-660e1590af54`; diagnostic rerun only. The original leak observation is retained, not turned green by retry. |
| Strict compile-only Effect mock, `--noEmit --strict --skipLibCheck --target ES2022 --module NodeNext --moduleResolution NodeNext` | **Passed** | Exact installed RC declarations; no implemented Bumbledb API or complete dependency typecheck. |
| `pnpm exec tsc --noEmit` in each of `ts/` and `ts-log/` | **Passed** | Existing TypeScript surfaces/dependencies, not freshly built native artifacts or the proposed Effect SDK. No build/pack/publish hooks were run. |
| [Effect runtime probe](03-effect-runtime-probe.md) | **7 probe groups passed** | Real Effect runtime with mock resources, not Rust/Node cancellation, memory, S3 or performance qualification. |
| Documentation consistency and `git diff HEAD --check` | **Passed** | 26 proposal/implementation Markdown files, 175 local file links, complete 68-ID audit coverage and 220 detailed gate-family definitions/cross-index. Completeness, not test execution. |

Nextest reported LEAK for `float_codec::every_truncated_float_payload_refuses_and_legacy_tags_keep_their_meaning`. That status indicates captured output handles remaining open after process exit, not proof of a native heap/LMDB leak. The cause was not identified in this review. Preserve/reproduce the parallel schedule and investigate runner/process/descriptor lifetime before qualification; an isolated pass does not close it.

Historical implementation run `6a6700d4-aacd-4e19-a8be-81b9c6f302c7` reported **2,069 passed, 1 failed, 30 skipped**; the failure was `bumbledb-log::lane_e_lease::leases_are_disjoint_across_processes`. Source evolved afterward. The current focused pass is evidence for its tested scope, not a replacement whole-suite result. Earlier audit and C-removal baseline evidence remains separately dated.

## Concrete resume points

1. Finish F64 deliberately through every semantic boundary. Workspace errors include `crates/bumbledb-query/tests/notation_corpus.rs:167` and benchmark comparison, judgment, conformance, digest, theory/query generation, independent oracles, SQLite mapping and translation. Do not silence them with wildcards, `todo!`, skipped tests or an oracle copied from the implementation. NaN/zero identity and nonfinite SQLite comparison need explicit semantics.
2. Complete Node marshaling/output/tag/bind handling. Current errors are `ts/crate/src/marshal.rs:216,423,1252,1314,1350`, both `wire_tags!` invocations in `src/tags.rs`, and `src/lib.rs:221`. Mirror `NonCanonicalF64` into `ts/crate/log-identities.json`, `src/log.rs::BATCH_DECODE_IDENTITIES` and its exhaustive witness. Preserve exact diagnostic float bits as text across JS, not rounded numbers.
3. Implement the single Effect API from [35](../final-solution/35-effect-typescript-contract.md), with [34](../final-solution/34-sdk-syntax-and-composition.md) as the cross-language syntax target. Delete old wrappers/surfaces rather than layering a second API over them. The runtime probes do not implement native ownership transfer or interruption races.
4. Resume the milestone dependency order in [60](../final-solution/60-implementation-and-release-plan.md), and keep [50](../final-solution/50-audit-closure-matrix.md) and [70](../final-solution/70-test-and-release-gates.md) honest. The successor storage/protocol/query/migration/Lean/performance work is still substantial. No audit issue is globally closed here.

## Review and release boundary

The proposal still chooses a small core: canonical set data, LMDB, Free Join, and one checked native representation. Effect supplies language-level execution/lifetimes, not a second query engine, tenant registry, migration journal or protocol. The final correction removes one state machine rather than adding a feature: ordinary insert/delete effects rerun while the draft is building; its lifecycle alone guards consumption and failure. Repeated ingestion still consumes budget; immutable sealed commands are the retry unit for log.

This checkpoint includes all non-ignored tracked/untracked work requested by the owner, not ignored build caches, installed dependencies, temporary probes or credentials. No source fix, release promotion, real cloud operation or data rewrite is performed by the final review. CI on this branch should expose the unfinished compilation above. The criterion for 1.0 remains **all required gates actually passing**, not a pushed commit or an approved proposal.
