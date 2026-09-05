# Swarm execution — 21 lanes, one integrated outcome

The target is the owner’s next orchestrated Grok Fast run. “Swarm” means at least **12 concurrent execution workers, excluding the coordinator**, after F0. Use more useful capacity when available; do not pad with idle reviewers. A model name is not evidence of correctness. Fixed instructions and verified integration carry the work.

No product tests, builds, typechecks, lint/format checks, package install/build hooks, benchmark runs or interim commits until the final source barrier. Author discriminators concurrently. Read-only inspection, diffs and proposal-link checks are permitted. Final checks include repairs and reruns, not one attempt.

## F0 — short architectural/declaration cut, coordinator only

Inspect the current branch, dirty changes and all active writers. Do not reset or overwrite work. This packet assumes the preceding swarm is quiescent before implementation; do not race two swarms. Recheck source findings, mark already-fixed mechanisms for preservation, but retain their acceptance obligations.

Publish these concrete source declarations/ownership rules using C1–C8. This is a small interface cut, not a serial implementation phase:

- Compiled projection ID/descriptor/witness accessors; full-state and lawful-parent incremental judge entry points.
- Inseparable charged owners and transaction-scoped scratch writes; generation owner handles and borrowed resolver views.
- Owned core snapshot plus borrowed per-operation read frame; private stage population/final-admission/install result.
- Result delivery ticket/commit/abort contract and admitted native output owner.
- Worker-routed capability/resource header, worker-table payload ownership and guaranteed close-drain signal.
- Checked root/DecisionRef and streamed walker interface; contextual transport receive contract.
- Shared scoped scalar AST with unresolved source fields; mandatory compiled-chain input including every snapshot.
- Internal repository lock acquisition/drain using existing kernel directory exclusion; no stale-file recovery.

Choose exact Rust/TS spellings and fields before workers alter consumers. Declarations may temporarily fail to compile as implementations land; no dummy successful implementations, alias shims or duplicate transitional API. Put them with their owning source module, not in a new interface dump. If a real source constraint contradicts a selected contract, resolve it once here and update affected packet sections; do not ask every worker to invent an alternative.

The coordinator owns root exports/manifests/locks and cross-lane signature disputes. Once F0 declarations are published, all listed lanes have substantive work available. Consumers need not wait for producer internals to be green.

## Exclusive ownership roster

Paths are repository-relative. A path prefix includes its descendants unless excluded. Adjacent inline tests belong to the source owner. New test files stay beside their consumer and are assigned before creation; no fixtures directory. The full lane instruction gives its integration-test carveouts.

