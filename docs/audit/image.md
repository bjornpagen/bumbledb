# Image and view correctness audit

## Scope (files and docs read, with line counts)

Paper (algorithmic authority):
- `docs/free-join-paper/arXiv-2301.10841v2/main.tex` (162) and every `.tex` it inputs:
  `tex/00-abstract.tex` (15), `tex/01-intro.tex` (243), `tex/02-background.tex` (510),
  `tex/03-free-join.tex` (608), `tex/04-optimizations.tex` (478), `tex/05-eval.tex` (337),
  `tex/06-discussion.tex` (85). (`07`/`08` are empty and not input; `025-tale.tex` is not input.)

Architecture docs (product authority), in order:
- `docs/architecture/README.md` (71), `00-product.md` (186), `10-data-model.md` (227),
  `20-query-ir.md` (178), `30-execution.md` (295), `40-storage.md` (205),
  `50-validation.md` (179), `60-api.md` (120).

Audited files:
- `crates/bumbledb/src/image.rs` (695)
- `crates/bumbledb/src/image/cache.rs` (450)
- `crates/bumbledb/src/image/view.rs` (576)

Supporting code read to trace call paths end-to-end (not audited exhaustively, but every
interaction with the three files above was traced):
- `crates/bumbledb/src/encoding.rs` (488) — canonical encodings, `FactLayout`
- `crates/bumbledb/src/storage/read.rs` (418) — `scan`, `row_count`, width checks
- `crates/bumbledb/src/storage/env.rs` (excerpts) — snapshot-sourced `generation()`
- `crates/bumbledb/src/storage/commit.rs` (excerpts) — tx-id advance, eviction input
- `crates/bumbledb/src/storage/dict.rs` (excerpts) — `SENTINEL_ID`, mint assert, lookups
- `crates/bumbledb/src/storage/delta.rs` (excerpts) — pending-intern mint assert
- `crates/bumbledb/src/api/db.rs` (excerpts) — commit → `evict_older_than` wiring
- `crates/bumbledb/src/api/prepared.rs` (excerpts) — `bind_param`, `resolve_filter`,
  `resolve_selection`, `run_join`, the view memo
- `crates/bumbledb/src/exec/kernel.rs` (285) — NEON/scalar filter kernels
- `crates/bumbledb/src/exec/colt.rs` (1201) — the View consumer (`word_at`, `select`, `reset`)
- `crates/bumbledb/src/exec/run.rs` (excerpts) — residual word comparison
- `crates/bumbledb/src/ir.rs` / `ir/validate.rs` / `ir/normalize.rs` (excerpts) —
  `CmpOp::compare`, `cmp_legal`, filter lowering
- `crates/bumbledb/src/plan/fj.rs` / `plan/selectivity.rs` (excerpts) — `split_filters`,
  the `distinct()` consumer

`cargo test -p bumbledb --lib image::` run: 18/18 pass.

## Verdict

The image/view/cache subsystem is correct against its documented contract. Decode is
byte-faithful for all six types (words are `from_be_bytes` of the canonical encoding, so
u64 word order is value order for U64 and biased I64, and intern-id equality is value
equality); positions are dense scan ordinals independent of row-id holes and cross-checked
against the `S` counter in both directions; the distinct counter is an exact set count for
every column shape including empty and all-duplicate columns; the cache's
`(relation, snapshot-sourced generation)` keying makes a generation-mismatched read
unrepresentable, the two-builder adoption race converges on one Arc of provably identical
content, and the double-checked eviction re-check closes the re-insert-after-evict leak;
`apply`'s kernel-pivot and scalar paths are compaction-correct and were traced against the
conjunction semantics for every FilterPredicate × Const combination; ordered comparison on
intern words and enum/bool bytes is unreachable through the validation roster; and the
u64::MAX miss sentinel yields exactly the documented per-operator semantics (Eq empty via
short-circuit, Ne matches every stored row) because both mint paths assert it is never
issued. I found no wrong-result, data-loss, or crash-on-valid-input bug. The findings
below are one corruption-handling shape gap (panic where a typed error is documented) and
three documentation-truth defects.

## Findings

### [LOW] Image build trusts the stored row count for slab sizing before verifying it

