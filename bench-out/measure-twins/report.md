# The Measure phase — the measure-or-merge twins (cleanup-0.5.0 rulings 6, 7, 8)

`docs/prds/cleanup-0.5.0/prd-M-measure.md` Part 1. Protocol: each twin
runs interleaved A/B (fast path vs generic machinery), release build,
min-of-N, through `scripts/measure.sh`; result equivalence asserted in
the twin itself. Exactly one verdict per twin: **law** (record number +
reverses-if at the site and in `40-execution.md`) or **merge** (delete
the fast path, route generic, oracle re-referees).

## THE PRE-STATED THRESHOLD (written before any number below existed)

A twin's fast path is a **real win** iff `generic/fast >= 1.09` on the
min-of-N interleaved ratio — the crucible ADOPT precedent (~9%, the
smallest banked win in `40-execution.md`'s decision records). Below
1.09 the verdict is **merge**, no negotiation. Tier: all three twins
are warm in-process microlanes over DRAM-resident fixtures; the tier
is stated with each number per the landing bar.

- Twin 1, leaf-elision complex (ruling 6): `cargo test -p bumbledb
  --release --lib -- --ignored measure_twins_leaf_elision --nocapture
  --test-threads=1`; ratio = generic/elided ns-per-exec.
- Twin 2, all-words finalize (ruling 7): same invocation,
  `measure_twins_all_words_finalize`; ratio = resolved/words
  ns-per-finalize, BOTH sinks (projection and aggregate). Each sink is
  judged against 1.09 on its own; a split verdict records per-sink
  truth and the merge applies only to a route below the bar.
- Twin 3, permuted-identity determinant (ruling 8): same invocation,
  `permuted_identity_determinant_twin`; ratio = identity-permuted/direct
  ns-per-fact.

## Machine conditions

- Host: Apple M2 Max, macOS (Darwin 24.6.0), release build.
- Session: 2026-07-19, through `scripts/measure.sh` (the mkdir lock).
- Idle check before each timed block: `uptime` + `ps` scan for foreign
  heavy processes; recorded below.

## The numbers

(appended by the timed runs; nothing above this line changes after)
### Idle checks (recorded at run time, 2026-07-19 ~15:46 local)

- `uptime`: load averages 2.26 2.95 3.22 (12-core M2 Max; falling).
- `ps` top consumers: this session's toolchain + sibling claude agents at
  ~5–12% CPU each, no foreign heavy process, no other measurement holder
  (`/tmp/bumbledb.measure.lock` acquired fresh for every run).

### Twin 1 — the leaf-elision complex (ruling 6)

Fixture: 256 single-posting + 128×8 multi-posting accounts; self-join
`Q(x,y) :- Posting(A,x), Posting(A,y)`; 700 answers/exec; firing proof
asserted (A engaged, B routed generic); equivalence asserted. Warm DRAM,
release, interleaved min-of-7 × 10 execs/sample.

| process run | elided ns/exec | generic ns/exec | generic/elided |
|---|---|---|---|
| 1 | 138,008 | 236,216 | **1.712** |
| 2 | 107,262 | 181,104 | **1.688** |

**VERDICT: LAW** (1.69–1.71 ≥ 1.09). Recorded at
`crates/bumbledb/src/exec/run/leaf.rs` (site) and
`docs/architecture/40-execution.md` § the leaf fast paths, with the
reverses-if clause. The twin and the `disable_leaf_elision` switch died
with the verdict.

### Twin 2 — the all-words finalize fast path (ruling 7)

Fixture: 20,000 postings; projection sink (20,000 answers) and
CountDistinct aggregate sink (997 groups); equivalence asserted (and
falsifier-guarded end to end by `tests/fixpoint_finalize_hunt.rs`).
Warm DRAM, release, interleaved min-of-7 finalizes.

| process run | sink | words ns | resolved ns | resolved/words |
|---|---|---|---|---|
| 1 | projection | 56,958 | 56,875 | 0.999 |
| 1 | aggregate | 9,166 | 9,167 | 1.000 |
| 2 | projection | 57,042 | 56,791 | 0.996 |
| 2 | aggregate | 9,125 | 9,167 | 1.005 |

**VERDICT: MERGE** (both sinks a dead heat, < 1.09). Executed: the
`AnswerHeap` seal, `fill_word_answers`, and `push_word_answer` deleted;
every finalize routes through the resolving fill (whose fixed-type arms
ARE the word path — the dispatch count never differed). Gravestone at
`crates/bumbledb/src/api/prepared/finalize.rs`; correctness re-refereed
by the finalize-hunt falsifier + full engine suite + the bench verify
oracle (this session's ephemeral re-earn runs on a fresh stamp).

### Twin 3 — the permuted-identity determinant (ruling 8)

Fixture: commit-shaped 3-field interval projection, 200,000 per-fact
re-derivations per sample; byte-equivalence of the identity arm
asserted. Warm DRAM, release, interleaved min-of-7.

| process run | direct ns/fact | identity-permuted ns/fact | permuted/direct |
|---|---|---|---|
| 1 | 13 | 17 | **1.233** |
| 2 | 13 | 17 | **1.249** |

**VERDICT: LAW** (1.23–1.25 ≥ 1.09) — the permuted arm's per-fact
inverse search is real cost; `determinant_image` stays as the direct
arm. Recorded at `crates/bumbledb/src/storage/keys.rs` (site) and
`docs/architecture/50-storage.md` § key layout, with the reverses-if
clause. The twin died with the verdict.
