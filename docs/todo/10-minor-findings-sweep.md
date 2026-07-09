# 10 — Minor findings sweep

**Kind:** batch of small, independent items from the 2026-07-09 review. None is
urgent; each is cheap. Sweep in one change or cherry-pick.

## A — Containment ndistinct rung takes the first match, not the tightest

`distinct_of`'s containment rung returns on the **first** statement whose source
projection is `[field]` (`crates/bumbledb/src/plan/selectivity.rs:205-214`). A
field under two unconditional containments to different-sized targets should take
the `min` over target row counts — looser bound today, never unsound, perf-only.
One-line fold instead of early return; a unit test with two containments pins it.

## B — Stale load-factor docstrings

`colt/force.rs:79` and the `colt.rs` map docstring say "rehash-doubling at 75%
load"; the code enforces max load 0.4 (`force.rs:87`,
`(len+1)*5 > nbuckets*16`), consistent with the bucket-of-8 design. Fix the two
comments — sizing-sensitive structure, misleading for maintainers. (Also
`40-execution.md` "measured mechanisms" repeats the 75% figure — amend per rule 5.)

## C — `next_origin: u32` unchecked growth

`self.next_origin += 1` (`exec/run.rs:446`, minted at
`probe_pass.rs:296-298`) overflows beyond 2³² absorb-node survivors: debug panic,
release wrap → wrong origin cancelled → **silently dropped valid rows**. Beyond
the ≤10⁷ scale axiom (intermediates can exceed base cardinality, but not by 400×
at this scale) — still, wrap-to-wrong-results is the worst failure shape. A
checked increment returning a typed overflow error (the `Overflow` variant
exists) costs nothing measurable at batch granularity; or a
`const_assert`-style justification comment if refused.

## D — `From<BulkLoadError> for Error` drops the committed-chunk count

`api/db/write.rs:176-181` discards `committed` for `?` ergonomics — but the count
is the whole reason the type carries it (resumable partial imports; prior chunks
stay committed by design, `write.rs:119-157`). Either carry it into the `Error`
variant's payload or document loudly on `bulk_load` that resumability requires
matching `BulkLoadError` before `?`.

## E — `Error::source()` chains only for `Io`/`Lmdb`

`error/convert.rs:41-49`. `Corruption`/`Schema`/`Validation`/`FactShape` carry
structured payloads invisible to `std::error::Error` chain-walking (anyhow/eyre
`.chain()`). Intentional (payloads are structured, not nested errors) — if kept,
one doc sentence on `Error` saying so, so it reads as a decision rather than an
omission.

## F — Dict leak is broader than "deleted facts leak"

`flush_counters` writes **all** `pending_interns` on any state-changing commit
(`storage/commit/write.rs:180`, `delta/intern.rs:71`): a novel string interned for
a fact whose insert turned out to be a no-op still lands in `_dict` when any other
fact changed state — an id no committed fact ever referenced. Within the
accepted-leak axiom (`10-data-model.md`), but the OPEN item's trigger metric
("dictionary growth dominating store size") should count this class too. Either
filter pending interns to applied facts at flush, or amend the accepted-leak
sentence to cover never-referenced ids. Amending is fine; filtering is ~a set
intersection at flush time if the no-op information from TODO item 02 lands.

## G — `advance_serial_marks` before the idempotence check

`delta/insert.rs:26` advances serial marks even for base no-op inserts; a purely
no-op transaction inserting an unseen explicit serial value can dirty a mark and
trigger a counters-only commit. Harmless (marks never regress committed state) —
either reorder after the no-op determination where cheap, or add the one-line
comment saying it is deliberate.

## Verified-sound notes (do not re-audit)

Recorded so this sweep doesn't grow: sign-flip BE encoding memcmp-orders all types
(`encoding/encode.rs:20`, test `keys.rs:445`); neighbor-probe sufficiency and
half-open adjacency (`applier.rs:224-278`); coverage-walk gap logic
(`judgment.rs:508-599`); ψ-qualified re-establishment (`judgment.rs:266-289`);
serial reuse after abort; empty relation/param-set/negation edges in the executor;
survivor compaction and pipeline aliasing discipline; cache generation
race-closers (`get_or_build.rs:33,90`); parked-reader coherence
(`api/db/read.rs`, `write.rs:69-97`); arena handle stability (`arena.rs:39-57`);
fingerprint sensitivity (length prefixes, variant order, serial toggles —
`fingerprint.rs` tests).
