# PRD 01: Paper Gap Inventory

## Purpose

Convert all known mismatches between Bumbledb, Rosetta, and the Free Join paper into explicit implementation obligations before optimization starts.

## Current Gaps To Record

Create a tracked inventory document or test fixture that lists every gap below with owner PRD number and status.

## Required Gap Entries

- Paper uses bag semantics in examples; Bumbledb must keep set semantics.
- Paper assumes full conjunctive queries internally; Bumbledb exposes projection but must execute over full bindings and deduplicate projected facts.
- Paper assumes selections pushed to base tables; Bumbledb currently converts some predicates to source filters but applies them after relation images are available.
- Paper `GHT::iter` returns an iterator; Bumbledb currently returns `Vec<EncodedTuple>`.
- Paper `iter_batch` is true batching; Bumbledb currently chunks a fully materialized vector.
- Paper COLT stores offsets into column-oriented base data; Bumbledb stores per-cell `Vec<u8>` values in `Vec<Vec<u8>>` columns.
- Paper lazily builds tries only when needed; Bumbledb builds source offset lists and can force maps eagerly through cover choice and iteration.
- Paper starts from optimized binary plans; Bumbledb currently uses deterministic generated candidates and simple scores.
- Paper factors lookups conservatively; Bumbledb has factoring but must prove lookup order and movement decisions with traceable plan rewrite events.
- Paper dynamic cover selection chooses the source with fewest keys; Bumbledb often uses offset counts as an estimate.
- Paper emphasizes vectorized execution; Bumbledb public default is scalar.
- Paper implementation is main memory; Bumbledb adapts to LMDB snapshots and must make that adaptation explicit.
- Paper discusses materialization bottlenecks; Bumbledb still decodes and materializes in sink paths in ways that can dominate small result queries.
- Paper does not address durable storage accelerators; Bumbledb has namespaces but must ensure accelerators are optional for correctness.
- Current explain says timings and allocations are not collected; this suite must make that false.

## Required Implementation

- Add `docs/free-join-performance-hardening-prds/PAPER_GAP_INVENTORY.md` or equivalent during this PRD.
- Each entry must include `gap`, `risk`, `target_prd`, and `acceptance_signal`.
- Each later PRD that closes a gap must update the inventory.
- The inventory must distinguish product rejections from implementation gaps. Bag semantics are rejected, not a gap to close.

## Passing Criteria

- The inventory exists and mentions every required gap above.
- Every gap has exactly one target PRD or is marked `product-rejected` with Rosetta justification.
- The inventory states that x86 SIMD is product-rejected for this suite.
- `rg "TODO|TBD" docs/free-join-performance-hardening-prds/PAPER_GAP_INVENTORY.md` returns no matches.
- Global acceptance from PRD 00 passes.