`crates/bumbledb/src/image.rs:192-204`. Documented invariant: "Corrupt data is a hard
error, never a skip" (`40-storage.md`) — corruption surfaces as a typed
`Error::Corruption`, and runtime query errors are enumerated as `Overflow`/`Corruption`
(`60-api.md`). `build` reads `row_count` from the `S` counter and immediately computes
`vec![0u64; word_cols * (row_count + SET_STRIDE/8 + LINE/8)]` (and the analogous byte
slab) before anything cross-checks the counter against the `F` scan. Concrete failure
scenario: an `S` value corrupted to a huge number (e.g. `0xFFFF_FFFF_FFFF_FFFF`; the code
path exists — `read::row_count` accepts any 8-byte value) makes the multiplication
overflow (panic in debug; silent wrap in release producing an undersized slab) or drives
a multi-exabyte allocation into an abort. In the release-wrap case the fill loop's
bounds-checked indexing or the post-loop `position != row_count` check still stops the
build — no wrong results can escape, because the image is never constructed — but the
failure surfaces as a panic/abort instead of the documented typed corruption error, and
the panic poisons the cache mutex for every later reader if it fires inside
`get_or_build`'s build window (it does not hold the lock, so in practice only the calling
query dies). Fix direction: range-check `row_count` (against `usize::try_from` plus a
sanity ceiling, or use checked arithmetic) and return
`Corruption(CorruptionError::MalformedValue("S row count"))`-class errors on failure.

### [NOTE] `get_or_build`'s doc comment is fused onto `peek`; `get_or_build` is undocumented

`crates/bumbledb/src/image/cache.rs:89-117`. Documented invariant: README rule 5 / product
success criterion 4 ("docs stay true"). The doc block preceding `pub fn peek` begins
"Returns the image of `rel` at the reader's generation, **building it outside the lock on
a miss**…" — the exact opposite of peek's actual never-builds contract — and only then
continues with peek's real description; `get_or_build` itself carries no doc comment at
all. Concrete failure scenario: rustdoc renders peek with a description promising builds;
a future caller (e.g. a prepare-time statistics path) trusts it and expects a build, or a
maintainer "fixes" peek to match its docs, reintroducing the prepare-time build the
selectivity ladder explicitly forbids (`30-execution.md`: "prepare never builds"). Fix
direction: split the two doc blocks at the `pub fn peek` boundary.

### [NOTE] `Const::PendingIntern` doc overstates the miss semantics (Eq-only claim stated unconditionally)

`crates/bumbledb/src/image/view.rs:22-25`. Documented invariant: `20-query-ir.md` — "Miss
semantics are per operator: … an `Eq` use matches nothing (and may short-circuit the query
to empty, **the only case where that is sound**) while an `Ne` use matches every stored
value." The `Const` doc comment says "a dictionary miss means the query is empty on this
snapshot, so the evaluator never sees one." The second half is true (verified:
`resolve_filter` at `api/prepared.rs:1084-1112` replaces every `PendingIntern` before
`apply` runs — Eq-miss short-circuits, non-Eq miss becomes `Word(SENTINEL_ID)`), but the
first half is only true for Eq anchors; under Ne a miss does not empty the query. Concrete
failure scenario: a maintainer reading this comment "optimizes" resolution to short-circuit
on any miss, silently emptying `memo != $missing_string` queries that must return every
row. Fix direction: reword to "an `Eq` miss empties the query; any other operator resolves
the miss to the sentinel id."

### [NOTE] The named cold dual-output builder exists only under `#[cfg(test)]`

`crates/bumbledb/src/image/view.rs:315-339`. Documented mechanism: `40-storage.md` — "on a
cold relation with a filtered query, one *storage* scan produces both the cached unfiltered
image and the query-local survivor view." Production never calls `build_with_filters`; the
live cold path is `cache.get_or_build` (one storage scan) followed by `apply` over the
decoded columns (`api/prepared.rs:956-958`), which satisfies the doc's substance — the
storage scan happens once, and the doc's own parenthetical says the filter is "a second
pass over the decoded in-memory columns." No behavioral divergence and no double storage
scan; this is naming drift only: the function whose doc comment claims to *be* the
40-storage mechanism is test-only, and the mechanism's real reader is the
`get_or_build`+`apply` composition. Fix direction: either delete `build_with_filters`
(its test can compose the two calls) or note in `40-storage.md` that the dual output is
the composition, not a dedicated builder.

## Checked and sound

