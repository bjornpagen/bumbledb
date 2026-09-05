# Convergence status — F0 published, twelve-lane frontier launched

Baseline: dirty `codex/bumbledb-1-0`, HEAD `4a0573692431ae7b2f8ef82d8663bd199a4058da`, 2026-09-05. Preceding swarm (`c0fb388e`) is ended (last compiler-repair agent aborted). No concurrent writers. Dirty work preserved.

**Accepted qualification evidence: none. Release: unqualified.**

## F0 declarations (coordinator, 2026-09-05)

Exact spellings published in owning modules. Temporary noncompilation of consumers is expected. No dummy-success bodies.

| Contract | Spelling | Module |
| --- | --- | --- |
| C1 descriptor/witness | `ProjectionInternKey`, `DistinctnessWitness`, `CompiledTheory::{intern_key,source_projection,target_projection,distinctness_witness,full_row_witness}` | `schema/compiled.rs` |
| C4 judgment | `LawfulParent`, `judge_complete`, `judge_incremental` (requires `LawfulParent`) | `schema/judge.rs` |
| C2 charged owner | `DecodedRow` borrow/`into_owner` only — `into_values`/`into_parts` deleted; `ChargedBytes::into_owner`; `ScratchWriteBatch` | `canonical.rs`, `work/owners.rs`, `exec/scratch.rs` |
| C3 generation | `GenerationState`, `GenerationHandle`, `ResolverView` | `work/cache.rs` |
| C4 owned read / stage | `OwnedRead<S>`, `ReadFrame<'read, S>`; `UnreadyStore::populate` (no `store()`/`disarm`); `InstallOutcome` | `api/db/read_instance.rs`, `storage/store/staging.rs` |
| C8 delivery | `DeliveryTicket` (commit after native output registration; abort drops preview) | `api/prepared/result.rs` |
| C6 roots/walk/receive | `RecoveryRoot::{checkpoint_only,suffix}`; `OBJECT_REF_WIRE_BYTES=49`; `ChainVisitor`; existing `TransportContext`/`ReceivingStore` | `manifest.rs`, `store.rs`, `history/locator.rs`, `store/receive.rs` |
| C7 capability | `ResourceHeader`, `CloseDrain`, `Capability { runtime, worker, kind, id, generation }` | `ts/crate/src/runtime/registry.rs` |
| C1/C8 scalar/chain | `ScalarLeafScope`; `CompiledChainInput` (every snapshot mandatory) | `ts/src/scalar.ts`, `ts/crate/src/migration_wire.rs` |
| C8 lock | `RepositoryLock` = existing `DirectoryLock`; `acquire_repository_lock`; no stale unlink | `store/fence.rs` |

Coordinator hubs remain: root/child manifests, `lib.rs` exports, `schema.rs`, packet, `PROMPT.md`.

## Ready queue

| Lane | Scope | State |
| --- | --- | --- |
| L01 | Compiled projections/witnesses | ReadyForIntegration |
| L02 | Complete/incremental judgment/evidence | ReadyForIntegration |
| L03 | Charged owners/exact scratch | ReadyForIntegration |
| L04 | Generation-owned cache/text | ReadyForIntegration |
| L06 | Pack/aggregate sinks | ReadyForIntegration |
| L08 | Publication certainty/local parent | ReadyForIntegration |
| L09 | Root/codec/walker/GC | ReadyForIntegration |
| L11 | Real transport/kernel exclusion | ReadyForIntegration |
| L12 | Fixed-worker resource ownership | ReadyForIntegration |
| L15 | Shared scalar/query authoring | ReadyForIntegration |
| L17 | Effect log/generated repository | ReadyForIntegration |
| L21 | Gates/packaging/permanent docs | ReadyForIntegration |
| L05 | Query/derived/results | ReadyForIntegration |
| L07 | Core read/work/staging/LMDB | ReadyForIntegration |
| L10 | Bounded lifecycle/migration | ReadyForIntegration |
| L13 | Native core/delivery/drafts | ReadyForIntegration |
| L14 | Native log/chain/repository lock | ReadyForIntegration |
| L16 | Effect core | ReadyForIntegration |
| L18 | Public app/SDK specimens | ReadyForIntegration |
| L19 | Lean/refinement | ReadyForIntegration |
| L20 | Benchmark/storage/constant inputs | ReadyForIntegration |

## Steering (2026-09-05 13:46) — source-reviewed blockers

ReadyForIntegration does not establish correctness. Preserve landed contracts; do not restart. No overlapping writers.

