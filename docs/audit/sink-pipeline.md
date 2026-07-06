# Sink and execute-pipeline correctness audit

Audited 2026-07-06 against the Free Join paper and the architecture docs as normative
contracts. Method: every suspected issue was traced with concrete values through the
code path before being reported; `cargo test -p bumbledb` was green throughout.

## Scope (files and docs read, with line counts)

Paper (all `\input`s of `main.tex`):

- `docs/free-join-paper/arXiv-2301.10841v2/main.tex` (162)
- `tex/00-abstract.tex` (15), `tex/01-intro.tex` (243), `tex/02-background.tex` (510),
  `tex/03-free-join.tex` (608), `tex/04-optimizations.tex` (478), `tex/05-eval.tex`
  (337), `tex/06-discussion.tex` (85); `tex/07-relatedworks.tex` / `08-conclusion.tex`
  are empty and commented out of `main.tex`. (`tex/025-tale.tex` (358) also read,
  though `main.tex` does not input it.)

Architecture docs, in order: `README.md` (71), `00-product.md` (186), `10-data-model.md`
(227), `20-query-ir.md` (178), `30-execution.md` (296), `40-storage.md` (205),
`50-validation.md` (179), `60-api.md` (120).

Audit targets, exhaustively: `crates/bumbledb/src/exec/sink.rs` (784),
`crates/bumbledb/src/api/prepared.rs` (2140).

Read in full or in the load-bearing parts because the targets' correctness depends on
them: `exec/run.rs` (1451, full), `exec/wordmap.rs` (222, full), `exec/dispatch.rs`
(637, full), `exec/colt.rs` (1200, all non-test code), `exec/explain.rs` (counters),
`plan/fj.rs` (`validate`, `derive_nodes`, `split_filters`, `check_selections`,
`provably_distinct`), `ir/validate.rs` (witness accessors, `check_finds`, roster),
`ir/normalize.rs` (literal→`Const` lowering, repeated-var handling),
`image/view.rs` (`View`, `apply`, `recycle`), `storage/dict.rs` (sentinel).

## Verdict

The set-semantics core is correct. The aggregate seen-set keys the **full** binding
slot array (all query variables), the group key is the non-aggregated finds in find
order in both `emit` and `finalize_into`, and the `distinct_bindings` elision proof
holds end-to-end: I traced that under unique coverage (variable-bound plus
Eq-filtered fields per occurrence) the executor cannot emit one binding twice, because
COLT enumerates each fact-path exactly once (force lands every position in exactly one
child; iteration walks dense lists or position lists without repetition) and unique
coverage makes distinct facts produce distinct bound words. The D2 subtree skip is
sound: `ProjectionSink` signals on every emit, but the executor unwinds only through
maximal trailing runs of nodes that bind no projected variable, and the aggregate sink
never signals. Sum/Min/Max/Count accumulate and finalize correctly at the boundaries;
empty input yields the empty set; the ViewMemo swaps its (colt, generation, filters)
triple atomically on every path including error paths; param-miss sentinel discipline
is airtight in all four consumers; `profile`/`explain` run exactly `execute`'s
semantics through passive counters. I found no CRITICAL or HIGH defect. The findings
below are one memory-retention doc contradiction in the memo's parked placeholders,
one panic-instead-of-error edge, and three notes.

## Findings

### [MEDIUM] Parked placeholder COLTs pin prepare-generation images for the prepared query's lifetime

