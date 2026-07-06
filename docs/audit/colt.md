# COLT and word-map correctness audit

Audit date: 2026-07-06. Auditor mandate: absolute correctness of the COLT (Column-Oriented
Lazy Trie) and the sink word maps, against the Free Join paper (§4.2 especially) and the
architecture docs.

## Scope (files and docs read, with line counts)

Paper (`docs/free-join-paper/arXiv-2301.10841v2/`):

- `main.tex` (162) and its inputs; read fully: `tex/02-background.tex` (510, the §2 CQ
  assumptions), `tex/03-free-join.tex` (608, GHT/plan/execution), `tex/04-optimizations.tex`
  (478 — §4.2 COLT lines 163–345 traced statement-by-statement against the code, §4.3
  vectorized execution, §4.4 dynamic covers); skimmed for assumptions: `tex/00`, `01`,
  `025`, `05`, `06` (`07`/`08` are empty).

Architecture docs, in order: `README.md` (71), `00-product.md` (186), `10-data-model.md`
(227), `20-query-ir.md` (178), `30-execution.md` (295), `40-storage.md` (205),
`50-validation.md` (179), `60-api.md` (120).

Audited exhaustively:

- `crates/bumbledb/src/exec/colt.rs` (1200) — every function traced.
- `crates/bumbledb/src/exec/wordmap.rs` (222) — every function traced.