- **Decode fidelity, all six types.** Word columns store
  `u64::from_be_bytes(canonical bytes)`: U64 → the value (big-endian is order-preserving,
  `encoding.rs:99`), I64 → the sign-flipped biased word (`encode_i64` = `value ^ 1<<63`
  big-endian, so unsigned word order equals signed value order — verified by trace over
  `{MIN, MIN+1, -1, 0, 1, MAX}` and by the in-repo ordering tests), String/Bytes → the
  big-endian intern id (injective by dictionary construction). Byte columns store the raw
  byte after a *validated* decode: `decode_bool` rejects anything but 0x00/0x01,
  `decode_enum` range-checks against the variant count — corruption aborts the build,
  never a skip (`image.rs:252-260`, test `scan_corruption_aborts_the_build`).
- **Position density under row-id holes.** Positions are the scan enumeration ordinal,
  incremented per yielded fact; `read::scan` iterates the `F` prefix in row-id order and
  deleted rows are absent keys. Verified against the delete-then-rebuild test
  (`positions_stay_dense_under_row_id_holes`) and by reading the scan cursor.
- **RowCountMismatch, both directions.** A scan yielding more rows than `S` errors before
  writing out of bounds (`image.rs:231`); fewer rows errors after the loop
  (`image.rs:267`). The `F` value width is checked per row inside `scan` itself, so the
  8-byte field slicing in the fill loop can never see a short fact.
