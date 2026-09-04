# Scope, provenance, and validation record

Audit date: 2026-09-04. This document distinguishes what was read, what was run, and what remains unverified.

## Audited state

- Workspace: `/Users/bjorn/Documents/bumbledb`.
- Git HEAD: `dc078489c98777600e92cbc49590219e0e2f6122`.
- Findings target the **working tree**, not that commit alone. The tree already contained uncommitted 0.20.3 version/lock changes, log key-grammar/store changes, TypeScript-log build/package changes, and publishing-script/manifest changes.
- Workspace version during the audit: 0.20.3. The root workspace excludes `ts/crate` and `crates/bumbledb-c`; a workspace Rust pass is therefore **not** a fresh Node/C build or test pass.
- Rust: `rustc 1.99.0-nightly (d453bdd8f 2026-08-14)`, selected through the repository's pinned `nightly-2026-08-15` toolchain.
- Node: v26.4.0, darwin-arm64. Existing native artifact: `bumbledb-node 0.20.3 (bumbledb storage format v8)`.
- Local filesystem case-alias behavior was observed on this Mac's scratch filesystem. Do not generalize that trigger to every filesystem.

Line references are one-based references to the audited working tree and can move after edits. The stated function and failure path should be used with the line number. External harnesses were not committed as executable source; their exact source is preserved inside Markdown.

## Review method and coverage

The main review integrated philosophy, architecture, deployment, performance and assurance. Three parallel reviewers inspected engine/query semantics; log/storage/checkpoint/lease behavior; and SDK/FFI/tenant hosting. Follow-up passes challenged severity, causal assumptions, documentation claims, and untested consequences rather than merely adding more findings.

The review prioritized:

- Current `bumbledb-log` and `ts-log` state transitions, especially pending state, publication, retention, cleanup, and lifetime.
- Admission/canonical-value boundaries, fresh identity persistence, snapshots, compaction, rejection diagnostics and data lifetime.
- Query output errors, prepared state, materialization and resource limits.
- JavaScript aliasing and asynchronous interleavings; native resource ownership; tenant borrows and cache identity.
- The relationship between current code, historical thesis, Lean premises, test gates, package scripts and deployment examples.

Approximately **920 tracked Rust/TypeScript/Lean files and 276,374 lines** were counted, including tests, formal sources, and benchmarks. This was not a line-by-line review of all of them. It was a broad, multi-reviewer audit focused on high-leverage invariants and targeted counterexamples. Neither code-generation origin nor confident comments were treated as evidence of correctness or incorrectness.

Historical design context was consulted read-only using Nessie: the July 10 context was read in full; the beginning/recent tail and selected portions of the later large conversation were consulted, not exhaustively read. Those materials helped recover intent and distinguish the original single-process thesis from later replication scope. They influenced the philosophical synthesis in [01](01-philosophy.md), but current implementation and observed behavior grounded findings. No external context was created or updated.

## Existing validation gates executed

Commands below ran from the repository root unless another directory is specified. Results are tool-output records, not benchmark claims.

| Gate | Command / location | Result | Boundary |
| --- | --- | --- | --- |
| Workspace Rust tests | `cargo nextest run --workspace --locked --no-fail-fast --status-level fail --final-status-level fail` | **2,049 passed; 30 skipped; 2 slow; exit 0**. Run summary: 505.293 seconds | Default selected features, root workspace lock; not every feature/platform combination or separate Node/C crates |
| Lean and three-way conformance | `scripts/lean.sh` | Lean build completed, 44 jobs; **277 conformance cases, 0 disagreements**; three-way comparator test passed, exit 0 | Abstract semantic/algorithm and finite conformance evidence, not verification of Rust or crash/storage mechanisms |
| Formatting | `cargo fmt --all --check` | **Passed, exit 0**, independently run | No formatter writes |
| Dependency-lean log build | `cargo check -p bumbledb-log --no-default-features --locked` | **Passed, exit 0** | Compile check, not backend/runtime qualification |
| Workspace static lint | `cargo clippy --workspace --locked --all-targets -- -D warnings` | **Passed, exit 0** | Workspace excludes separate Node/C crates; no all-features claim |
| Selected TypeScript log suites | Explicit source-condition test invocation from `ts-log`, preserved in [32](32-sdk-test-evidence.md) | **132 passed, 0 failed, 0 skipped** | Current TS sources with existing native artifact |
| Selected engine SDK / Node-FFI suites | Explicit source-condition test invocation from `ts`, preserved in [32](32-sdk-test-evidence.md) | **77 passed, 0 failed, 0 skipped** | Not a fresh native build or C ABI test |

