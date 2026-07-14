# The conformance corpus (PRD 13) — the interchange format

One JSON document per case, designed for hand-reading: a case file is
the debugging surface when the three oracles disagree. The Rust
serializer and corpus builder live in
`crates/bumbledb-bench/src/conformance.rs`; the Lean decoder/evaluator
in `Bumbledb/Conformance.lean` (`lake exe conformance cases/`, driver
in `Main.lean`). The evaluation is the DENOTATION: plain projections
run through `evalList`, proved equal to the set denotation by
`eval_sound` (`Bumbledb/Query/Denotation.lean`); aggregate heads run
the recorded glue over PRD 05's proved computable folds
(`Bumbledb/Conformance.lean`, module doc).

## Values (the tagged form)

| tag | example | notes |
|---|---|---|
| `bool` | `{"bool":true}` | |
| `u64` | `{"u64":18446744073709551615}` | full range, exact |
| `i64` | `{"i64":-3}` | |
| `str` | `{"str":2}` | a per-case intern id (see `strings`) |
| `bytes` | `{"bytes":[7,0,255]}` | `bytes<N>`, N = the array length |
| `interval_u64` | `{"interval_u64":[3,10]}` | half-open `[start, end)` |
| `interval_i64` | `{"interval_i64":[0,9223372036854775807]}` | `end = MAX_END` IS the ray `[0, ∞)` |
| `mask` | `{"mask":["before","meets"]}` | params only — the Allen mask value |

Rays need no special spelling: an interval whose `end` is the element
domain's ceiling (`2^64−1` for u64, `2^63−1` for i64) is the ray, on
both sides of the lane (`Interval.isRay`).

## One annotated example

```jsonc
{
"case":"hand-closed-join",                  // the file's identity
"provenance":{"hand":"hand-closed-join",    // how to regenerate it:
              "world_seed":12603137},       //   a named hand case, or
                                            //   {world_seed, case_seed,
                                            //    draw} replayed through
                                            //   Rng::new(case_seed)
"strings":[],                               // the used slice of the
                                            // intern dictionary:
                                            // [id, "text"] pairs —
                                            // hand-readability only;
                                            // Lean compares ids
"theory":{
  "relations":[                             // mentioned relations only
    {"id":1,"name":"Account","closed":false,
     "fields":["u64","u64","u64"]},         // positional field types
    {"id":13,"name":"Currency","closed":true,
     "fields":["u64","u64"]}],              // field 0 = the synthetic id
  "ground_axioms":[                         // closed relations' sealed
    {"relation":13,"facts":[                // extensions — ordinary
      [{"u64":0},{"u64":2}],                // facts to the matching
      [{"u64":1},{"u64":2}],                // equation
      [{"u64":2},{"u64":0}]]}]},
"instance":[                                // the open relations the
  {"relation":1,"facts":[                   // query mentions (and no
    [{"u64":0},{"u64":0},{"u64":0}],        // more: snapshot_single —
    [{"u64":1},{"u64":0},{"u64":2}],        // the denotation reads
    [{"u64":2},{"u64":0},{"u64":2}],        // nothing else)
    [{"u64":3},{"u64":0},{"u64":2}],
    [{"u64":4},{"u64":0},{"u64":1}]]}],
"query":{"rules":[                          // the IR, serialized
  {"finds":[{"var":0},{"var":1},{"var":2}], // head positions: {"var"},
                                            // {"measure"}, {"agg":{…}},
                                            // {"agg_measure":{…}}
   "atoms":[                                // [field, term] bindings;
     {"relation":1,"bindings":[[0,{"var":0}],[2,{"var":1}]]},
     {"relation":13,"bindings":[[0,{"var":1}],[1,{"var":2}]]}],
   "negated":[],                            // anti-join atoms
   "conditions":[]}]},                      // {"cmp":{op,lhs,rhs}} |
                                            // {"and":[…]} | {"or":[…]};
                                            // allen carries "mask" or
                                            // "mask_param" beside "op"
"params":[],                                // positional: {"scalar":v} |
                                            // {"set":[v…]} | {"mask":[…]}
"answers":[                                 // the ENGINE's answers,
  [{"u64":0},{"u64":0},{"u64":2}],          // canonically sorted (below)
  [{"u64":1},{"u64":2},{"u64":0}],
  [{"u64":2},{"u64":2},{"u64":0}],
  [{"u64":3},{"u64":2},{"u64":0}],
  [{"u64":4},{"u64":1},{"u64":2}]]
}
```

## Canonical answer order

Each row renders to its compact tagged form (exactly as above, no
whitespace); rows sort by lexicographic byte order of that rendering;
duplicates cannot exist (set semantics). The serializer writes the
`answers` block in this order, and the Lean side re-renders BOTH its
own answers and the decoded `answers` block with its own renderer
before comparing — the comparison is value-level, so a cross-language
byte-format drift cannot silently pass or fail a case.

## The membership lowering (why a query file can differ from the IR)

The engine's membership BINDING is a typing rule, not a syntax node:
an element-typed term on an interval field means point membership,
resolved by the validator (`ir/validate/context.rs::resolve_bivalents`).
The Lean matching equation reads every binding as value selection, so
the serializer performs the same resolution: such a binding becomes a
fresh interval variable plus a `PointIn` condition — the predicate form
the typing rule licenses. The engine executes the original query; Lean
evaluates the lowered one; their agreement is part of what the lane
checks.

## Scope fences (recorded exclusions — counted, never silent)

The corpus is Tiny-scale, valid-arm only. Per-build coverage is logged
by the builder and the comparator (`Report::coverage_line`); the
checked-in corpus was built at **217/323 expressible** (200 seeded +
17 hand cases):

* **hostile arm** — not drawn at all: structurally-free IR types
  nothing and belongs to the validation-totality fuzz lane.
* **unresolved string literals** (31) — the model has no intern
  dictionary; a query/param string outside the world's vocabulary is
  the engine's dictionary-miss latch, excluded on principle.
* **negated-atom membership** (5) — the membership lowering has no
  home inside an anti-join atom (no fresh variable may bind there).
* **element-typed param-set membership** (0 this build) — the lowered
  `PointIn`-with-set shape would violate the `WellTyped` premise
  `eval_sound` names.
* **engine runtime errors** (0 this build) — `Overflow` /
  `MeasureOfRay`: the lane compares answer sets on error-free
  executions only (the model reads a ray's measure as `none`; the
  engine raises — the recorded Level-0 narrowing).
* **slow** (61) and **wide** (9) — naive wall time over 25 ms or
  answers over 512 rows: per-push CI budget; shrink the case, never
  the model.

## Regeneration and the three-way comparator

* Regenerate: `cargo test -p bumbledb-bench
  regenerate_the_conformance_corpus -- --ignored --nocapture`
  (deterministic: identical bytes from identical seeds, forever).
* Compare (engine · naive · file bytes): `cargo test -p bumbledb-bench
  the_corpus_replays_byte_identical` — runs in the plain workspace
  suite.
* Compare three ways (adds the Lean run): `cargo test -p
  bumbledb-bench three_way_conformance -- --ignored --nocapture`
  (needs `lake` on PATH; the CI lean lane runs it).
* Lean alone: `lake exe conformance conformance/cases` from `lean/`
  (wired into `scripts/lean.sh`).

A DISAGREEMENT IS A TROPHY — engine bug, naive-model bug, or spec bug
all count; triage per the fuzzing charter
(`docs/architecture/60-validation.md`). Report prominently; never
"fix" the corpus to make a disagreement go away.