- **Slab sizing and stagger arithmetic.** Worst-case per-column placement slack is
  alignment (< LINE/elem) plus 127 residue steps of LINE bytes: 2047 words < the
  2064-word per-column budget; 16383 bytes < the 16512-byte budget — no out-of-bounds
  write is reachable at any column count (induction over `place`'s cursor). The residue
  walk advances the L1D set-stride slot by exactly 1 per step, so 128 iterations visit
  all 128 residues; the >128-column fallback reuses residues but never overlaps ranges.
  One shared `ResidueStagger` covers both slabs using absolute addresses, so no two
  columns of a relation are congruent mod 16 KiB (asserted by the 12-column test). Column
  starts are element indices into a Vec that never reallocates after the address is read,
  and moving the Vec into the `Arc` does not move the heap buffer.
- **DistinctCounter exactness.** Capacity `next_pow2(2·max(row_count,1))` bounds occupancy
  at ≤ 50 %, so linear probing always terminates; the probe compares stored words, so hash
  collisions cannot double-count; `occupied` is cleared per column (`slots` is stale-safe
  behind it); empty columns count 0; the 256-slot byte table is trivially exact; intern-id
  injectivity makes word distincts equal value distincts for String/Bytes. The counter is
  built only after the row-count cross-check has passed, so its sizing input is the true
  count.
- **Cache generation keying is airtight.** The key generation comes from
  `ReadTxn::generation()`, which reads the `_meta` storage tx id *inside the reader's own
  snapshot* (memoized per txn in a `OnceCell`). The tx id advances exactly once per
  state-changing commit inside the same atomic LMDB txn as the data (`commit.rs:285-292`),
  and a no-op commit writes nothing at all (including pending interns, which are dropped),
  so two snapshots at equal generation are byte-identical in `F`/`S`. Therefore a cached
  image can never differ from what the keyed reader's snapshot would decode: a
  generation-mismatched observation is unrepresentable.
- **Two-builder adoption.** Both racers hold snapshots at the same generation ⇒ identical
  `F` bytes and `S` count ⇒ the deterministic build produces identical row order, column
  contents, and distinct counts (internal slab offsets may differ; nothing exposed
  differs). Insert-if-absent under the single mutex means the loser adopts before
  returning — asserted by the two-thread convergence test.
- **Eviction and the re-insert race.** `evict_older_than(report.new_generation)` runs
  after a successful changed commit, serialized under the writer mutex, with `newest`
  monotonic via `max`. `get_or_build` re-checks `generation < inner.newest` under the
  insert lock, so a builder racing a commit cannot resurrect an evicted generation (the
  leak the comment names); the pre-build `newest` snapshot is safe because `newest` only
  grows. Readers pinned at older generations keep their `Arc`s (map removal only drops
  the map's reference) and rebuild query-locally without polluting the map — all three
  behaviors pinned by tests. A commit failure never advances `newest` (eviction is only
  wired on `Ok` + `changed`).
- **peek vs get_or_build consistency.** `peek` returns exactly the map entry at the
  caller's snapshot generation or `None`, never builds, never consults `newest`; prepare's
  statistics ladder (its only production reader) treats `None` as "degrade to bounds" —
  consistent with `30-execution.md`'s "prepare never builds".
- **apply: conjunction correctness for every FilterPredicate × Const combination.**
  Resolved constants entering `apply` are `Word`/`Byte` only (traced through `bind_param`
  and `resolve_filter`; `Param`/`PendingIntern` are substituted before execution, and the
  `unreachable!` arms in `row_matches` are genuinely unreachable through the public
  execution path). Word constant on word column and byte constant on byte column are the
  only type-legal pairs (validation), and `CmpOp::compare` over `Ord` evaluates each of
  Eq/Ne/Lt/Le/Gt/Ge faithfully in word/byte space, which is value space for every legal
  combination. `FieldsCompare` requires identical structural types (repeated in-atom
  variable, or same-atom var-var comparison), hence identical column kinds — the
  `unreachable!` mixed arm is unreachable.
- **Kernel-pivot refinement.** The pivot search only accepts predicates whose kernel
  actually ran (all `false` returns write nothing into `buf`); `survivors_only` is exactly
  the single-predicate case; the refine-in-place cursor write (`buf[cursor] = buf[read]`,
  `cursor += keep`) maintains `cursor ≤ read` so no unread survivor is clobbered, and
  `truncate(cursor)` drops exactly the failures; the pivot is skipped by index, and every
  other predicate — including earlier non-kernel ones — is evaluated per survivor.
  Boundary translations are exact: `Lt 0 → empty`, `Le c → [0,c]`, `Gt MAX → empty`,
  `Ge c → [c,MAX]`, Ne has no kernel shape and byte kernels accept only Eq (Bool/Enum are
  equality-only anyway). Range semantics on biased I64 words are order-faithful, so e.g.
  `x < 0` lowers to word range `[0, biased(0)-1]` correctly.
- **Scalar fallback compaction.** `resize(row_count)` then unconditional store /
  conditional advance, truncate — equivalent to the naive filter (matches the kernels'
  scalar reference bit-for-bit by the in-repo property tests, lane boundaries 0/1/15/16/17
  included).
- **View::All vs Survivors equivalence; empty predicates.** Empty predicate list returns
  `All` (asserted); a Survivors view containing every position is observationally
  identical to `All` through the entire consumer surface (`len`, `position_at`,
  `image()`, COLT root iteration and force). `recycle` round-trips survivor buffers;
  zero-residual occurrences always carry empty buffers, so no capacity is lost.
- **u32 position space.** Every position cast is `try_from(...).expect(...)` (apply,
  kernels, NEON tails) — a >2³²-row image panics loudly rather than truncating silently;
  unreachable under the 10⁷ scale axiom by ~2.5 orders of magnitude.
- **Ordered compares on intern words / enum bytes are unreachable.** `cmp_legal`
  (`ir/validate.rs:87-94`) admits Lt/Le/Gt/Ge only for U64/I64, for var-constant,
  var-param, and var-var (incl. same-atom → `FieldsCompare`) alike; roster tests pin
  rejection for String, Bool, and Enum. Enum equality compares the raw ordinal byte on
  both sides (or both widened to words in selections/slots) — value-faithful; enum
  variant-list equality is required for the two sides to unify at all.
- **Sentinel intern id under Ne, traced end-to-end.** Mint paths assert
  `id != u64::MAX` (`delta.rs:107-110` for the production pending-intern path,
  `dict.rs:83-86` for the test intern path), so no stored word can equal the sentinel.
  A missed String/Bytes param binds `Const::Word(u64::MAX)` flagged `missed`; a missed
  pending literal resolves the same way. Eq uses are all routed into selections by
  `split_filters`, where a miss short-circuits the whole conjunctive query to empty
  (sound: a conjunction with an unsatisfiable Eq is empty). Ne stays a residual filter;
  `row_matches`/`compare` evaluates `stored_word != u64::MAX` = true for every stored
  row — exactly the documented complement semantics. The view memo keys on the resolved
  filter (sentinel word included), so two distinct missing strings share one
  correct-for-both view.
- **View memo generation coherence.** `run_join` reads the generation once from the
  execution's own snapshot and binds every occurrence at it; a memo hit requires exact
  (generation, resolved-filters) equality, and generational immutability (no-op commits
  write nothing) makes a memoized view valid for its whole generation. Placeholder
  bindings (`generation: None`) can never false-hit; stale-first-then-LRU victim choice
  and the park/rebuild swap were traced for state consistency.
- **Executor word semantics downstream.** COLT `word_at` widens byte columns to u64, and
  selection resolution widens `Const::Byte` identically, so trie probes compare like with
  like; residual evaluation compares binding-slot words with `CmpOp::compare` under the
  same type discipline as filters.
