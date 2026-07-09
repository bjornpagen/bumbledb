# PRD 11 — Minor findings sweep

**Depends on:** 02 (item G's test uses the applied-inserts machinery).
**Modules:** per item below.
**Authority:** verdicts pinned by the 2026-07-09 audit; the dictionary-leak item
(historically "F") was discharged as a doc amendment and is not here.

## A — Containment ndistinct rung: take the tightest, not the first

`distinct_of`'s containment rung returns on the **first** statement whose source
projection is `[field]` (`crates/bumbledb/src/plan/selectivity.rs:205-214`). A
field under two unconditional containments to different-sized targets should
take the `min` over target row counts. One-line fold instead of early return.
- `[test]` A schema with one field under two containments (targets of different
  sizes) pins the min.

## B — Stale load-factor docstrings

`colt/force.rs:79` and the `colt.rs` map docstring say "rehash-doubling at 75%
load"; the code enforces max load 0.4 (`force.rs:87`, `(len+1)*5 > nbuckets*16`),
consistent with the bucket-of-8 design. Fix both comments; amend
`40-execution.md`'s "measured mechanisms" 75% figure to the code's 0.4 (rule 5 —
current reality is the code).
- `[shape]` No "75%" load-factor claim survives in code or docs; the two
  comment sites state 0.4 with the `(len+1)*5 > nbuckets*16` derivation.

## C — `next_origin: u32` checked increment (decided: typed error)

`self.next_origin += 1` (`exec/run.rs:446`, minted at `probe_pass.rs:296-298`)
wraps in release beyond 2³² absorb-node survivors → wrong origin cancelled →
silently dropped valid rows — the worst failure shape in the taxonomy. Beyond
the scale axiom, but a checked increment returning the existing typed `Overflow`
variant costs nothing measurable at batch granularity. The justification-comment
alternative is rejected: no comment fixes silent wrong results.
- `[shape]` The increment is checked; the error path reuses `Overflow`; the
  check sits at mint granularity, not per-row (confirm placement keeps it off
  the per-tuple path).

## D — `From<BulkLoadError> for Error` carries the committed count (decided)

`api/db/write.rs:176-181` discards `committed` for `?` ergonomics — but the
count is the whole reason the type exists (resumable partial imports). Carry it
into the `Error` variant's payload; structured payloads are the house doctrine.
- `[test]` A mid-stream bulk-load failure surfaced through `?` still exposes
  the committed count from the `Error`.
- `[shape]` `70-api.md` ETL section reflects the payload (rule 5).

## E — `Error::source()` scope is a decision — document it

`error/convert.rs:41-49`: only `Io`/`Lmdb` chain through `source()`;
`Corruption`/`Schema`/`Validation`/`FactShape` carry structured payloads
invisible to `std::error::Error` chain-walking. This is a decision (payloads are
structured data, not nested errors) — one rustdoc sentence on `Error` saying so.
- `[shape]` The sentence exists on the `Error` type's doc comment.

## G — `advance_serial_marks` before the idempotence check: prove unreachable

`delta/insert.rs:26` advances serial marks even for base no-op inserts. The
audit's position: the dirty-mark scenario is **unreachable** — the committed
high-water covers every committed serial value (explicit inserts advance past
their value at their original commit), and a no-op insert means the fact is
already committed, so its serial value is already covered. The work: verify the
invariant against the code (including the bulk-load and explicit-resupply
paths); if it holds, state it as a comment at the advance site **plus a test**
proving a pure-no-op transaction never triggers a counters-only commit (assert
the storage tx id AND the `Q` marks are untouched after a commit whose every op
was a no-op). If the code falsifies the reasoning, reorder the advance after the
no-op determination and say so in the commit body.
- `[test]` The pure-no-op-transaction test (whichever branch was taken).
- `[shape]` Either the invariant comment or the reorder exists — not neither,
  not both.

## Passing criteria (whole PRD)

- All item-level criteria above.
- `[gate]` Workspace gates green.

## Verified-sound notes (do not re-audit; carried from the review)

Sign-flip BE encoding memcmp-orders all types (`encoding/encode.rs:20`, test
`keys.rs:445`); neighbor-probe sufficiency and half-open adjacency
(`applier.rs:224-278`); coverage-walk gap logic (`judgment.rs:508-599`);
ψ-qualified re-establishment (`judgment.rs:266-289`); serial reuse after abort;
empty relation/param-set/negation edges in the executor; survivor compaction and
pipeline aliasing discipline; cache generation race-closers
(`get_or_build.rs:33,90`); parked-reader coherence (`api/db/read.rs`,
`write.rs:69-97`); arena handle stability (`arena.rs:39-57`); fingerprint
sensitivity (`fingerprint.rs` tests).
