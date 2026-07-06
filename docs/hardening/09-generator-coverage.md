# PRD 09 — The generator earns its coverage contract

Findings fixed (docs/audit/oracle.md): **MEDIUM** "Gates are only ever tested
in the always-true direction; no relation is ever empty"; **MEDIUM**
"Aggregates over U64 are never generated, and the U64 rule has a hole for
sums"; **MEDIUM** "The generator's asserted coverage contract is weaker than
the documented one" (the full enumerated hole list); **LOW** "Boundary param
sets probe only domain minima".

## Purpose

50-validation promises a feature-coverage contract — "every comparison op on
every legal type, empty relations, …" — and the audit enumerated exactly
what the generator provably never produces. Close every hole it names, add
the empty-store pass that makes gate-falsity and empty-relation semantics
oracle-checked, and upgrade the asserted contract from per-op totals to the
per-(op, type) matrix so the contract can never quietly weaken again.

## Technical direction

- **The empty-store verify pass.** The elegant fix for "no relation is ever
  empty / gates never false": `verify::run_prepared` gains a second,
  *cheap* phase — a fresh store pair (bumbledb + SQLite) with the schema
  loaded and **zero rows anywhere**, over which every family (all ten) and a
  seeded slice of randomized queries (e.g. 100) run and compare. Every gate
  is false, every scan is empty, every aggregate folds nothing (the
  empty-set-not-NULL rule and the HAVING template earn their keep), every
  selection misses. Costs milliseconds; covers the entire empty-relation
  semantic surface at once, with zero corpus/digest churn. The stamp's case
  accounting includes the empty-store cases.
- **Generator matrix extensions** (`querygen.rs`), each enumerated by the
  audit:
  - **Cross-atom residuals:** the self-join shape gains (50% of the time) an
    ordered comparison between the two amounts (`x < y` / `x >= y` …) —
    both sides i64 by construction; this is the randomized twin of PRD 08's
    spread family.
  - **U64 ordered dressing:** Lt/Le/Gt/Ge on u64 id-typed fields
    (posting.account/instrument/transfer domains) — literals and params
    drawn in-domain; boundary flavor includes `domain` and `domain - 1`.
  - **U64 aggregates:** Sum/Min/Max over u64 fields with provably bounded
    sums (sum of account ids over any group ≤ accounts² ≪ 2⁶³ at every
    scale — state the bound in a comment as the Sum-range rule requires),
    plus Count alongside.
  - **Bytes Eq/Ne:** dressing on `Transfer.extref` — the in-vocabulary hit
    literal is the *actual* extref of a seeded row (recompute via
    `gen::row`), the miss is a fresh 16-byte value; params likewise (the
    currently-dead Bytes arm of `param_value` comes alive).
  - **Ne on enums, bools, and i64:** extend the existing arms beyond Eq.
  - **U64 param misses:** the `SetKind::Miss` arm for u64 draws
    out-of-domain (`domain + offset`), matching the family policies.
  - **Boundary maxima:** the Boundary flavor alternates minima and maxima
    (domain-1, window `hi`, last enum ordinal, `true`).
  - **Gate diversity:** gates draw from {Tag, TagNote} (both non-empty in
    the corpus — falsity is the empty-store pass's job; diversity here is
    about relation shape).
- **The contract test becomes the matrix.** `Coverage` grows per-(op, type)
  counters (a small fixed table: 6 ops × {u64, i64, enum, bool, string,
  bytes} where legal); the contract test asserts every *legal* cell > 0 at
  n = 1000, plus the new construct counters (cross-atom residuals, u64
  aggregates, bytes hit/miss, param misses per type). Re-derive the
  determinism pin (query #500 changes — deliberate re-pin).
- **50-validation amendment:** the coverage contract paragraph is rewritten
  to state exactly what is now asserted (the matrix, the empty-store pass,
  the Sum-range corollary: "generators must bound reachable sums below
  2⁶³"), closing the README-rule-5 violation the audit flagged. The
  enumerated remaining non-goals (three-plus occurrences of one relation,
  multi-aggregate randomized finds, bool group keys, gate+aggregate
  combinations) are either added cheaply — gate+aggregate is one line in
  the Gated arm; multi-aggregate finds is a small extension worth taking;
  bool group keys ride the aggregate group-key chooser — or explicitly
  listed as out with a reason. Decide per item in this PRD; the doc must
  match the code either way.

## Non-goals

Corpus/schema changes (the empty-store pass makes the empty-relation corpus
variant unnecessary); negation/recursion; adversarial NUL literals (excluded
by PRD 07's translator boundary, asserted there).

## Passing criteria

- The empty-store pass runs inside `verify::run_prepared` (families + 100
  randomized cases against empty stores), is counted in `VerifyReport.cases`,
  and verify-S stays green — with a seeded assertion that at least one
  gate-bearing query executed against the empty store (gate-false coverage
  is structural, not incidental).
- The upgraded coverage contract passes at n = 1000: every legal (op, type)
  cell nonzero, every new construct counter nonzero, shape bands within the
  existing ±30% discipline; the determinism pin re-derived.
- The thousand-query validate+translate test still passes (the grammar's
  totality holds through every extension — Bytes literals, u64 ranges,
  cross-atom residuals all translate).
- Differential pin: the new constructs flow through the prepared-level
  seeded differential (extend `selection_params_rotate_differentially`'s
  pattern or the run.rs differential where appropriate) — at minimum, u64
  ordered comparisons and cross-atom residuals each get one deterministic
  nested-loop-checked case in the engine crate, independent of SQLite.
- 50-validation's contract paragraph matches the asserted matrix exactly.
- `scripts/check.sh` green (full verify-S with the enlarged case set).
