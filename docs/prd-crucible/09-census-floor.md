# PRD 09 — The census floor: re-audit at the new fixpoint

**Depends on:** 04–08 all landed (this PRD measures their combined
effect; it is Phase C's terminal reconciliation and must not start
early).
**Modules:** read-mostly across `crates/`; write access to
`docs/architecture/*` amendment blocks, `docs/reference/
apple-silicon-performance.md` (the staleness ruling), and this campaign's
README ledger.
**Authority:** the audit discipline from the witness campaign (batteries
of greps with expected-zero results, run at the END, so regressions
introduced mid-campaign cannot hide), and policy 5 (docs found wrong get
a conflict block, not a silent fix).
**Representation move:** none — this PRD converts "we believe the
campaign removed the defect classes" into counted, dated evidence, and
re-establishes the unreachable!/assert floor that future audits diff
against.

## Context (decided shape)

The batteries, each with its expected result recorded IN THIS FILE when
run:

1. **Vocabulary battery** (PRD 04/05 closure): the PRD 05 greps re-run
   at campaign end — `PredicateTree`, `.predicates`, `head_types`,
   `result_types`, `column_types`, `resolve_predicates` all zero across
   `crates/` AND `docs/`.
2. **Defensive-check census**: count `unreachable!`, `assert!`,
   `debug_assert!`, `.expect(` across `crates/bumbledb/src`, per file.
   The witness campaign's floor was 147 `unreachable!`; record the new
   floor and attribute the delta to PRDs by mechanism (02's slice APIs,
   08's classified comparisons). Any file whose count ROSE gets an
   explanation or a fix.
3. **Underscore battery**: zero `_`-prefixed parameters/functions
   outside trait-impl-required positions (the standing refactoring-debt
   rule).
4. **Lint posture**: zero `#[allow(` outside the recorded policy sites
   (the one `unsafe_code` policy site is on the record; `expect` is the
   house form). List every survivor with its justification line.
5. **Doc-reference battery**: zero references to deleted chapters or
   pre-reset numbering in `docs/architecture/` and `docs/cookbook.md`.
   Known offenders going in: six "30-execution" references confined to
   `docs/reference/` and `brainlift-sources/` — see the ruling below.
6. **Nightly-dividend battery** (PRD 02 closure): the "fixed-width
   slice" expect count at its recorded target; zero `(|| {` error
   closures.

**The staleness ruling** (decided here, executed here):
`docs/reference/apple-silicon-performance.md` cites pre-reset chapter
numbers and pre-reset measurements. Reference documents are records of
what was true when written — they are not silently renumbered. The
ruling: add a dated header block stating the document predates the
2026-07 reset, that its chapter citations refer to the pre-reset tree at
`1b65ae8^`, and that current measurement doctrine lives in the
architecture chapters; fix NOTHING else in it. `brainlift-sources/` gets
the same one-line treatment in its README if it has one, else a
`PROVENANCE.md` line. Post-reset docs citing dead chapters (if battery 5
finds any outside these two roots) are plain bugs — fix them.

## Technical direction

Run every battery, paste the actual command + count into this file's
"Results" section (append it; the PRD file is the ledger). Where a
battery fails: fix if mechanical, conflict-block per policy 5 if the fix
requires a decision, and never relax a battery to pass. Close by writing
the campaign amendment blocks the earlier PRDs promised, verifying each
landed doc amendment actually exists (grep, don't trust).

## Passing criteria

- `[shape]` All six batteries green with results recorded in this file
  (command, count, date).
- `[shape]` The staleness header block present in
  `apple-silicon-performance.md`; zero other edits to that file.
- `[shape]` Every doc amendment promised by PRDs 01–08 verified present
  (a checklist in the results section, each item with its grep).
- `[gate]` The full workspace gate suite green — this PRD is where
  "green at campaign close" from Phases A–C is actually cashed.

## Doc amendments (rule 5)

The verification checklist above IS this PRD's amendment duty; no new
prose beyond the staleness headers.

## Results (executed 2026-07-13, at `7635932` on `codex/prd-crucible-campaign`)

### Battery 1 — vocabulary (PRD 04/05 closure): GREEN

```
for tok in head_types result_types column_types PredicateTree '\.predicates' \
           MAX_PREDICATE_DEPTH PredicateNestingTooDeep resolve_predicates; do
  grep -rn -E "$tok" crates/ docs/ | grep -v '^docs/prd-crucible/'
done
```

**Zero hits for all eight tokens** across `crates/` and `docs/`
(2026-07-13). Exemption noted: `docs/prd-crucible/` files
(`README.md`, `04-predicate.md`, `05-condition-tree.md`, this file) name
the old tokens as their own rename ledgers — that is the record of the
rename, not a use of the vocabulary, and is exempt.

### Battery 2 — defensive-check census: GREEN (new floor recorded)

Command family, run 2026-07-13:

```
grep -rc 'unreachable!'    crates/bumbledb/src --include='*.rs'   # per file
grep -rc 'assert!'         crates/bumbledb/src --include='*.rs'   # per file; matches include debug_assert! lines
grep -rc 'debug_assert!'   crates/bumbledb/src --include='*.rs'   # per file
grep -rcF '.expect('       crates/bumbledb/src --include='*.rs'   # per file
```

Totals (occurrences): **`unreachable!` 124** (121 non-test + 3 in test
files), **plain `assert!` 621** (via `grep -rhoE '(^|[^_a-zA-Z])assert!'`,
excluding `debug_assert!`), **`debug_assert!` 57**, **`.expect(` 2026**
(the bulk in `tests/` modules).

**The new floor: 121 non-test `unreachable!`, down from the witness
campaign's 147** (non-test filter: exclude `/tests`, `tests.rs`,
`trace_tests.rs`; baseline re-measured at `eba32a5` with the same
filter = 147 exactly).

Per-file `unreachable!` census (the table future audits diff against):

```
api/db/encode_dyn.rs:1        api/db/get.rs:1               api/prepared/bind.rs:15
api/prepared/build.rs:2       api/prepared/execute.rs:2     api/prepared/result_buffer.rs:5
api/prepared/tests/sets.rs:1  encoding/encode.rs:2          exec/colt/append_child.rs:1
exec/colt/force.rs:2          exec/colt/iter.rs:1           exec/colt/select.rs:1
exec/colt/tests/model.rs:1    exec/dispatch/fact_word.rs:1  exec/dispatch/guard_probe_fact.rs:17
exec/run.rs:3                 exec/run/execute.rs:1         exec/sink/aggregate/finalize.rs:3
exec/sink/aggregate/fold_batch.rs:2  exec/sink/aggregate/fold_row.rs:1  exec/sink/aggregate/groups.rs:1
exec/sink/aggregate/new.rs:1  exec/sink/aggregate/sink.rs:5 exec/sink/projection/measured.rs:1
image/decode.rs:4             image/tests/pitch.rs:1        image/view.rs:2
image/view/apply.rs:30        ir.rs:1                       ir/normalize/fold.rs:3
ir/normalize/lower_literal.rs:2  ir/normalize/normalize.rs:1  ir/render.rs:1
ir/validate/validate.rs:1     plan/chase/evaluate.rs:1      plan/selectivity.rs:4
verify_store/facts.rs:2
```

**Delta attribution vs the `eba32a5` baseline** (per-file diff of the
same grep at both commits — the four moved files account for the entire
−26 non-test delta):

- `ir/normalize/place_comparisons.rs` 12 → 0, `ir/validate/context.rs`
  15 → 0, `ir/validate/validate.rs` 2 → 1 (−28): **PRD 08** (`2b90008`)
  — classification is proved once and sealed as `ClassifiedComparison`;
  placement consumes the witness with a total match, so the validator's
  and placer's illegal-shape re-derivation arms were deleted, not
  guarded.
- `verify_store/facts.rs` 0 → 2 (+2, **the one file that ROSE —
  explanation**): the citation-representation refactor (`3f167bb`, "a
  rejection is the complete violation set"). The judgment probes now
  destructure the total violation sum; both sites are `let … else
  unreachable!("the judgment probes cite containments only")` — the
  probe path constructs containment citations exclusively, and the
  destructure documents that invariant. Accepted, not fixed: the arm is
  the price of the total sum, and the message names the invariant.
- **PRD 02's slice APIs moved `.expect(` counts, not `unreachable!`**:
  e.g. `encoding/decode.rs` 4 → 2, `storage/keys.rs` 6 → 3,
  `exec/colt/grow.rs` 5 → 4, `exec/colt/probe.rs` 4 → 2,
  `storage/commit/judgment.rs` 4 → 1, `schema/validate.rs` 12 → 11,
  `verify_store/{facts,guards,counters,membership}.rs` 9 → 0 — the
  split-chain parsers made the length check the parse.

Other per-file rises vs baseline, all four counters (each explained):

- `.expect(`: `exec/kernel/filter.rs` 0 → 5 — **PRD 03** (`6c8af0c`),
  the new `std::simd` filter kernels' `usize::try_from`/`u32::try_from`
  narrowing expects ("positions fit u32" family — integer narrowing,
  out of the PRD 02 slice family per its recorded ruling);
  `ir/validate/finds.rs` 0 → 1 — **PRD 04** (`a45afea`),
  `over.expect("validated: only Count is nullary")`, a
  sealed-at-validation invariant; `allen.rs` 2 → 8 — **PRD 15**
  (`7635932`), all six new sites in the `#[cfg(test)]` exhaustive
  small-world tests; `ir/validate/tests/signature.rs` 0 → 3 and
  `schema/tests/closed_member.rs` 0 → 1 — new test files from PRDs
  04/08.
- `assert!`/`debug_assert!`: `ir/normalize/lower_literal.rs` 0 → 1
  (`debug_assert!(tail.is_empty(), "encode_fixed_bytes pads to whole
  words")` — the `as_chunks` port replaced an `.expect` guard, net
  defensive-count neutral); `allen.rs` 4 → 5 and the test-file rises
  (`explain.rs`, `kernel/tests.rs`, `pipeline.rs`, `normalize/tests.rs`)
  are new campaign tests (PRDs 03/04/08/15) asserting in test code.
- Every other file held or fell.

### Battery 3 — underscore: GREEN

```
grep -rn -E 'fn _|fn [a-z_]+\(_[a-z]|[,(] *_[a-z][a-zA-Z0-9_]*\s*:' crates/ --include='*.rs'
```

8 hits (2026-07-13), all in exempt positions; zero outside them:

- Trait-impl-required (the unused parameter is the trait's signature):
  `Continuation::maximal(_start, _frontier)` at
  `interval/sweep.rs:157,332` (test impls) and
  `storage/commit/judgment.rs:814` (a gap at the probe IS the
  violation — the bounds are irrelevant to the verdict);
  `Sink::end_scan(_scan)` at `exec/sink/projection/sink.rs:174` (test
  impl counts scans, ignores the leaf).
- The `obs.rs` feature-off stub surface (lines 456, 488, 496, 503):
  `#[cfg(not(feature = "trace"))]` bodies with the documented contract
  "identical signatures, empty bodies, ZST guard — call sites never
  write `#[cfg]`". The parameters exist because the trace-on twin's
  signature requires them; this is the cfg-parity analogue of the
  trait-impl exemption and predates the campaign.

### Battery 4 — lint posture: GREEN

```
grep -rn '#\[allow(' crates/ fuzz/
```

**Exactly one survivor** (2026-07-13), the recorded `unsafe_code` policy
site — `crates/bumbledb/src/exec/kernel/prefetch.rs:8`, justification
lines quoted:

> `// \`unsafe\` exists only in the aarch64 body; the portable body is
> safe, so an` `// expectation would be unfulfilled when this same item
> is built off aarch64.`

`#[expect]` is the house form: 106 sites across `crates/`.

### Battery 5 — doc references: GREEN (staleness ruling executed)

```
grep -rnoE '[0-9]{2}-[a-z-]+\.md' docs/ --include='*.md' \
  | grep -vE '^docs/(prd-crucible|reference|brainlift-sources)/' \
  | grep -vE '(00-product|10-data-model|20-query-ir|30-dependencies|40-execution|50-storage|60-validation|70-api)\.md'
```

**Zero dead-chapter references** in `docs/architecture/`,
`docs/cookbook.md`, and everything else outside the two exempt roots
(2026-07-13). The known offenders were confirmed confined exactly as
predicted: six `30-execution` references — five in
`docs/reference/apple-silicon-performance.md`, one in
`docs/brainlift-sources/free-join-paper.md` — plus four pre-reset
`40-storage.md` citations in the same two roots.

**The staleness ruling, executed**: the dated header block is at the top
of `docs/reference/apple-silicon-performance.md` (predates the 2026-07
reset; chapter citations refer to the pre-reset tree at `1b65ae8^`;
current measurement doctrine lives in the architecture chapters) — zero
other edits to that file. `docs/brainlift-sources/README.md` exists and
got the one-line treatment (pre-reset citations are historical
provenance at `1b65ae8^`, exempt from sweeps). No post-reset doc outside
those roots cited a dead chapter, so no bug fixes were required.

### Battery 6 — nightly dividend (PRD 02 closure): GREEN

```
grep -rn '"fixed-width slice"\|"8-byte field"\|"8-byte half"\|"16-byte field"\|"8-byte word"' crates --include='*.rs'
# → 0 (2026-07-13)
grep -rn '})()' crates --include='*.rs'
# → 0 immediately-invoked closures (2026-07-13); `(|| {` hits are all
#   closure *arguments* (spawn/get_or_init/then/or_else), not the
#   immediately-invoked error idiom — try blocks stand in its place
```

The slice-guard expect family holds at its recorded floor, **exactly the
5 justified survivors** from PRD 02's ledger, re-verified at their
sites: `encoding/decode.rs:125` (`field_word_bytes`),
`encoding/decode.rs:159` (`decode_field` interval arm),
`storage/commit/judgment.rs:644` (`check_coverage` guard scratch),
`exec/wordmap/probe.rs:64` (mirror-tail window),
`encoding/tests.rs:240` (`bytes<12>` pad-word fixture). Out-of-family
per the recorded rulings: the `"positions fit u32"` narrowing family
(now also in PRD 03's `kernel/filter.rs`) and the stored-counter value
assertions.

### Doc-amendment verification checklist (PRDs 01–08, 10–15)

Each PRD's "Doc amendments" section read; each promise grepped in the
target doc (all 2026-07-13):

- [x] **PRD 01** — `00-product.md` toolchain posture:
  `grep -n 'one pinned nightly' docs/architecture/00-product.md` →
  line 209 (decision, refused stable-split alternative, deliberate-move
  rule). Repo README gate section: `grep -n 'pinned toolchain'
  README.md` → line 323.
- [x] **PRD 02** — none promised; none required.
- [x] **PRD 03** — `40-execution.md` § sanctioned kernel shapes:
  `grep -n 'portable/intrinsic split' docs/architecture/40-execution.md`
  → line 588, pointing at the PRD's verdict matrix as the record.
- [x] **PRD 04** — `20-query-ir.md` predicate concept + no-reference
  fence: `grep -n 'defines one anonymous predicate\|The fence'
  docs/architecture/20-query-ir.md` → lines 56, 66. `70-api.md`
  buffer-typing authority: `grep -n 'PreparedQuery::predicate'
  docs/architecture/70-api.md` → line 368.
- [x] **PRD 05** — `grep -n 'ConditionTree'
  docs/architecture/20-query-ir.md` → lines 242, 246–248, 482 (grammar
  and lowering).
- [x] **PRD 06** — cookbook recipe: `grep -n '## 24. The closure idiom'
  docs/cookbook.md` → line 801. README recipe count: `grep -n
  'twenty-five worked schemas' README.md` → line 294 (names
  "host-driven closures").
- [x] **PRD 07** — the pointer line: `grep -n 'recursion-design'
  docs/architecture/20-query-ir.md` → line 116
  (`docs/reference/recursion-design.md`; a firing trigger goes through
  that ledger).
- [x] **PRD 08** — `grep -n 'the fifth sealed finding'
  docs/architecture/20-query-ir.md` → line 672 (proved once, sealed,
  consumed totally, alongside the four witness-set findings).
- [x] **PRD 10** — `grep -n 'entropy seam'
  docs/architecture/60-validation.md` → lines 339 (generator section:
  two sources, one generator, digest-pinned) and 416; Tiny's purpose at
  lines 345–348.
- [x] **PRD 11** — `grep -n 'The fuzzing charter'
  docs/architecture/60-validation.md` → line 412: the charter section
  carries targets, oracle discipline, corpus policy, and the trophy
  ledger location (`fuzz/README.md`).
- [x] **PRD 12** — the ops line in the charter: ten-verb alphabet and
  the five oracles enumerated (`grep -n 'op-stream flagship'
  docs/architecture/60-validation.md`).
- [x] **PRD 13** — charter one-liners for `query` and `rewrites`
  present; the rewrite sections' sentence: `grep -rn 'semantics-preserving
  by the rewrites fuzz target' docs/architecture/` →
  `20-query-ir.md:578` (the fold, `fold-off`) and
  `40-execution.md:366` (the chase, `chase-off`).
- [x] **PRD 14** — `grep -n 'Crashpoints: the named atomicity structure'
  docs/architecture/50-storage.md` → line 235 with the crashpoint table
  at 249 and the recovery claim; the crash line in the charter.
- [x] **PRD 15** — `grep -n 'Small worlds, Miri, and ASAN'
  docs/architecture/60-validation.md` → line 478 (what is enumerated is
  never fuzzed; the Miri lane's honest FFI scope at 511; the ASAN
  coverage claim at 525).

**No missing amendments — nothing needed adding.**

### Terminal gate (the campaign's "green at close", cashed)

All three runs observed 2026-07-13:

- `scripts/check.sh` — **exit 0**, "all gates green": fmt, clippy `-D
  warnings`, workspace tests, doc tests, the release allocation gate,
  PRD 13's fold-off matrix line (`cargo test -p bumbledb --features
  fold-off`), and the bench crate under `obs`. The one SKIPPED line is
  the script's own recorded environmental branch (the
  x86_64-unknown-linux-gnu cross check needs a cross C toolchain LMDB
  links against; the script reports skip-vs-pass honestly by design).
- `cargo build -p bumbledb-bench --release` (fresh) then
  `scripts/check-asm.sh` — **exit 0**, "all gates green": all three
  Allen NEON symbols free of scalar flag writers
  (`cmp`/`csel`/`adds`/`ccmp`) and calls.
- `cargo test` in `fuzz/` — **exit 0**: crash sweep
  (`every_crashpoint_recovers_across_the_prefix_matrix`), crash-corpus
  replay, theory/ops corpus replays, and the query/rewrites corpus
  replays (the long S-scale query replay passed, 395s) — 6 passed, 0
  failed, 0 skipped. The one `#[ignore]` (`crash_child`) is the
  crash-child process body spawned BY the sweep and replay parents — an
  architectural subprocess entry point, not a skipped lane.

**Verdict: PRD 09 green. The melt's terminal reconciliation is counted,
dated, and on the ledger; the new census floor is 121.**