Read to establish reachability of suspected hazards (context, not audited line-by-line):
`crates/bumbledb/src/exec/run.rs` (1451, the executor's iterate/probe/journal discipline),
`crates/bumbledb/src/api/prepared.rs` (the `run_join`/`ViewMemo`/`select` driver,
lines 380–990), `crates/bumbledb/src/image/view.rs` (577, `View::position_at`/`len`
immutability), `crates/bumbledb/src/plan/fj.rs` (trie-schema derivation and cover
validation, lines 300–380 and 464–515), `crates/bumbledb/src/exec/sink.rs` (WordMap
consumers). Sanity: `cargo test -p bumbledb --lib -- exec::colt exec::wordmap` — 19 passed.

## Verdict

No CRITICAL or HIGH finding. Iterate-then-probe over these structures enumerates exactly
the view's tuples grouped by level keys — force ingests each parent position exactly once
into exactly one child, chunk chains preserve counts, and the set-semantic sinks absorb
the deliberate duplicate-key yields of last-level unforced iteration, so there are no
omissions and no spurious tuples on any path reachable through the executor. The
selection-level prefix arithmetic is clean: every public API adds `selection_levels`
exactly once and all internal indexing is uniformly selection-inclusive — no off-by-one
was found after tracing every use of `level`, `selection_levels`, and `schema_columns`.
Open-addressing termination is proven by the growth-trigger arithmetic (a free slot
always exists, including arity-0 keys and the capacity minima), and `grow_map`/
`WordMap::grow` preserve dense insertion order, key bytes, and slot values (including
chunked-child `NodeRef`s) across doublings. What remains are latent contract fragilities —
the resume token's meaning is coupled to the node's forced state with nothing but
structure defending it, and two release-mode-silent debug-assert contracts — plus benign
allocation notes. The findings below are ordered by severity.

## Findings

### [MEDIUM] Forcing a node with an outstanding iteration token silently reinterprets the token

`crates/bumbledb/src/exec/colt.rs:437–452` (state dispatch in `iter_batch_at`), `:462–525`
(positions tokens), `:529–559` (dense map token).

**Invariant at stake:** `BatchToken` is documented only as "opaque resume token; start at
`default()`" (colt.rs:60). Its actual meaning depends on the node's state at each call: a
view index (Root), a packed `(chunk+2, offset)` with `EXHAUSTED = 1<<32` (Chunks), or a
dense-list index (Forced map). Nothing ties a token to the state it was minted under.

**Concrete failure scenario (API-level, traced):** node A is `Chunks { count: 100 }` at
its last (suffix) level. `iter_batch` yields 5 entries, returning
`token = (first+2)<<32 | 5`. The caller then probes the *same cursor* at the same level
(`get`/`get_prehashed`) — `probe_child_at` forces A. The next `iter_batch` call sees
`NodeState::Forced` and routes the chunk-packed token into `iter_map`, where
`dense_idx ≈ 8.6·10⁹ > m.len` yields `(0, token)` — the drain terminates and the
remaining 95 positions are silently omitted (wrong results, not a panic). The mirrored
case (Root token into `iter_map`) yields distinct keys where positions were expected —
duplicates/omissions both possible.

**Reachability today: none.** The executor (`run.rs:398–444`) probes only sibling
occurrences (plan validity forbids two subatoms of one occurrence per node, so the
iterated cover's Colt is never probed at the iterated cursor) and children *below*
yielded cursors; each occurrence's `(cursor, level)` advance in lockstep through the
plan, so a node is touched at exactly one level from exactly one plan node, and chunk
chains are immutable once their parent's force completes. `select` runs strictly before
the join. Verified by tracing every `iter_batch`/`get_prehashed`/`ensure_forced` call
site (`run.rs`, `prepared.rs`).

**Fix direction:** spend one token bit as a state tag (positions vs dense) and
`debug_assert` compatibility in `iter_batch_at`; or `debug_assert` in `force` that the
node being forced has no outstanding iteration (cheapest honest guard: assert in
`iter_batch_at` that a nonzero token's kind matches the node's current state).

### [LOW] `Cursor::Row` iteration ignores `max`

`crates/bumbledb/src/exec/colt.rs:427–435`.

**Invariant at stake:** "Copies up to `max` entries into the caller's buffers"
(colt.rs:390).

**Concrete failure scenario:** `iter_batch(Cursor::Row(p), level, default, keys, children,
0)` passes the undersized-buffer assert vacuously (`children_out.len() >= 0`), then writes
`children_out[0]` — an index panic on an empty buffer, or, with nonempty buffers, returns
`yielded = 1 > max`, violating the contract. Unreachable today:
`Executor::with_batch_size` asserts `batch > 0` (run.rs:217) and the tests use ≥1.

**Fix direction:** `if token.0 > 0 || max == 0 { return (0, token); }`.

### [LOW] `start()` without `select()` is debug-guarded only; the release failure mode is wrong results

`crates/bumbledb/src/exec/colt.rs:237–241` (`debug_assert!(self.selected)`), with the
reset re-arm at `:207–208`.

**Invariant at stake:** "select() runs before the join" — for a selection-bearing trie,
`start` after `reset` is the root *above* the selection prefix.

**Concrete failure scenario:** a future call path executes the join after `reset` without
`select`. In release, `start()` returns `Cursor::Node(NodeRef(0))`; the executor then
iterates the root at join level 0 = internal level `selection_levels`, reading join
columns over the *whole unfiltered view* — every Eq-constant predicate is silently
dropped and extra rows are emitted. Today the single driver (`run_join`,
`prepared.rs:970–981`) calls `select` on **every** occurrence every execution (empty key
lists included, which set `selected` and return the root), and a selection miss
short-circuits before the executor runs — so the contract holds at the one call site.
But a wrong-results failure mode deserves more than a debug assert.

**Fix direction:** a release `assert!` (cost: once per occurrence per execution — noise
against the join), or fold the flag into the type (`start()` consumed by `execute`
returning `Option<Cursor>`).

### [NOTE] Chunk token packing and the chunk-index sentinel wrap at 2³²-scale chunk counts

`crates/bumbledb/src/exec/colt.rs:521` (`(u64::from(chunk) + 2) << 32` overflows u64 when
`chunk = u32::MAX − 1`), `:726/:760` (`chunk_idx` reaching `u32::MAX` would alias the
`next` sentinel; the `try_from` panics only past 2³²). Requires ≥ 2³²−2 chunks ≈ 2.7·10¹¹
positions in one Colt — four orders of magnitude beyond the ≤10⁷-fact scale axiom
(`00-product.md`), and positions themselves are u32. Recorded so nobody trusts tokens
beyond the axiom; no action warranted.

### [NOTE] `position_matches` truncates via `zip`: a short key prefix-matches in release

`crates/bumbledb/src/exec/colt.rs:317–322`. `schema_columns[level].iter().zip(key)` stops
at the shorter side, so a key shorter than the level's arity passes as a prefix match —
a pinned-row probe (`Cursor::Row`) would then accept a tuple it should reject. Guarded by
`debug_assert_eq!(key.len(), self.arity_at(level))` at `:354` only. All in-tree callers
pass exact arities (selection probes pass 1-word slices; the executor slices
`probe_keys[k*sub_arity..(k+1)*sub_arity]` sized from the subatom, run.rs:437). A
`debug_assert_eq!(key.len(), self.schema_columns[level].len())` inside `position_matches`
itself (or `zip_eq` semantics) would localize the guard.

### [NOTE] Pre-probe growth counts appends as prospective inserts (both maps)

`crates/bumbledb/src/exec/colt.rs:660` and `crates/bumbledb/src/exec/wordmap.rs:69`. The
growth check runs before the probe, so an arrival whose key already exists (COLT: an
`append_child` position; WordMap: a pure lookup of an existing group/seen key) at the
threshold doubles the table without adding a key. Over-allocation only — never
under-allocation; the load invariants (<75% / ≤50%) hold a fortiori, so probe termination
is unaffected. Note the order itself is the *correct* one: the probe index is always
computed against the post-growth table (the probe-then-grow order would be the real bug —
verified absent in both files).

### [NOTE] `WordMap::grow` re-allocates the dense list on every doubling

`crates/bumbledb/src/exec/wordmap.rs:134–136`. `std::mem::take(&mut self.dense)` discards
the retained capacity and the subsequent `reserve` allocates fresh. Correctness is
unaffected (insertion order rebuilt exactly), and the churn is confined to sanctioned
growth before the zero-alloc fixpoint (`30-execution.md` protocol: post-warmup growth is
already a gate failure). Since the entry count is unchanged by a rehash, the list could
be rewritten in place.

## Checked and sound

- **Trie semantics (paper §3.1/§4.2):** `force` is a single pass ingesting each parent
  position exactly once into exactly one key's child (probe hit ⇒ `append_child`; miss ⇒
  new `Single` slot + dense entry); positions within a child are distinct because the
  parent's positions are distinct (set-semantic images; survivor views are strictly
  ascending). Iterate-then-probe therefore enumerates exactly the relation's tuples
  grouped by level keys — verified by trace and by the `get_and_iter_agree_with_a_naive_
  oracle` test's construction.
