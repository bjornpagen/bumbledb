# The conformance corpus (PRD 13; judgment + recursive arms) — the interchange format

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

Three case kinds share the directory, dispatched by FILE NAME:
`judgment-*.json` is a **judgment case** (the write-side arm, below);
`program-*.json` is a **program case** (the recursive arm, below);
everything else is a **query case**. A judgment case also carries
`"kind":"judgment"` for self-description.

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
| `interval_u64_fixed` | `{"interval_u64_fixed":[3,5]}` | `[start, width]` — the width is the TYPE'S (`interval_u64_fixed<5>` in the field list); the decoder re-checks the Q2 bound `start + w < MAX_END` and `w ≥ 1`, refusing at-bound/past-bound starts and `w = 0` (`Conformance.lean`'s ceiling `#guard`s) |
| `interval_i64_fixed` | `{"interval_i64_fixed":[3,5]}` | the i64 twin — never a ray, by the same bound |
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

The query corpus is Tiny-scale, valid-arm only. Per-build coverage is
logged by the builder and the comparator (`Report::coverage_line`);
the checked-in corpus was built at **219/325 expressible** (200 seeded
+ 19 hand cases), plus the 24 hand judgment cases outside the report
(they have no expressibility gate):

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
* **membership under an additive fold** (0 this build) — a fired
  lowering under a `Count`/`Sum` head (scalar or measure): the fresh
  interval variable enters the Lean fold domain (`Conformance.lean`'s
  `ruleBindings` spans `body.allVars`) that neither the engine
  (membership is a filter, never a binding) nor the naive model has —
  `membership_lowering_preserves` licenses set-semantics answers only,
  so the combination is refused representation rather than left to
  querygen's accidental non-overlap (today's corpus has zero
  aggregate-plus-membership cases by generator shape alone).
* **engine runtime errors** (0 this build) — `Overflow` /
  `MeasureOfRay`: the lane compares answer sets on error-free
  executions only (the model reads a ray's measure as `none`; the
  engine raises — the recorded Level-0 narrowing).
* **slow** (61) and **wide** (9) — naive wall time over 25 ms or
  answers over 512 rows: per-push CI budget; shrink the case, never
  the model.

## Judgment cases — the write-side third oracle

