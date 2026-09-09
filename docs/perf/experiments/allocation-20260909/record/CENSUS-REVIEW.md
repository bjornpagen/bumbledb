# Allocation site census — baseline engine, repaired fixture

Completed 2026-09-09 02:42:49 UTC, session 33201 exit 0. All 54 windows and
all printed site rows were reviewed. Engine source remained b02a641e; the
test-only Account.id key declaration and normal fixture smoke test are saved
in `census-retry-1/source-patch.log`. The original failed `census/` remains.
Frozen executable SHA-256:
`73ce2a7808a30172087db79e7a1ed5cc085c3f6dde91c21bb76b05fcb2012d65`.
Matching packed-symbol UUID: `6C9D0ABC-8265-3E7F-A249-26C05E66BE8F`.
Zero dropped attribution events. This was allocation attribution, not another
full CPU trace. Both full-trace entry points remain disabled.

## Interpretation

The recorder's own backtraces allocate, so global counters in attributed
windows include substantial measurement overhead. SITE rows record the outer
requests only. Summing the printed rows **including the aggregate remainder**
gives their recorded totals below, not a retained-heap/RSS estimate. Realloc
sizes count the full requested new size. These totals include caller work
within each window, and do not establish allocator physical traffic.

| Attributed operation | Recorded outer requests | Requested bytes |
|---|---:|---:|
| create fixture | 279 | 20,432 |
| reopen fixture | 251 | 17,604 |
| prepare 4-atom, 2-condition, 2-rule chain | 712 | 118,210 |
| prepare recursive query | 511 | 82,117 |
| insert one posting | 33 | 1,989 |
| insert 512 postings (one-row calls) | 4,283 | 139,593 |
| small 5-parent edit | 193 | 6,030 |
| overwrite eight determinants | 77 | 3,618 |
| cold join | 78 | 115,866 |
| cold Pack | 71 | 21,685 |
| cold Allen query | 72 | 32,716 |
| cold recursive query, cap 164 | 97 | 388,780 |
| first join after commit | 14 | 81,362 |
| dynamic insert 10k | 62,238 | 3,180,565 |
| dynamic scan 10k | 10,001 | 720,002 |
| typed insert 10k strings | 41,690 | 2,216,372 |
| typed scan 10k strings | 1 | 2 |

Every one of the 12 unattributed warmed execution windows records zero
requests/frees/bytes. These windows are **inside an already-open snapshot**;
the 24-byte WorkContext owner in the 66-family public-operation counters was
created outside them. The two diagnostics agree, not contradict one another.
The dense-key allocations in the separate closed-domain benchmark scenarios
are not directly site-confirmed by this different census schema.

Unattributed cold sum/count requests 970,264 bytes in 55 events, with 410,224
bytes relinquished (including old realloc layouts). That window is over the
same already-built images after earlier queries, not an independent cold DB.
Unattributed recursive caps 110/120/140 request 27,565 / 55,556 / 186,705 bytes
in 66 / 76 / 88 events. Fresh query owners are created outside each execution
window; shared relation images may already be cached. Do not compare those
global totals to the attributed cap-164 global counters.

## Owners and priorities

1. **P1 remains the first CPU experiment.** Broad saved CPU traces put repeated
   sibling routing/probing on the expensive r4 path, whose relevant source is
   unchanged. Neither warm counter pass nor this census supports a per-row
   allocation rewrite there. Test adjacent tagged-cursor run reuse while
   retaining the existing full-batch prefetch pass. Predict less setup, not
   fewer warm allocations; correctness and negative controls decide acceptance.
2. **M1 remains open, not quantified as a win.** Cold join SITE rows include
   68,896 bytes for two image column buffers, 14,336 bytes of COLT u32 pool
   growth, 8,348 bytes of filter survivors and 8,192 bytes for a u64 pool.
   Post-commit join requests 67,328 image bytes, 8,352 filter bytes and a
   4,800-byte result resize, with no listed COLT pool growth. Pool reuse is
   working; source-confirmed duplicate-triggered growth and dead construction
   lengths need pool-level live-length/capacity fixtures, not claims from RSS.
3. **Recursive/result storage is a material cold owner.** At cap 164, six
   WordMap key-table requests total 258,048 bytes; dense iteration-index growth
   adds 31,744. The full caller path goes through the recursive result sink.
   This is result/history set storage, not automatically needless duplication.
   Preserve fixed-point distinctness and delta semantics before any change.
4. **Prepared-state scratch is visible.** The 4-atom/2-rule preparation has
   23,520 bytes in ViewMemo occurrence-vector growth plus separately owned
   executor key/hash/binding arrays. Full query schemas/DNF topology matter;
   ordinary preparation requests scale from 23,090 to 259,579 bytes in the
   audited chain cases. This is not warm per-call memory and does not justify
   a quota or capped-memory architecture.
5. **Output contracts matter.** Dynamic scan allocates exactly one 72-byte
   three-Value owned row per fact, plus a two-byte storage range boundary.
   Typed borrowed scanning has only that range boundary, with no per-row
   request. Do not call owned output a gratuitous engine allocation. Dynamic
   insertion's leading 960,000 bytes are caller-side `rows.clone()`; typed
   insertion also constructs its 240,000-byte input vector inside the window.
6. **New M3 lead: repeated owned storage range boundaries.** Both 10k inserts
   show two sets of 10,000 18-byte requests at `DataTree::prefix_iter`, line 103,
   where `heed::Database::range` owns its bound. The existing prefix copy is
   already a fixed stack array. Avoid replacing that array with another buffer
   scheme; investigate whether the underlying cursor API can avoid per-seek
   bound ownership while preserving the custom physical comparator, error
   propagation and transaction lifetime. This is a source/site-supported open
   lead, not a selected fix or a reason to outrank the expensive CPU path.

The census does not qualify the 512 MB Pi, nor exhaust the saved traces.
No full trace, release, or publication is authorized by these results.