- **The suffix rule:** `is_suffix = level + 1 == schema_columns.len()` (colt.rs:438) is a
  *strengthened, direct* form of the paper's `is_suffix(vars, relation.schema)`: unforced
  iteration happens only at the last trie level, where duplicate key-words with pinned-row
  children are exactly the paper's bag-semantic leaf yield, collapsed by the set-semantic
  sinks (D2). At any non-last level, unforced iteration is forced first — required,
  because the fused `(key, child)` API cannot group children without a map. Neither
  direction loses tuples; the deviation is sound under `30-execution.md`'s set semantics.
- **Selection-level arithmetic, exhaustively:** `schema_columns` = selection singletons
  then join levels (colt.rs:172–177); the public/internal split adds `selection_levels`
  exactly once each in `get_prehashed`, `ensure_forced`, `iter_batch`, and test-only
  `arity` (colt.rs:254, 342, 381, 407); `select` walks internal levels `0..selection_levels`
  from the root; `arity_at`/`position_matches`/`iter_positions`/`iter_map`/`force`/
  `force_ingest` are uniformly internal. The executor deals in join levels only and its
  `level + 1` bookkeeping (run.rs:487, 495) matches the trie-schema-per-subatom derivation
  (fj.rs:339–345). No off-by-one anywhere.
- **Selection discipline:** `run_join` selects every occurrence every execution (empty
  key lists included), so `start()` is always fresh; a miss returns before the executor
  with the sink already reset (empty result — the documented Eq-miss semantics);
  contradictory selections fall out of a deeper probe miss with no special casing;
  zero selections ⇒ root (`selected` pre-armed by `new`/`reset`); an all-duplicates
  selection column ⇒ one key with a chunked child of `count = view.len()`; `Cursor::Row`
  pinning across selection levels is a per-level `position_matches` equality check.
  Repeated `select` across memoized executions re-probes already-forced maps in O(1) per
  param — the amortization contract holds.
- **Open-addressing termination at exactly-full boundaries:** COLT grows when
  `(len+1)·4 ≥ cap·3` *before* inserting, so post-insert load < 75% inductively; one
  doubling always suffices (`len+1 < 1.5·cap` after doubling since `len < 0.75·cap`);
  capacity is ≥2 always (count 0 ⇒ 2; the 16-clamp and `next_power_of_two` verified
  against the doc formula and both sizing tests) — so ≥ ¼ of slots are always empty and
  `probe_hashed`'s linear scan terminates. `force_ingest` can never probe an always-full
  map. Arity-0 maps: one key ever (`hash_words(&[])` constant, empty-slice compares
  well-defined at `keys_start` even when the key slab is empty), `len ≤ 1`, growth
  trigger unreachable. WordMap: grow-before-probe keeps load ≤ 50% with `max(8)` floor —
  same termination argument.
