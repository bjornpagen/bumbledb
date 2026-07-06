# PRD 08 — Three new families: spread, triangle, true balance

Findings fixed (docs/audit/oracle.md): **MEDIUM** "No oracle query ever
contains a cross-atom residual comparison"; **MEDIUM** "No cyclic join
anywhere in the oracle, though 50-validation promises one"; **NOTE** "The
balance family's Sum is not a balance — duplicate amounts collapse".

## Purpose

The family list is the benchmark's identity and the oracle's backbone — and
it exercises neither the residual machinery (earliest-node placement,
per-node evaluation, survivor compaction) nor a cyclic hypergraph (the exact
class where the paper's cover definition was already caught wrong once, per
30-execution's own record). And the family named `balance` folds distinct
(account, amount) pairs, not a ledger balance. Two new gated families and one
corrected one; every digest re-baseline is deliberate and named.

## Technical direction

- **`spread` — the cross-atom residual family.**
  `Q(x, y) :- Posting(transfer = t, amount = x), Posting(transfer = t,
  amount = y), x < y` — the audit's own construction: the self-join shape
  querygen already produces, plus the one thing missing everywhere, a
  cross-atom ordered residual. Exercises `PlacedComparison` placement,
  residual evaluation at the join node, and compaction. Params: none (one
  empty set — a full-relation family like stats) or a transfer-range param
  if runtime at S demands narrowing; decide by measuring the S row count
  first (each transfer has ~2 postings → ~50k result rows: acceptable;
  start param-less). Hand-written golden SQL
  (`SELECT DISTINCT p1.amount, p2.amount FROM ... WHERE p1.transfer =
  p2.transfer AND p1.amount < p2.amount` — write it BY HAND per the
  arbitration rule, pin against `translate`). Kind::Gate.
- **`triangle` — the cyclic family.** A true 3-cycle over the ledger schema
  via self-joins on Posting's three FK fields:
  `Q(a) :- Posting(account = a, instrument = i), Posting(instrument = i,
  transfer = w), Posting(transfer = w, account = a)` — three occurrences,
  three shared variables, cyclic hypergraph, exactly the dynamic-cover
  stress the paper's triangle exposes. Verify the planner/lowering accept it
  (they must — it is a valid conjunctive query; if the DP order or factor
  produces a degenerate plan, that is a finding, not a blocker). Golden SQL
  by hand. Params: none, or one account-range narrowing param if S-scale
  cardinality explodes (measure first: the result is distinct accounts
  appearing in such cycles — bounded by 500 at S; the *work* is the
  question; if the S profile shows it beyond the 10 ms budget class, add a
  `account < ?0` selection param with documented policy). Kind::Gate.
- **`balance` becomes a true balance.** Bind the serial id:
  `Q(a, Sum(amount)) :- Posting(id, account = a, amount), Account(id = a,
  holder = ?0)` — the id binding makes every posting a distinct binding, so
  the fold is the ledger balance, and (audit's observation) the
  distinct-bindings elision engages (unique coverage) — the family now also
  exercises the seen-set-elided aggregate path under the oracle. Update the
  golden (the inner DISTINCT now includes the id column), the family docs,
  and 50-validation's example if it still shows the collapsing form.
- **Registry mechanics:** families::all() grows to ten; every pinned digest
  moves — the family-list digest, QUERIES.md golden, the verify stamp (via
  family digest), the bench report goldens that enumerate families, and the
  tripwire tables (`read_work_is_bounded_by_selectivity` and the rotation
  tripwire need entries and bounds for spread/triangle — derive their bounds
  from corpus constants the way every existing entry does, with the
  derivation in comments). querygen is untouched here (PRD 09 owns the
  generator); these are pinned families.
- **Param policies documented** in each family's `param_policy` string and
  QUERIES.md as always.

## Non-goals

Generator changes (PRD 09); relaxing the every-family-must-win rule (both new
families are gates — if the engine loses one, that is the benchmark doing its
job); JOB-style shapes beyond the one cycle.

## Passing criteria

- Both new families validate, prepare, translate, and their hand-written
  goldens pin byte-for-byte against `translate` (the existing per-family
  golden test extends to ten).
- Verify-S green over ten families — the engine and SQLite agree on spread
  and triangle across their param sets (this is the point: the residual and
  cyclic paths are now oracle-covered every commit).
- The balance rebind: verify green, AND a targeted unit test pinning the
  semantic change — a corpus slice with two equal-amount postings on one
  account shows the sum counting both (engine and translated-SQL both).
- profile()-based structural bounds for the two new families added to the
  tripwire module, derivations in comments; the honesty (est/actual) test
  covers them with pins consistent with its existing tiers.
- Every moved golden/digest re-pinned deliberately, named in the commit
  message. `scripts/check.sh` green.
