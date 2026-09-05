# Current core findings — source, not test results

Baseline is the dirty tree described in [90](90-evidence-and-retirement.md). Paths below are relative to `crates/bumbledb/src/` unless explicitly rooted. No execution reproduction is claimed. Inspect named symbols before editing; preserve fixes that land meanwhile. All prior CORE-001–025 obligations remain in [50](50-audit-closure-matrix.md).

## CORE-003/004/005 — Charges still escape the payload

**Evidence:** `canonical.rs::DecodedRow` exposes owning `values` and `into_values`; `storage/store/judge_bridge.rs` uses that extraction in production. `work/owners.rs::ChargedBuffer` now reserves before growth, a real repair, but searches found no production allocation consumers. `exec/run/ledger.rs` accounts pool capacity after construction and can drop reservations while reusable capacity survives.

**Counterexample:** decode a large row, extract its boxed values, retain those values after the decode scope; the ledger refunds although the allocation is live. Alternatively force a first COLT growth under less than its required working allowance: a post-growth refusal did not bound peak allocation.

**Root/correction:** accounting has a different owner/lifetime from memory. C2’s private allocation owner moves through decode, judge, image, sink, result and conversion; reserve before growth and retain reusable pool charges. L03 produces the ownership primitive; L02/L04/L05/L06/L13 replace actual consumers.

**Delete:** charge-shedding extraction, documented-hazard-positive assertions, redundant unused charged wrappers, post-hoc pool admission. **Acceptance:** D01/D08, not only the ChargedBuffer unit test. Source-confirmed defect plus incomplete consumer cutover.

## CORE-006/013 — Cache map entry is not allocation ownership

**Evidence:** `image/cache.rs::Cached` holds an Arc image beside its charge; trim removes the charge while another Arc can hold the image. Closed images use an uncharged OnceLock. `work/cache.rs::GenerationLease` has no Drop decrement; `ImageCache::pin_generation` has no production acquisition found, only a cache test. Check-then-advance is unsynchronized with new pins. `image/nonresident.rs::NonresidentTextStore` has test constructors but no production activation found.

**Counterexample:** two prepared queries retain text-bearing images; trim the cache, admit a new resolver generation, then use the old image. The claimed lease does not protect this path. Numeric images can remain alive after eviction refunds their charge even without text.

**Correction:** L04 implements C3 generation-owned resolvers/images, charges inside shared allocations, synchronized rotation and weak idle memos; L05/L07/L12/L13 retain the real owners. Nonresident queries construct the bounded resolver. **Delete:** detached pin counters and cache-entry-only charges. D01/D02/D29 include concurrent rotation and retained old readers. Do not merely add the missing Drop.

## CORE-009/020/024 — Preserve transactional scratch repair, prove composition

**Evidence:** `exec/scratch.rs` now has pending-charge machinery; `schema/judge.rs` takes explicit JudgeScratch. These replace specific earlier bugs. They are not verified behavior. `exec/scratch/capability.rs` still needs enforced policy and production consumer review.

**Failure to exclude:** MapFull abort leaves charge or bucket sequence changes committed in RAM; retry consumes twice; equal-size overwrite exhausts a lifetime counter; failed setup adopts/removes someone else’s directory.

**Correction/owner:** L03 uses one exact scratch owner, transaction-scoped pending mutations and enforced disk/capacity policy; L02/L05/L06/L10 consume it without reflective error dispatch. **Delete:** TypeId/Any capability selection and duplicate scratch maps after use is replaced. D03 with actual retry and collision hooks. Classification: repaired source to preserve plus required unverified schedules, not a claim the old bug still exists everywhere.

## CORE-002/011/012/022/025 — Shared theory is real; missing consumers still matter

**Evidence:** `Schema::shared_compiled_theory` and `DeterminantTable` now share compiled metadata; `LawAdjacency::delta_local_skippable` is used by judgment. Preserve that work. Containment/capacity source access and planner/fallback witness consumption remain incomplete; prior STATUS explicitly deferred them.

**Counterexample:** a one-group source removal with many unrelated groups should judge affected capacity/coverage through compiled group access, not rescan an entire relation. A key-bound nonresident query should seek the same persisted projection. A pointwise match is not scalar uniqueness.

**Correction:** L01 finishes interned source/target projection descriptors, positional permutations and checked witnesses; L02 judges exact affected groups; L05 uses them in planner/fallback. **Delete:** raw-schema re-interpretation and blanket scans where the accepted access witness applies. D04/D10 inspect real persisted keys and visited rows; forced collisions must not change results. Structural incompleteness, not a promised universal O(delta) bound.

## CORE-021 — Canonical evidence must be selected, not cosmetically sorted

**Evidence:** `schema/judge.rs` now introduces `fact_sort_key`; preserve it. Every key/containment/capacity path still needs review for citation selection before truncation. `bumbledb-log/src/apply.rs::outcomes_agree` compares the committed evidence.

**Counterexample:** more offenders than the citation limit, opposite row insertion/remint order, same logical rejected command. Selecting the first physical rows then sorting cannot yield portable receipts.