- **`grow_map`:** fresh slot/key/dense ranges at the slab tails; `copy_within` source
  (old slab) and destination (new tail) are disjoint by construction — and `copy_within`
  is memmove-safe regardless; re-probe walks the *old* dense range in insertion order
  while pushing the *new* range at the tail (reads always below `dense_start`, writes at
  or above), preserving dense order — pinned by the determinism test; slot values move
  verbatim, so `Slot::Node(NodeRef)` children survive (NodeRefs index `self.nodes`, which
  map growth never touches); the in-flight local `m` is the only map descriptor mutated
  and it is pushed to `self.maps` only after the force completes — no torn map is ever
  observable, and `grow_map` is unreachable for already-pushed maps (sole caller chain:
  `force → force_ingest → grow_map`).
- **BatchToken resume across all three paths, under interleaved probes of *other*
  nodes:** Root tokens index the `View`, immutable for the execution; Chunks tokens index
  chunk chains that are immutable once their parent's force returns (append_child runs
  only inside a force, and only on children of the map being built); dense tokens index a
  map's contiguous dense range, complete before the map is pushed and never extended
  after. Interleaved forces of other nodes only *append* to `nodes`/`chunks`/`maps`/
  `slots`/`keys`/`dense` — index-addressed pools keep every outstanding index valid across
  `Vec` reallocation (no pointers anywhere). Boundary tokens verified: exact-`max` stop at
  a chunk boundary resumes via the `offset == len → next/EXHAUSTED` path; `EXHAUSTED`
  (high half 1) is disjoint from start (0) and packed chunks (high half ≥ 2); Root token 0
  on an empty view returns `(0, 0)` and the caller's `yielded == 0` break is correct.
- **`append_child` bookkeeping:** singleton → chunk upgrade preserves the dense entry
  (slot index unchanged, contents upgraded in place); `len`/`next`/`count` maintained at
  `CHUNK_LEN` boundaries (len ≤ 64 fits u8); `count` equals total positions, matching
  `key_count`'s `Estimate` and `force`'s sizing input.
- **`key_count` labels:** never forces; `Exact` = forced map `len`, `Estimate` = position
  counts (Root: view len; Chunks: count; Row: 1 — conservative and admissible under the
  magnitude-first rule). `better_cover`'s magnitude-first/label-tiebreak semantics match
  `30-execution.md` verbatim.
- **`reset`/`recycle`:** pools cleared with capacity retained, single Root node re-pushed,
  `start`/`selected` re-armed exactly as `new`; the old view's survivor buffer ping-pongs
  through `View::recycle` into `memo.spare_buffers`; the watermark test pins shape-stable
  reuse. Stale cursors cannot cross a reset: the executor re-derives all cursors from
  `colt.start()` per execution.
- **`hash_words`:** one deterministic function on every path — force ingest, grow rehash,
  `select`, and the executor's phase-1 `hash_key` — so a key's hash is consistent between
  build and probe; same-hash-different-key chains are resolved by full-key comparison at
  every occupied slot; `grow_map`'s compare-free "first empty slot" re-probe is justified
  by key distinctness (insert-only, no tombstones).
- **WordMap coherence across clear→grow→insert:** `clear` is O(len) via the dense list,
  leaves stale key bytes unreachable behind `None` values (probe short-circuits on
  `is_none()` before any key compare — traced the stale-twin collision chain case);
  `grow` moves values losslessly via `Option::take` and preserves insertion order;
  `iter` during mutation is unrepresentable (`&self` iterator vs `&mut self` mutators —
  borrow-checked); zero-arity maps are one global group (pinned by test); the
  `groups`-map index pattern in `AggregateSink` (`next = groups.len()` before
  `get_or_insert_with`) is coherent with the accumulator layout.
- **Arity-0 levels, single-position relations, empty views:** gate atoms iterate one
  empty-key entry per position at the suffix and probe-force to a nonemptiness hit
  (both pinned by tests, colt.rs:1185 and run.rs:907); a single-position relation pins a
  `Cursor::Row` end-to-end; an empty view forces to a capacity-2 empty map whose probes
  miss and whose iteration yields nothing.
- **Executor-side reachability guarantees the structures lean on** (verified, not
  assumed): plan validity forbids two subatoms of one occurrence per node; covers bind
  exactly the node's new variables (fj.rs:497–506, with the rebind regression test);
  occurrence cursors and levels advance in lockstep through the journal discipline, so
  `force(node, level)` is only ever called at one level per node — the `iter_map`
  arity debug-assert and the trust-the-caller level parameter are safe under the
  descent discipline.