| Lane | Exclusive production writes | Outcome / declaration consumers |
| --- | --- | --- |
| L01 | `crates/bumbledb/src/schema/compiled.rs`, `storage/store/det_index.rs` under that crate | Real shared law/probe projections → L02/L05/L07 |
| L02 | `crates/bumbledb/src/schema/judge.rs`, `schema/judge/**`, `schema/evidence.rs`, `storage/store/judge_bridge.rs` | Complete versus incremental judgment, portable evidence → L07/L08/L14 |
| L03 | `crates/bumbledb/src/work.rs`, `work/**` except work/cache.rs; `canonical.rs`, `canonical/**`, `exec/scratch.rs`, `exec/scratch/**` | Charged rows/buffers and exact scratch → L02/L04/L05/L06/L10/L13 |
| L04 | `crates/bumbledb/src/image.rs`, `image/**`, `work/cache.rs` | Generation-owned images/resolver → L05/L07/L12/L13 |
| L05 | `crates/bumbledb/src/api/prepared.rs`, `api/prepared/**`, `plan.rs`, `plan/**`, `exec/**` except scratch.rs/scratch/**/sink.rs/sink/** | Bounded Free Join/derived/fallback/results, delivery tickets → L06/L07/L13 |
| L06 | `crates/bumbledb/src/exec/sink.rs`, `exec/sink/**` | Bounded exact sinks, streamed Pack → L05 |
| L07 | `crates/bumbledb/src/api/db.rs`, `api/db/**`; `storage/store.rs`, `storage/store/**` except det_index.rs/judge_bridge.rs | Owned read/public work/staging/LMDB → L05/L08/L10/L12 |
| L08 | `crates/bumbledb-log/src/admin.rs`, `apply.rs`, `certainty.rs`, `writer/**`, `identities.rs` | Evidence and coherent local transitions → L10/L14/L17 |
| L09 | `crates/bumbledb-log/src/history/**`, `manifest.rs`, `gc.rs`, `local_roots.rs` | Checked refs/one traversal/retention → L08/L10/L11 |
| L10 | `crates/bumbledb-log/src/recovery.rs`, `restore.rs`, `backup.rs`, `checkpointer.rs`, `migration/**`, `bin/**` | Bounded private lifecycle and transforms → L14/L17 |
| L11 | `crates/bumbledb-log/src/store.rs`, `store/**` | Contextual actual transport and kernel exclusion → L08/L09/L10/L14 |
| L12 | `ts/crate/src/runtime.rs`, `runtime/**`, `runtime_wire.rs` | Fixed-worker resource ownership → L13/L14/L16 |
| L13 | `ts/crate/src/db_wire.rs`, `db_wire/**`, `marshal.rs` | Core operations/drafts/delivery consume owners → L14/L16 |
| L14 | `ts/crate/src/log_wire.rs`, `log_wire/**`, `migration_wire.rs`, `log.rs` | Native log/chain/repository-lock boundary → L17 |
| L15 | `ts/src/scalar.ts`, `query/**`, `query.ts` if present, `fields.ts`, `spec.ts` | Shared pure typed/scoped authoring → L16/L17 |
| L16 | `ts/src/**` except L15’s paths and index.ts | Effect core ownership/canonical boundary/internal seam → L17/L18 |
| L17 | `ts-log/src/**` except index.ts | Thin Effect log and generated repository transaction → L18 |
| L18 | `examples/**`, root README.md, ts/README.md, ts/COOKBOOK.md, ts-log/README.md | Honest public usages and packed consumer specimens → L21 |
| L19 | `lean/**`, scripts/lean.sh, scripts/spec-census.sh, scripts/spec-gen.py | Current proof/refinement correspondence → L21 |
| L20 | `crates/bumbledb-bench/**`, performance scripts named below | Useful benchmarks/storage/constant qualification inputs → L21 |
| L21 | `docs/reference/**`, `.github/workflows/**`, `.config/**`, other scripts/**, ts/scripts/**, ts-log/scripts/** | Current gates, packaging, exact evidence, permanent handoff |

Coordinator-only hubs: root/child Cargo.toml, Cargo.lock, package.json and package lockfiles, rust-toolchain.toml, crates/*/src/lib.rs, `crates/bumbledb/src/schema.rs`, `ts/crate/src/tags.rs`, `fingerprint_lock.rs`, `ts/src/index.ts`, `ts-log/src/index.ts`, root PROMPT.md and this packet. Unlisted production files (including theory scalar kernels/macros) remain coordinator-owned until explicitly assigned once. Do not infer permission from a shared prefix in prose.

L20 script carveout: scripts/measure.sh, bench-night.sh, check-asm.sh, ramdisk.sh, flame.sh, flamediff.sh, flame.py, bench_viz.py, and their existing benchmark visualization inputs. L21 owns the remaining scripts, except L19’s three. Review existing fixture inputs by responsibility; if retained, move necessary inputs beside their consuming tests without creating a replacement fixture tree.

Shared test helper hubs `crates/bumbledb-log/tests/lane_support/**` and `migration_support/**` are coordinator-only. Integration test allocations are in lane chapters; no “all tests” ownership that overlaps them.

## Prove the ready frontier

Start these twelve together immediately after F0. None waits for a whole subsystem. Their write sets above are disjoint; each can implement against the selected declaration while another producer’s body is unfinished.

| Initial worker | Useful immediate work | Input published at F0 | Notify as soon as usable |
| --- | --- | --- | --- |
| L01 | Compile missing source/target descriptors and intern keys | C1 descriptor identity | L02/L05/L07 |
| L02 | Separate full judge, implement canonical bounded citations | C1 descriptors + C4 judgment entry | L07/L08 |
| L03 | Close DecodedRow escape; transaction-scoped scratch bookkeeping | C2 owner/visitor declarations | L04/L05/L06 |
| L04 | Put charge/resolver in generation/image owners | C2 charge + C3 generation shape | L05/L07 |
| L06 | Replace Pack resident dictionaries/all_claims with ordered scratch | C2 scratch key/visitor shape | L05 |
| L08 | Fix dispatch placement and coherent negative/local-control proof | C5 attempts + C6 walker signature | L10/L14 |
| L09 | Correct checkpoint roots/49-byte codec and unified walker | C6 checked refs | L08/L10/L11 |
| L11 | Remove whole-body receiving and preserve backend observations | C6 receive + kernel exclusion | L08/L10/L14 |
| L12 | Replace stack reactors with worker-owned resource tables | C4 owned read + C7 capability shape | L13/L14/L16 |
| L15 | Implement shared AST with unresolved field nodes | C1 scalar declaration | L14/L16/L17 |
| L17 | Kernel-lock-scoped generation and same-FD bounded I/O | C8 internal lock/chain boundary | L14/L18 |
| L21 | Fix provenance/retirement/gate sensitivity and harness ownership | C9 evidence identity | All/coordinator |