`crates/bumbledb/src/api/prepared.rs:455-469` (`build_view_memo`), `:884-897`
(`ViewMemo::bind`, whose own comment says "zero-residual occurrences always land here,
so their parked slots stay untouched forever").

Invariant at stake: `40-storage.md` — "Steady-state process heap = LMDB's mmap + **the
newest generation's images** + per-prepared-query pools + a constant"; `30-execution.md`
bounds the memo at "four COLT high-waters per occurrence per prepared query" (COLT
scratch, not image slabs).

Concrete scenario: prepare a selection-only query (any query whose predicates are all
Eq — the common ledger shape) at generation T against a 100 MB relation. Each of the
occurrence's `PARKED_SLOTS = 3` `ParkedView`s is constructed holding
`View::All(Arc::clone(&image@T))`. Commit T+1, T+2, …: every execution takes the
rebuild-in-place path (active is stale; the park path requires an active binding at the
*current* generation), so the parked slots are never swapped, never reset, and never
victimized — the `Arc<RelationImage@T>` refcount never drops while the prepared query
lives, even after the cache evicts generation T. The same holds for residual-filter
occurrences whose filter values change only across generations. A long-lived
application holding N prepared queries retains up to one stale generation's worth of
each query's relation images per query — full-width column slabs, not scratch — which
can breach the 2 GB envelope the docs promise. Results are never wrong (placeholder
generation `None` can never hit).

Fix direction: initialize parked slots with an imageless/empty view, or have `bind()`
drop (or downgrade to empty views) parked entries whose generation is older than the
requested one — the doc already declares them "never hittable again."

### [LOW] Result byte-heap offsets panic past 4 GiB instead of erroring

`crates/bumbledb/src/api/prepared.rs:221-223` — `u32::try_from(start).expect("buffer
bytes fit u32 offsets")` (and the companion `"intern lengths fit u32"`).

Invariant at stake: `60-api.md`'s typed runtime-error taxonomy; `30-execution.md` says
resource limits are absent with *the OS* as backstop — an `expect` panic is neither a
typed error nor an OS kill.

Concrete scenario: a projection whose distinct String/Bytes payload exceeds 4 GiB in
one `ResultBuffer` (e.g., ~70 M distinct 64-byte strings — outside the scale axiom but
valid input) panics mid-finalize. Cell ranges themselves are `usize`, so this is purely
the memo's packing choice.

Fix direction: return `Error::Overflow`-style typed error, or document the panic as the
resource backstop.

### [NOTE] A failed execution leaves partial rows in the caller's buffer

`crates/bumbledb/src/api/prepared.rs:1116-1141` (`finalize`),
`crates/bumbledb/src/exec/sink.rs:219-229` (`finalize` range check). An `Overflow`,
`NonUtf8Intern`, or dictionary-corruption error propagates after complete prior rows
(and the torn row's prefix cells) have landed in `out`. `ResultBuffer::len()` floors,
so the torn row is invisible and `get()` stays in bounds; the next `execute` clears.
Correct, but the contract "ignore `out` on `Err`" is nowhere documented — worth one
sentence in `60-api.md`.

### [NOTE] ProjectionSink's unconditional `SkipSuffix` is safe only jointly with the executor

`crates/bumbledb/src/exec/sink.rs:80-93` returns `SkipSuffix` on *every* emit
(including the first, including duplicates). Soundness lives entirely in
`crates/bumbledb/src/exec/run.rs:504-515` (absorb at any `sink_relevant` node) plus
the covers-bind-exactly-the-new-vars rule. I verified the pair is correct — the skip
crosses only nodes binding non-projected variables, and the triggering emit is already
the witness — but the sink in isolation would be unsound under any executor that
honored the signal at a projected-variable node. The coupling is commented in both
files; recorded here so a future executor change re-derives it.

### [NOTE] `resolve_filter`'s Eq-miss short-circuit is dead code on the Free Join path

`crates/bumbledb/src/api/prepared.rs:1096-1098`. `split_filters`
(`plan/fj.rs:386-407`) routes every Eq-against-a-constant into selections and
`check_selections` rejects leaks, so plan-occurrence filters never carry `Eq`; the
miss-under-Eq arm can only fire for selections (`resolve_selection`) and never here.
Harmless defensive redundancy, semantically consistent if it ever did fire.

## Checked and sound

- **Seen-key composition**: `AggregateSink::emit` dedups on slots `0..slot_count` —
  the full binding array over all query variables, never the projected/group slots
  (`sink.rs:236-241`). Slot totality holds: the plan partitions every occurrence's
  variables, every variable is new in exactly one node, and each node's chosen cover
  binds exactly its new vars before recursing, so every slot is written before any emit.
- **Elision proof at execution**: `provably_distinct` (`fj.rs:433-462`) runs on the
  *normalized* occurrences (Eq filters still present, before selection extraction) and
  requires var-bound ∪ Eq-filtered fields ⊇ a unique constraint per occurrence. Traced:
  distinct emits ⇒ distinct fact-tuples (COLT `force` lands each position in exactly
  one child; `iter_map` walks the dense occupied list; `iter_positions` walks each
  chunk once; `Cursor::Row` yields once) ⇒ distinct bound words (two facts agreeing on
  all bound fields would collide on the unique key) ⇒ distinct bindings. Zero-var
  (gate) atoms can never have unique coverage, so the flag is false and the seen-set
  dedups their duplicate emits. Guard probes emit ≤ 1 binding, so
  `ExecPlan::distinct_bindings() == true` there is sound.
- **Sum**: i128/u128 accumulation; single range check at finalization with inclusive
  bounds exactly at `i64::MIN`/`i64::MAX`/`u64::MAX` (`i64::try_from`/`u64::try_from`);
  signed decode of the biased word before accumulating (`word_to_i64` inverts
  `encode_i64` exactly); order-independence and the deterministic overflow are pinned
  by tests; i128 cannot overflow under < 2⁶⁴ terms.
- **Min/Max over words**: I64's sign-flipped big-endian word order equals numeric
  order; U64 words are the values. Validation (`ir/validate.rs::check_finds`) restricts
  Sum/Min/Max to U64/I64, makes Count nullary, rejects aggregate-over-group-key and
  duplicate find terms — so no Enum/Bool/String word ever reaches an accumulator.
  Seeds (`u64::MAX`/`u64::MIN`) are safe because a group exists only after its first
  fold.
- **Count** counts the group's distinct binding set (increments once per surviving
  emit, after the seen-set or under the proven flag); result decodes as U64.
- **Empty input** ⇒ empty group map ⇒ `finalize_into` emits zero rows — never a 0/NULL
  row; pinned by test, matching the documented divergence from SQL.
- **Group-key words → results**: finalize interleaves key words and accumulator
  results in find order on both sides; each word decodes by the find's result type
  (Sum(I64)→re-encoded biased word→I64 cell; Count→U64; String/Bytes group keys resolve
  through the dictionary).
- **D2 skip vs. distinct bindings**: the aggregate sink returns `Continue`
  unconditionally — no suffix is ever skipped under aggregation; for projection the
  skip drops only duplicate *witnesses*, never a distinct projected tuple (traced
  through multi-node existential suffixes, residual-carrying suffixes, and the
  absorb-at-relevant-node unwind).
- **ViewMemo atomicity**: the (colt, generation, filters) triple moves as a unit on the
  parked-hit swap, the park-to-victim swap, and the rebuild (all fallible operations —
  `get_or_build` — precede any mutation of the triple except a completed park-swap,
  which itself leaves a self-consistent victim triple active). An error mid-`run_join`
  leaves every occurrence's triple consistent for the next execution. Parked entries
  stay pairwise distinct and distinct from the active binding. Victim choice is
  stale-generation-first then LRU. Tick is u64 (no realistic overflow). Self-joins get
  fully independent per-occurrence memo state and trie schemas. Executing against an
  *older* snapshot after a newer one degrades to rebuild-in-place, never wrong data
  (generation comes from the transaction's own snapshot).
- **Selections vs. the memo key**: selections live in prepended trie levels, not the
  memo key; `select()` re-probes every occurrence every execution before the executor
  reads `start()`, so parked swaps can never leak a stale selection cursor; a probe
  miss returns with the sink reset and finalize yields the empty set.
- **Param discipline**: `bind_param` shares `value_matches` (kind, enum range, UTF-8)
  with validation; params are dense (BTreeMap id-order iteration + validation's gap
  rejection makes positional indexing sound); it produces only `Const::Word`/`Byte`,
  so `resolve_selection`'s `unreachable!` on Param/PendingIntern-in-params holds; one
  resolution per param is reused consistently across selections, filters, and guard
  keys. I64 params encode to the biased word matching column words and canonical
  guard-key bytes; Byte constants widen exactly as image byte-columns and `fact_word`
  do.
- **Miss semantics, all four consumers**: Eq-miss empties the query via
  `resolve_selection` (None → short-circuit), `resolve_filter` (same, though dead —
  see NOTE), guard keys (sentinel bytes → `U`/`M` probe miss), and guard remaining
  filters (sentinel word → Eq fails). Ne-miss resolves to the sentinel and matches
  every stored value (dict mint asserts the sentinel is never issued). Range ops can't
  miss (only U64/I64). Pinned by the Ne-miss regression tests.
- **finalize / ResolveMemo**: keyed on `(word, tag)` so String/Bytes never alias even
  though ids share a counter; cleared at the *start* of every finalize, so ranges can
  never dangle across buffers or executions; the error-path placeholder is unreachable
  (the error aborts the finalize that created it); UTF-8 is validated before bytes land,
  making `ResultBuffer::get`'s expect safe; the byte heap is append-only during a
  finalize, so shared `(start, len)` ranges have no mutation hazard.
- **ResultBuffer coherence**: arity stamped per execution before any fallible step;
  `clear` retains capacity; `get` asserts row/column bounds; `len` floors torn rows.
- **profile/explain ≡ execute**: identical pipeline (bind → resolve/short-circuit →
  sink reset → `run_join` → finalize); the guard branch delegates wholesale to
  `execute`; `CountingCounters` methods are pure increments into plan-sized arrays —
  no semantic effect (verified against `explain.rs`).
- **WordMap**: power-of-two capacity, ≤ 50 % load (probes terminate), insert-only,
  growth re-probes in dense insertion order (iteration determinism survives rehash),
  zero-arity keys form exactly one global group (the all-aggregate case), clear is
  O(occupied).
- **Executor/batching**: two-phase probing and branchless compaction are
  order-preserving; per-batch journals restore cursors exactly (including on the skip
  break); `sibling_children` is entry-indexed and only read for survivors; residual and
  probe sources resolve Batch-vs-Slot correctly under dynamic cover choice because
  covers bind exactly the node's new vars; batch-size equality (1/2/64/128/1024,
  empty/partial batches) and a randomized differential family against a nested-loop
  oracle pin all of it.
- **Guard path**: classification requires one occurrence, no residuals, Eq coverage of
  a unique constraint or the full fact; duplicate Eq filters on a key field keep the
  extra one in `remaining_filters` (conflicting constants correctly empty);
  `execute_guard` binds every slot in `plan.vars` order matching
  `ExecPlan::slot_of`, respects the ordinary sinks (aggregate-over-point-lookup folds
  one binding; a miss folds none → empty set), and never touches images.
- Full `cargo test -p bumbledb` passed during the audit.