The separate Lean lane may overlap semantic coverage of the workspace tests; do not add all these counts into a purported number of independent proofs. Its build emitted a nonfatal unused-`ρ` warning at `lean/Bumbledb/Query/Denotation.lean:439`. The external engine harness emitted its own warning; that is recorded separately in [22](22-engine-test-evidence.md), not attributed to the clean workspace Clippy run.

The S3 bucket environment required by credential-gated tests was checked and was unset. **No real AWS operation or cloud fault test was performed.** A test that returns early without configured credentials can appear passed; it is not evidence that an S3 operation succeeded. The 30 nextest skips likewise must not be interpreted as successful checks.

## Targeted probes

| Campaign | Actual execution | Evidence preserved | Important limitation |
| --- | --- | --- | --- |
| Rust replication | Ten asserted scenarios: stale retired slot, incomparable floor, two GC paths, ambiguous IDs, lower-ID retry loop, reachable scratch, delayed backlink, filesystem alias, tenant cache leak | [11 — Full manifest, source, commands, captured output](11-replication-test-evidence.md) | Some schedules construct deliberate valid states; ambiguity uses injected transport result; no real AWS; pause/crash fencing cases remain static |
| Engine/query | Seven finding observations, including ten concurrent compaction copies and one abrupt-exit child | [22 — Exact harness, manifest, outputs](22-engine-test-evidence.md) | Harness prints anomalies rather than asserting them; own dependency lock; no power-cut or resource-exhaustion test |
| SDK/hosting | Nine inline probes, including controlled lease clock and two-process cache reuse | [32 — Exact invocations, output, test commands](32-sdk-test-evidence.md) | Existing native artifact, no rebuild; isolated one-off runs, not stress qualification |

External Rust projects:

- `/tmp/bumbledb-replication-audit.xjDrVs`.
- `/tmp/bumbledb-engine-audit.6iFaKq`.

Each depended on the repository's current path crates but resolved its own Cargo lock. For example, the replication harness used blake3 1.8.7. These are **not** represented as executions under the root workspace lock. The workspace-locked baseline is the separate nextest/check/Clippy evidence above.

Temporary database and build artifacts may remain in those locations and the SDK probe directories listed in [32](32-sdk-test-evidence.md). The audit did not delete them. Continued existence is not required to read the preserved evidence. Reproductions must use fresh disposable fixture directories, never application databases.

## What was not done

- No implementation fixes, source/test edits, commits, package publication, external messages, or deployment changes.
- No full build/pack/publish battery: package scripts can regenerate artifacts and alter manifests. Selected non-source-mutating gates were used instead.
- No fresh Node bridge or C library build; no C sanitizers, Miri campaign, cross-platform binary matrix or ABI fuzz campaign.
- No physical power cuts, disk-controller fault tests, real process-suspension filesystem fencing campaign, cloud credentials/permission changes, actual S3 timeout injection, cross-region tests or restore from a real backup.
- No new performance benchmark, AWS price estimate, production load trace, customer database inspection, or validation of all historical benchmark raw data.
- No claim of exhaustive query optimizer correctness, exhaustive parser fuzzing, comprehensive adversarial security review, or independent verification of the cited literature.

Static findings retain their static label. A proposed consequence is not promoted to reproduced just because a related primitive was exercised. Contract gaps are not silently counted as accepted-state corruption. Conditions such as custom codec implementation, premature ID escape, same-schema cache reuse, or case-insensitive filesystems remain attached to their findings.

## Change hygiene

All audit deliverables are Markdown under the new `audit/` directory. The initial dirty workspace was preserved, including existing deleted/modified key-grammar files, publishing manifests, and untracked TypeScript-log build support. Builds and probes created their normal ignored or temporary artifacts but did not intentionally regenerate shipped source or package contents.

The final workspace status was compared with the recorded initial set of user changes. No additional tracked-source changes were introduced by the audit. There was no cleanup of user work and no commit.

## Using this evidence later

Treat the reports as a dated record, not evergreen current-code truth. When a fix lands, append its commit, regression names, supported environment, and closure reasoning to a resolution record. Keep the original counterexample accessible. If a finding is rejected, preserve the evidence that rejects it. The [test campaign](51-test-campaign.md) and [roadmap](60-roadmap.md) provide the next verification steps; they are not claims those steps have already passed.