Immediately queue L05/L07/L10/L13/L14/L16/L18/L19/L20, or launch them too if useful capacity exists. L07 can implement the owned-read and private staging bodies while L02 completes judgment; L13 can remove native eager-page advancement while L05 completes its ticket. Do not wait for an entire “wave” to finish. On a finished/blocked lane, schedule the next ready one.

Producer-consumer cycles are **contract cycles**, already cut at F0: C2↔cache, core-read↔native, result↔delivery and scalar↔migration. Do not freeze a broken uncharged/whole-Vec interface to avoid talking. A signature conflict goes to the coordinator, who updates both consumers once.

## Dispatch and status protocol

Send each worker: the common dispatch preamble in its lane chapter, its entire lane section, the exact referenced C contracts, D cases and relevant finding sections. Do not rely on previous chat history. Workers read the actual named source before editing.

A worker handoff contains changed paths, produced contract declarations, every adapted caller, predecessor deletions, authored discriminator symbols, **verification NotRun**, and unresolved seams. It does not say “release-ready.” No per-agent report files or journals.

Use only these lane states in STATUS: Queued, Implementing, BlockedOn(named contract), ReadyForIntegration, Integrated. The coordinator alone assigns Integrated after tracing source. Qualified is reserved for the final evidence phase. “Source-complete with mandatory gaps” is not a state. Never bury an open contract behind “future tightening.”

The coordinator reviews each returned patch against the counterexample, not its summary. Keep every real incomplete consumer assigned. Use narrow follow-ups in the same lane when possible; do not launch waves of independent redesign.

## Mandatory integration journeys

The coordinator must trace each path, including the listed adverse branch, before opening the final check barrier:

1. **Create/restore → populate invalid/valid final sets → complete judge → install → reopen.** Invalid preexisting facts cannot borrow the incremental lawful-parent premise; nonempty-required valid targets work. L02/L07/L10/L14.
2. **Snapshot → prepare → large text/derived/Pack execution → complete result → multirow native pull → interrupted/refused delivery → retry/close.** Track each allocation/generation owner and cursor commit. L03/L04/L05/L06/L12/L13/L16.
3. **One-worker open/read/close with many idle snapshots; saturated queues and reachable JS tokens.** No parked worker, same-pool wait, global payload lock, counter-only drain or rejected cleanup. L07/L12/L13/L14/L16/L17.
4. **Sealed command → authority dispatch → lost response → concurrent receipt retirement → resolve → diagnostic failure.** Track stable identity and strongest evidence through Effect. L08/L09/L11/L14/L17.
5. **Checkpoint at nonzero tip → decode → suffix fetch → GC → relocated backup → fresh restore.** Stop at authenticated base; count exact fetch budget; no epoch probes or tail aggregation. L09/L10/L11/L14.
6. **Schema edit → symbolic source-field arithmetic → verified full chain → exclusive generator → manifest commit → initialize/migrate → packed application reopen.** No missing-snapshot shortcut, live-lock stealing or handwritten plan bytes. L01/L10/L14/L15/L17/L18/L21.
7. **Final permanent spec/inventory → retire packet → candidate digest → current-addon gates/real platforms → exact final commit.** No hidden input change after qualification. L19/L20/L21/coordinator.

## Stop conditions

Stop for a real scope/contract contradiction, unavailable required environment, or new authority needed for live data/publication. Continue independent in-scope work. Do not stop merely because the in-flight tree does not compile, and do not run checks early to produce comforting progress.

The packet is successful when its selected small machine and preserved obligations are implemented and qualified, not when every worker returns a confident paragraph. See [90](90-evidence-and-retirement.md) for the only final retirement/commit order.