| Item | Blocker | Owners this turn |
| --- | --- | --- |
| 1 | Live ticket + abandoned output not traced as one path | L12 reclaim + L13 `publication.accept` + L16 cancel-without-take |
| 2 | `supervise` keeps queued Page/Rows after Effect abort; abandoned output retained | L12 reclaim + L16 Effect drain (test-only instrumentation) |
| 3 | TextEq → 4-arg `holds` → memo stamp not traced as one path | L04 store + L05-text memos/`string_field` + L03 `ScratchTextLookup` |
| 4 | Filename `is_unpublished_staging_path`; full receipt-key Vecs in `project_unready` / `seal_new_incarnation` | L08 delete filename gate; L10 bounded batches; L07 staging |
| 5 | Gratuitous blank lines between consecutive fields/variants; fragmented comments | Each idle owner on exclusive files only — no rustfmt-as-fix, no comment/string rewrite |

Coordinator marks Integrated only after producer → consumer → failure → cleanup and predecessor deletion.

## Steering (2026-09-05 13:36) — production paths, not compilation

Preserve landed contracts (`on_work`, `ResidentAdmit`/`open_nonresident`, `SCRATCH_TOKEN_TAG`, `read_stream`/`import_stream` `AsRef`, charged restore chunks, one `DeliveryTicket`). Do not restart the architecture. Coordinator marks Integrated only after tracing producers, consumers, failure cleanup and predecessor deletion. No tests/builds/typechecks until the source barrier.

| Item | Obligation | Owner this turn | Held off L05 files |
| --- | --- | --- | --- |
| 1 | One execution-wide text equality at the resolver; bind memos to owner/epoch | L04 producer; L05-text consume after current L05 pass | `execute.rs` drops `nonresident` while `param_word_memo` / `ResolveMemo.arena_ranges` retain tokens |
| 2 | Stream spill; no whole-stage `pending` Vec | L03 visitor + L06 aggregate/computed sinks | `reach.rs` `seal_projection_scratch` / `seal_scratch_range` / `append_scratch_range` stay L05-derived |
| 3 | Restore stays private until the final incarnation is complete | L10 + L07 unready writes + L08 no `materialize` escape | `complete_install` today runs before `finish_new_incarnation` tip/genesis |
| 4 | One cancel/output/cursor transition at `dispatch_payload_message` | L12 runtime + L16 interrupt at that boundary | Local `QueuedOutput` ≠ `operation.output` |
| 5 | Real batch + reserve before copy | L13 `transfer_from_payload` | `into_cursor(1)` / `preview_page` encode-before-fit stay L05-delivery |
| 6 | D08/D09 match original obligations | L21 packed-consumer only this turn | `gates.rs` is L05-delivery |

**L05 split (in flight — exclusive files, no overlap):**
- **L05-text:** `text.rs`, `bind.rs`, `fallback.rs`, `resolve_memo.rs`, `answers.rs`, `execute.rs`, param memos in `prepared.rs`
- **L05-derived:** `reach.rs`, `derived.rs`, `run_join.rs`, `source.rs`, `computed.rs`, `exec/run/**`, `exec/dispatch/**`
- **L05-delivery:** `result.rs`, `result/tests/**`, `tests/gates.rs`

Open seams:
- L15: `Scalar.add(Scalar.field("units"), Scalar.u64(1n))` constructs unresolved AST; hub exports `ScalarNode`/`ScalarLeafScope`/`ScalarResultKind`. Not Integrated: L14 bind, L16/L17 consumers. Verification NotRun.
- L01: interned descriptors; `fingerprint_routing` hashes `ProjectionId`; `DeterminantTable::compile` returns `Result`. L07 `store_env.rs` now takes `compile(schema)?`. Not Integrated: L05/L07 consumers of interned descriptors and `visit_projection` still need tracing. Verification NotRun.
- L09: `read_stream<B: AsRef<[u8]>>` + charged walk held. `lane_gc.rs` / `lane_erase.rs` consume `get_verified(..., ctx) -> ChargedBytes` (`as_bytes` / `into_owner()`). L10 already updated `adversarial-process.rs` / `gate-baseline-ports.rs`. Not Integrated: not traced walker → import → native restore. Verification NotRun.
- L08: Filename readiness deleted (`is_unpublished_staging_path` gone). Ready-only = admitted `Db` + committed authority attachment; dest name ignored. Unready cannot call `materialize` (`compile_fail`). Post-check I/O failures stay `Local`, not “nothing installed.” `status_hosted(..., work)` + `submit_certain` held. Not Integrated: L10 must keep hydrate on unready (bounded-receipt pass in flight); do not wrap unready as `Db` to call `materialize`. Verification NotRun.
- L03: `ScratchTextLookup` published — one-env `TextForward`/`TextReverse` (slots 4–5) plus a charged cache admitted only after `ScratchAppend::finish`. Failed put is not a `get_*` hit. Hub exports `ScratchTextLookup`. `on_work` / `OrderLog` / `ScratchAppend` held. Not Integrated: L04 must drop uncharged `by_text`/`texts` and the second relation (bounded-dictionary pass in flight). Verification NotRun.
- L06: `ScratchAppend` / `OrderLog` / fallible `stream_*` held. Sink-test `apply` now passes `image.generation().text_eq(None)`. Hygiene journals deleted. L05-derived `reach.rs` consume request is stale. Not Integrated: not traced visitor → stream → seal. Verification NotRun.

