# PRD 18 — The generation witness: read-compute-write as a value

**Depends on:** nothing in this set; lands on the current write path.
**Modules:** `crates/bumbledb/src/api/db/` (`write.rs`, the new entry point),
`crates/bumbledb/src/api/db.rs` (snapshot surface), `error.rs`,
`crates/bumbledb-bench` (naive model + differential scenarios), docs.
**Authority:** `00-product.md` (concurrency model, deleted vocabulary),
`70-api.md` (transactions, the full-queries-in-write-txns ruling),
`30-dependencies.md` (final-state judgment).
**Representation move — stated in the lineage's own terms.** A read-compute-
write race is a *control-flow* problem, and the industry's answers are
control-flow answers: row locks, `SELECT FOR UPDATE`, serializable retry
machinery — ordering imposed on traces. The representation answer (SICP,
Insight 14: reify control flow as data) is to make "the state I read" a
**value**: every snapshot already knows its generation, so the fact a lock
would protect — *nothing changed since I looked* — becomes a proposition the
commit can check in one integer compare. And the API carries the proof
instead of a claim (King, Insight 6 — parse, don't validate): the entry point
takes the **snapshot itself** as the witness, not a raw integer a caller
could fabricate or stale-cache. The silent interleave — the one bug class the
single-writer design still admitted — becomes unrepresentable as a *silent*
event (Minsky): it is either absent or a typed error. One more SQL word joins
the deleted vocabulary with a real replacement behind it.

## Context (decided shape)

**The gap, precisely.** WriteTx point reads see the delta-overlaid final
state, so key-shaped check-then-act inside one write txn is race-free by
construction (the recorded TOCTOU claim). Full queries inside write txns are
forbidden (recorded ruling — images do not overlay deltas). Therefore
**query-driven writes** — update-where-predicate, insert-select, everything
Postgres spells with data-modifying CTEs — must read on a snapshot first,
then write. The writer mutex serializes write *transactions*, not
read-compute-write *sequences*: two host threads interleaving
snapshot-read → compute → write can clobber each other's premises. That is
the whole gap, and it is closed by one compare.

**The shape:**

- `Db::write_from(&self, witness: &Snapshot<'_>, f) -> Result<T>` — identical
  to `write` except the commit **aborts before any page is touched** if a
  state-changing commit has landed since the witness's generation, with the
  typed error `GenerationMoved { witnessed, current }`. The delta drops
  exactly as any abort does; nothing durable happened.
- **The witness is the snapshot, never an integer.** A `Snapshot` is evidence
  — generation read inside its own transaction (the existing race-closer),
  environment identity checked exactly as prepared queries check it
  (`ForeignSnapshot` on mismatch). An integer parameter would be a claim; the
  refusal is recorded.
- **State-changing generations only.** A counters-only/no-op commit by
  another thread does not invalidate anyone's reads and does not trip the
  witness — the compare targets the same generation the image cache keys on.
  Precision here is free and the sloppy alternative (any-commit) is recorded
  as rejected: it would manufacture spurious retries out of no-ops.
- **Retry is host policy.** The engine ships the error, never a loop — the
  staleness-signal doctrine verbatim (PRD 13 of `docs/prd/`): policy belongs
  to the host; the engine's job is to make the condition *checkable*.
  Conflicts are rare by the bursty-write design point; the host's retry is
  re-run-query → re-compute → `write_from` again.
- **Composition, documented as one story**: the witness is the scan-shaped
  guard (premises from full queries); WriteTx point reads remain the
  key-shaped guard (per-fact precision, zero retries). Together they are the
  complete conditional-write vocabulary: *read the model, propose a delta,
  commit iff the model you read is still the model.*
- **Deleted vocabulary, extended** (`00-product.md` table): *SELECT FOR
  UPDATE / row locks / SERIALIZABLE retry* → the generation witness plus
  WriteTx point reads under final-state judgment. Each word's replacement is
  strictly stronger: locks protect what you remembered to lock; the witness
  protects everything the snapshot saw.

## Technical direction

1. `write_from`: take the writer mutex, read the current state-changing
   generation inside the critical section, compare against the witness's,
   abort-or-proceed. One branch, cold on the success path, before the delta
   applies. (The generation the parked-reader machinery already maintains is
   the same number — no new counter exists.)
2. `Snapshot` exposes nothing new publicly beyond what `write_from` consumes;
   a `generation()` accessor ships only if the stats surface wants it
   (diagnostics, not API).
3. Error: `GenerationMoved { witnessed, current }` in the write group;
   payload is the two generations (ids, never strings).
4. Naive model: a counter compare — the semantics are two lines, which is the
   point; differential scenarios: (a) interleaved read-compute-write pairs,
   second aborts with the right payload; (b) no-op commit between read and
   write does NOT abort; (c) foreign snapshot rejected; (d) `write_from`
   with no intervening commit behaves byte-identically to `write`.
   **Error parity is asserted including the typed identity** — the
   direction-divergence lesson (`docs/prd/05`) applied from birth.
5. Docs write the *idiom*, not just the API: the conditional-write chapter
   shows update-where, insert-select, and derived-relation maintenance
   (PRD 19) each as query → compute → `write_from` → host retry.

## Passing criteria

- `[test]` The four differential scenarios above, engine vs naive model,
  verdict-and-payload identical.
- `[test]` Two real host threads (the one concurrency test the engine
  permits itself): interleaved sequences over one relation, final state
  equals a serial execution of the retried schedule — the witness's whole
  claim, exercised with actual parallelism.
- `[shape]` The success path of `write_from` differs from `write` by exactly
  one compare (read the diff); no lock is held across any read phase; no
  retry loop exists in the engine (grep).
- `[shape]` The deleted-vocabulary table carries the three new words with
  their replacement; the full-queries-in-write-txns ruling cross-references
  the witness as its compensating control.
- `[gate]` Workspace gates green; the alloc gate untouched (the write path
  was never in the zero-alloc contract's measured window).

## Doc amendments (rule 5)

`70-api.md`: the conditional-write chapter (witness + point-read guards, the
three idioms, host-retry convention). `00-product.md`: deleted-vocabulary
rows; the concurrency section gains one sentence ("read-compute-write is
optimistic, witnessed by snapshots, checked in O(1) at commit").
`30-dependencies.md`: cross-reference only (judgment semantics untouched —
the witness runs before the pipeline, and an aborted witness never reaches
judgment).
