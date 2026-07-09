# 10 — Minor findings sweep

**Kind:** batch of small, independent items. Verdicts pinned per the 2026-07-09
audit; F was resolved as a doc amendment and removed from this list (see git
history); G is downgraded to prove-unreachable. Sequencing: run after item 02
(G's test uses the applied-inserts fixture shape 02 introduces).

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
comments — sizing-sensitive structure, misleading for maintainers. Also
`40-execution.md` "measured mechanisms" repeats the 75% figure — amend per rule 5
(current reality is the code's 0.4).

## C — `next_origin: u32` unchecked growth — **decided: checked increment**

`self.next_origin += 1` (`exec/run.rs:446`, minted at
`probe_pass.rs:296-298`) overflows beyond 2³² absorb-node survivors: debug panic,
release wrap → wrong origin cancelled → **silently dropped valid rows**. Beyond
the scale axiom, but wrap-to-wrong-results is the worst failure shape in the
taxonomy and a checked increment returning the existing typed `Overflow` variant
costs nothing measurable at batch granularity. Take the checked increment; the
justification-comment alternative is rejected — no comment fixes silent wrong
results.

## D — `From<BulkLoadError> for Error` drops the committed-chunk count — **decided: carry the payload**

`api/db/write.rs:176-181` discards `committed` for `?` ergonomics — but the count
is the whole reason the type carries it (resumable partial imports; prior chunks
stay committed by design). Carry it into the `Error` variant's payload —
structured payloads are the house error doctrine; documenting the footgun instead
is rejected.

## E — `Error::source()` chains only for `Io`/`Lmdb` — **decided: document the decision**

`error/convert.rs:41-49`. `Corruption`/`Schema`/`Validation`/`FactShape` carry
structured payloads invisible to `std::error::Error` chain-walking. This is a
decision (payloads are structured data, not nested errors) — one rustdoc sentence
on `Error` saying so, so it reads as a decision rather than an omission.

## G — `advance_serial_marks` before the idempotence check — **decided: prove unreachable**

`delta/insert.rs:26` advances serial marks even for base no-op inserts. The
review proposed reorder-or-comment; the audit's position is that the dirty-mark
scenario is **unreachable**: the committed high-water covers every committed
serial value (explicit inserts advance past their value at their original
commit), and a no-op insert means the fact is already committed, so its serial
value is already covered — the mark cannot newly dirty. The work: verify this
invariant against the code; if it holds, state it as a comment at the advance
site **plus a test** proving a pure-no-op transaction never triggers a
counters-only commit. If the code falsifies the reasoning, reorder the advance
after the no-op determination and say so in the commit.

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