A judgment case compares COMMIT VERDICTS instead of answer sets: the
Lean side decodes `(theory, instance, delta)`, applies the delta by
row-set arithmetic (deletes removed, inserts added, no-ops cancelling
— `NaiveDb::staged`'s arithmetic), and runs the PROVED executable
judge `Txn.judgeB` (`Bumbledb/Decide.lean`), which agrees with the
model's `Txn.judge` verdict and violation sets phase for phase
(`Txn.judgeB_agrees_of_declared`). The Rust serializer
(`crates/bumbledb-bench/src/conformance/judgment.rs`) writes each
document only after the ENGINE and the NAIVE MODEL agreed on the
verdict, so the corpus run is the full three-way comparison. Every
fixture is hand-authored (judgment cases are theorem-shaped, not
distribution-shaped); replay rebuilds each by its provenance name.

The document shape (values, facts, and the `relations` block exactly
as in query cases; a closed relation's sealed field list opens with
the synthetic id, and its ground-axiom facts carry the row id at
position 0):

```jsonc
{
"case":"judgment-window-floor-childless",
"kind":"judgment",
"provenance":{"hand":"judgment-window-floor-childless"},
"theory":{
  "relations":[…],                          // as in query cases
  "ground_axioms":[…],                      // as in query cases
  "statements":[                            // the MATERIALIZED list —
                                            // indices ARE the engine's
                                            // statement ids
    {"functionality":{"relation":0,"projection":[0]}},
    {"containment":{"source":SIDE,"target":SIDE}},
    {"cardinality":{"source":SIDE,
                    "window":{"lo":1,"hi":2},   // "hi" absent = *
                    "target":SIDE}}]},
                                            // SIDE = {"relation","projection",
                                            //   "selection":[[field,[lit…]]…]}
                                            // — a literal SET reads
                                            // disjunctively
"instance":[…],                             // the committed pre-state
                                            // (green by construction)
"delta":{"deletes":[…],"inserts":[…]},      // {relation, facts} blocks
"verdict":"accept"                          // or:
// "verdict":{"reject":{"phase":"key","violations":[0]}}
// "verdict":{"reject":{"phase":"statement","violations":[2,3]}}
}
```

The verdict compares WHOLE, per phase: a key (functionality) violation
preempts the statement phase on all three oracles
(`Bumbledb/Txn.lean: judge_key_preempts`); `violations` is the
rejecting phase's complete set as ascending statement indices. The
containment `Direction` is a Rust-side refinement below the Lean
altitude (the Lean violation sets are per-statement), so a statement
cited in both directions appears once. Closed-relation writes are
outside this lane (a typed refusal before any judgment, not a
verdict); judgment fixtures carry no strings and no masks — the two
value tags that would need a per-case context. Closed-SOURCE
containments (domain quantification) are also outside the lane, and
deliberately: the engine's verdict is delta-restricted
(`Bumbledb/Txn/DeltaRestriction.lean: delta_restricted_commit_sound`,
sound only under its holds-before premise) while `Txn.judgeB` reads
the whole final state — a store whose targets have not landed accepts
every untouching commit yet judges reject in full state
(`Bumbledb/Countermodels.lean: incremental_verdict_needs_holds`; the
offline sweeper owns the class per
`docs/architecture/30-dependencies.md` § "Domain quantification,
worked"), so such a fixture would be a guaranteed mismatch on a
correct engine verdict. No fixture declares a closed source; the Rust
half is pinned by
`domain_quantification_judgments_are_outside_the_lane`.

The starter roster covers: both classical forms (scalar key;
containment — scalar, coverage, and the closed member set), the
extension form (windows at floor/ceiling/`n..n`/`0..*`/empty-parent),
the two-phase preemption mix,
set-selections deciding a verdict, the delete-then-reinsert
touched-group seam, and the permuted-interval lock — a statement
written `Claim(span, id) <= Slot(span, id)` against the pointwise key
DECLARED `(id, span)`: accepted through the set-canonical key
resolution (`Bumbledb/Schema.lean: Header.intervalSplit`), judged as
coverage, three-way agreed. The whole-list comparison surface is
exercised beyond singletons: a statement phase citing containment AND
cardinality as the ascending pair (`judgment-statement-mixed-citations`,
`[1,2]`), one containment cited in both directions and collapsed to one
id (`judgment-containment-both-directions` — the dedup rule above,
pinned pre-dedup by a unit test), and a two-key rejection
(`judgment-multi-key-collisions`, `[0,1]`).

## Program cases — the recursive third oracle

A program case carries the program cut instead of a query
(`Bumbledb/Query/Syntax.lean`: `Program`/`PredicateDef`/`PRule` —
rule heads are plain variable-id lists, so the recursive corpus is
projection-shaped by format; folds are the naive lane's, exactly as
`Pack` is on the query side). The Lean side decodes it, reads the
RECORDED stratification witness (the Rust side computes one witness —
the naive model's relaxation, mirroring the strata judge's
condensation; the denotation is witness-independent, the recorded
narrowing in `Bumbledb/Exec/Fixpoint.lean`), and runs the PROVED
fueled fixpoint `evalProgram` (`program_eval_sound` is its agreement
with the stratified denotation) against the recorded answers.

The Rust builder (`crates/bumbledb-bench/src/conformance/program.rs`)
writes each document only after the NAIVE stratified fixpoint answered
and — where the `WITH RECURSIVE` gate admits
(`translate::sqlite_program_expressible`) — `SQLite` agreed; mutual
and non-linear cases are naive-attested and still written (Lean judges
them too — exactly the coverage `SQLite` cannot give). The corpus
programs read the org tree only (closure sizes bounded by
construction), so every case stays hand-readable and the Lean run
stays a per-push lane.

```jsonc
{
"case":"program-hand-closure",
"provenance":{"hand":"program-hand-closure","world_seed":12603137},
                                            // or {world_seed, case_seed,
                                            //     variant} replayed through
                                            //     Rng::new(case_seed)
"strings":[…],                              // as in query cases
"theory":{…},                               // as in query cases
"instance":[…],                             // as in query cases
"program":{
  "predicates":[                            // PredId = index
    {"arity":2,"rules":[
      {"finds":[0,1],                       // PRule.finds: variable IDS
       "atoms":[                            // the source arm spelled:
         {"edb":7,"bindings":[[0,{"var":0}],[1,{"var":1}]]}],
       "negated":[],"conditions":[]},
      {"finds":[0,2],
       "atoms":[
         {"edb":7,"bindings":[[0,{"var":0}],[1,{"var":1}]]},
         {"idb":0,"bindings":[[0,{"var":1}],[1,{"var":2}]]}],
       "negated":[],"conditions":[]}]}],
  "output":0,
  "strata":[0]},                            // the recorded witness
"params":[],
"answers":[…]                               // the AGREED answers,
                                            // canonically sorted
}
```

## Regeneration and the three-way comparator

* Regenerate: `cargo test -p bumbledb-bench
  regenerate_the_conformance_corpus -- --ignored --nocapture`
  (deterministic: identical bytes from identical seeds, forever).
  The recursive arm regenerates independently
  (`regenerate_the_recursive_conformance_corpus` — `program-*.json`
  only, leaving the query lane's measured budgets untouched).
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