L02 owns bounded canonical top-k selection and all violated statements; L08/L14 preserve exact replay/receipt semantics. Delete sequence-based selection where it survives, never the independent replay check. D05 checks bytes and cited facts across import, resident and spill. Classification: partly repaired, not qualified.

## CORE-015/016 — New AdmittedStore does not currently prove admission

**Evidence:** `storage/store/staging.rs::UnreadyStore::admit` builds an empty ChangeSet and calls `owner.prepare`. `candidate.rs::prepare` sets `changes: Some(changes)`; `judge_bridge.rs::SchemaJudge::judge` selects delta-local judgment; every untouched law is skipped. The public `UnreadyStore::store` accessor also exposes the ordinary store. During the rewrite, log replaced create_staged with begin_staged/StagedPopulation, but its new install_judged_store repeats the same empty-delta incremental call. UnreadyStore::disarm also returns Store + a bare cleanup path before the type establishes readiness.

**Counterexample:** populate a stage with two distinct tuples sharing a declared scalar key (or a violated capacity), then call admit with no delta. It can be admitted without checking those tuples. Separately, a legal nonempty-required schema must not be rejected during empty staging.

**Correction:** C4 complete judgment and capability distinction, including the new install_judged_store/disarm callers; L02 supplies the complete entry, L07 makes readiness unforgeable and installs absent-or-complete, L10/L14 adopt it on every lifecycle path. Install settlement must derive from the actual no-clobber operation, not `dest.exists()`. **Delete:** empty-delta full-validation trick, unready Store escape and ready-path population. D06/D26. Source-confirmed admission bypass.

## CORE-004/014/017 — Explicit work cutover is underway, not finished

**Evidence:** public Db create/open/read/write now take work; preserve the change. Native and ordinary Rust call sites still need adaptation. Map resize and typed/public snapshot APIs remain cross-lane seams.

L07 supplies one explicit-work public core API and owned read substrate; L05/L12/L13 adapt execution; L18 compiles actual usage at the final barrier. Keep transaction affinity and same-thread resize refusal bounded, never a MAX/year convenience fallback. Opening under a too-small explicit ceiling refuses rather than silently raising it. D07 plus actual >32 GiB data in qualification. Mechanical call mismatches are not compiler results.

## CORE-004/007/010/022 — Free Join partial repairs leave large paths

**Evidence:** final work flush now propagates; RAM-first SealedStage exists. Preserve both. `api/prepared/reach.rs` still rebuilds full recursive delta/accumulated resident images through refill/append-drained paths; fallback and retained COLT growth must consume the new contracts.

**Counterexample:** computed/aggregate stage → negation → linear recursion with text and a frontier larger than working memory. Spilling seen does not help if the next round recreates every row as a resident image.

L05 owns end-to-end Resident|Scratch sources, compiled seeks, true Continue/Stop/Error propagation, bounded no-output work and pre-u32-limit regime selection. L03/L04 supply actual bounded owners/resolver. Delete forced whole-image resurrection, per-operand decode allocations and ignored stop returns. D08/D09/D10. No second optimizer or universal speedup claim.

## CORE-023 — Pack correctness was repaired by defeating spill

**Evidence:** `exec/sink/aggregate/spill.rs` now uses explicit wide mode and stable tokens: preserve those logical repairs. But group tokens and exact keys remain in resident Vec/BTreeMap, and `finalize_spilled` gathers every claim into `all_claims` then sorts it.

**Counterexample:** many wide groups across flushes exceed RAM; a query first spills successfully, then reloads all claims and copied group keys into RAM at finalization. Reverse-start overlaps also must remain correct.

L06 uses L03’s exact group→stable token table plus token→group lookup and ordered (token,start,end) claims. Iterate one group’s maximal union without all-claims/all-groups materialization. Token assignment order need not equal logical group order for set output; canonical sorting when required is separately bounded. **Delete:** resident wide-token dictionaries and all_claims. D11 tests multi-flush reversed/adjacent/disjoint groups, raw 0xFE narrow data and forced collisions under memory limits.

## CORE-008 / TS-005 — Public pull is a larger transaction than one row

**Evidence:** `api/prepared/result.rs::next_page_with_work` now advances only after its own cap check, a useful repair. It still builds before admission and drops delivery charge on return. `ts/crate/src/db_wire.rs::cursor_pull` loops over one-row core pages, advances on early rows, then propagates a later failure while discarding the accumulated output.

**Counterexample:** two rows each fit pageBytes but not together; the first is consumed, the second fails the remaining allowance, and retry starts after the first despite no page being delivered. Cancellation between rows creates the same gap.

L05 supplies C8 delivery tickets/owned admitted output; L13 commits position only when the complete native page is registered, and L16 retains one-shot scoped Stream semantics. **Delete:** eager next-row advancement inside an uncommitted native batch and post-copy reservation. D12/D25 test actual addon RAM/scratch, retries, cancellation and terminal failure. No public cursor escape.