**C2 resolution (coordinator):** Pack claims use `ScratchClaimKey` only (no `0xFE` mode word). `ScratchWideClaimKey` is the colliding-wide exact-key spelling, not Pack mode. One `ScratchEnv` with named maps is the selected substrate so a claim cursor and header `get` share one environment. RAM word tables charge through existing `ChargedBuffer`/working envelope — no second word-allocator type.
- L02: Incremental containment/capacity walk interned groups via `visit_compiled_group`. D26 + `LawfulParent::established` (`pub(crate)`) held. Hygiene: no field/variant blanks to remove; fragmented wraps repaired; stream-and-filter production journals deleted. Not Integrated: missing `source_projection`/`target_projection` on an ordinary side is an L01 intern gap; not traced judge → L07 `CandidateFacts`. Verification NotRun.
- L11: `get_verified(..., ctx) -> ChargedBytes` (work required; cap = `ctx.receive.max_bytes.min(reference.length)`). `get_verified_ctx` and 3-arg uncharged `Vec` are deleted. Production reads are only `receive_*` → `ReceivedBody`/`ReceivedHead`. Lock inode never unlinked. L09/L10 test consume of 3-arg `get_verified` is done. L10 `duty::AnyStore` now implements `receive_object` / `receive_head`. Not Integrated: real S3/IAM NotRun. Verification NotRun.
- L21: Packed-consumer has no D08/D09 stand-ins. L05-delivery replaced misleading `gates.rs` D08/D09 (retained WorkingBytes; derived-pipeline under `set_sink_ram(0)`). Packet still present; G15 blocked until writers freeze **and** post-retirement candidate. Do not start G15. Do not retire the packet. Verification NotRun.
- L17: JS `productionExclusion` table deleted. Generate cancel runs `joinPendingIo` (`Effect.ensuring`) then lock release. L16 `release` now clears `slot.owner` after join — L17 request consumed. Full chain + `compiledMappings`, unresolved convert, original-identity interrupt held. Keep `ensuring(joinPendingIo)` before `lock.release`. No public lock API. Not Integrated: not traced with L12 publication cancel probe. Verification NotRun.
- L14: Charged restore chunks + stamped lock + `submit_certain` + `assemble` + `status_hosted(..., work)` held. Hygiene: obsolete `to_vec` restore journals and deleted `logSnapshotClose` header removed; field/variant lists already tight. L16/L17 lock-Effect request is stale. Not Integrated: hosted restore/migrate still C08; not traced with L10 import + L16 scoped join. Verification NotRun.
- L12: `PublicationSink::accept(output, commit)` registers `operation.output` and commits the live ticket as one locked transition. Abandoned Page/Rows reclaim on cancel/close without JS take. L13 live-pull consume request consumed. Collect/draft/finish still ignore the sink; `run_payload_publication` then calls no-op `accept_publication` / `reject_publication`. Not Integrated: not traced accept → reclaim → L16 cancel-without-take. Verification NotRun.
- L04: Exact scratch maps + charged 64 KiB warm alias cache. Tokens are `TAG|dense` only. `TextStoreEpoch` is a full owner `u64`. `holds`/`apply` take `TextEq`. `RowWords` marks String columns (`string_field` / `field_is_string`); `ImageRow` and the L04 test provider already did. L05-text `ScratchRow` / `BindingOps` consume request consumed. Not Integrated: not traced store → TextEq → L05-text memos. Verification NotRun.
- L07: `host_scan_batch` / `delete_host_batch` + `HostWindow`/`HostResume` published. L10 receipt cleanup now loops `delete_host_batch`. Readiness is `AdmittedStore` after `admit`. Not Integrated: not traced batch delete → admit → install. Verification NotRun.
- L10: Tip-check → inspect digests → `write_host` (binding/genesis) → L07 `delete_host_batch` / `HostResume` loop → one `complete_install`. `duty::AnyStore` is a `ReceivingStore` (`receive_object` / `receive_head`); library verbs take it directly; `with_store!` unwraps gone. No receipt `Vec`, no `Cancelled` scan-stop. Settlement-class post-install failures. Filename readiness not restored. Not Integrated: not traced unready batch → admit → install → L08 authority gate. Verification NotRun.
- L16: Aborted Effect completions call `runtimeCancel` and do **not** `take`. `runtimeArmPublicationCancel` + same-cursor retry held. Lock cleanup-before-acquire + one-close `release` held. L12 abandoned-output reclaim request consumed (native producer). Not Integrated: not traced L12 `PublicationSink` → L16 cancel-without-take → close. Verification NotRun.
- L13: Live pull keeps the original `DeliveryTicket` and commits only inside `publication.accept`; accept `Err` aborts that same ticket. Every `submit_payload` job is 3-arg. Collect/draft/finish ignore the sink. `accept_publication` / `reject_publication` are no-op fallbacks after live accept/abort. No re-preview; `parked_slots` / `pending_slots` gone. Not Integrated: not traced with L12 reclaim + L16 cancel-without-take. Verification NotRun.
- L20: Compact 13-cell scorecard held. Collision/f64/admission correspondence lives in `bumbledb-bench` (`C-D04-collision-bytes`, `C-D19-*`, `C-G03-*`); three-way cargo tests removed from `lean.sh`. Oracle is `judge_final_state`, not the planner. Not Integrated: L21 must census the seven `C-*` ids from `correspondence::OWNED_CASES` and must not treat `lean.sh` as a cargo-test owner; historical benches still omit `WorkContext`; G15 blocked. Verification NotRun.
- L18: Specimens call `InvariantRejected`, `db.snapshot(&work)`, `Db::close() -> CloseReport`, generated `{ manifest, plans, snapshots }`. Claim-only Scalar/JSON reconstruction deleted. No hub manifest edits. L07 close/`InvariantRejected` and L17 unresolved convert requests consumed. Not Integrated: Rust `CloseReport` has no `Failed` arm (TS still has `kind: "failed"`); generated artifacts absent until F3 (`loadGeneratedMigrations` fails honestly); `coreProgram` still needs L21 `ManagedRuntime`. Verification NotRun.
- L19: Chapter-60 proofs remapped to `judge_complete` / `judge_incremental(LawfulParent)` / `judge_final_state`. Correspondence in `lean/correspondence.md`. Independent oracles stay `judge_final_state`, `staged.rs`, `history_model.rs` — not the production planner. `LawfulParent::established` stays `pub(crate)`; do not restore empty-prepare. Not Integrated: L21 must take Bridge + correspondence + `lean.sh` / `spec-census.sh` as permanent scope (G03/G04/G07 + D04/D05/D19/D26); L20 owns three-way/cargo tests removed from `lean.sh`. `lake build` NotRun. Verification NotRun.
- L05: L05-text fallible `tokens_equal` / `holds`. L05-delivery abort is ticket-local; `encoded_lens` gone. L05-derived `stash_aggregate` uses L06 `admit_dest` + `stream_finalize`; `apply` is `Result<View>`. COLT force/growth is `Result<_, WorkError>` — no sentinel `0`; L05-derived propagates `Err` at every consume site (`bind` before force/select). L05-plan `SealedRow` marks String columns and uses 4-arg `holds`; resolver `Err` is not a dropped id. Not Integrated. Verification NotRun.
- **Canonical `DeliveryTicket` (one contract, no native twin):** `open` → `preview_page` (charged preview, no advance) → `adopt` (convert under admitted overlap, no commit) → register native `QueuedOutput` → `commit` (then advance). `abort` = resource refusal, position unchanged, retry same row, no data. Backing failure stays failed (never EOF). Post-work checkpoint must not discard a page after consuming its rows. Delete `inspect`/`copy`/`is_terminal`/`fail_closed` as a second API — those behaviors live on the names above.
- Integration counterexamples (not Integrated): L12 recursive `lane_send` under `runtime.state` and `RefCell` reborrow in `drop_closing_entry`; L16/L17 register lock cleanup before interruptible acquire (prefer deleting redundant bookkeeping); L04/L05 scratch-backed text resolver on the production beyond-memory path; replace claim-only gates (`type_name`/`size_of`/fn-ref) with consumer counterexamples. Do not assign Integrated from declarations or summaries.
- Coordinator assigns Integrated only after tracing producers, callers, failure cleanup and deletions.

## Known release prerequisites

- Close all source counterexamples, including D24–D29.
- Transfer 68 audit IDs, 220 child behaviors, 78 review IDs and D01–D29 into permanent contracts.
- No product checks until the final reviewed source barrier.
- Real S3/IAM and Graviton remain unqualified until actually run.
- Retire the packet only after permanent transfer; qualify the post-retirement candidate before the sole final commit/push.
