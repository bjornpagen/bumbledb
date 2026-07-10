# PRD 07 — The chase-evaluator: folding stage-zero atoms

**Depends on:** 03 (virtual images), 04 (the sealed extensions' compiled
form), 05 (post-enum IR), 06 (the fold-shaped differential family exists
BEFORE the fold — the oracle precedes the optimization, per the shipping
law).
**Modules:** `crates/bumbledb/src/plan/chase.rs` (the pass), `ir/normalize/`
(the filter shapes it emits into), `api/prepared/build.rs` (ordering),
`api/stats.rs` (EXPLAIN).
**Authority:** `40-execution.md` (the elimination pass), the staging law.
**Representation move:** the chase generalizes from *eliminator* to
**evaluator**. Elimination removes atoms that statements prove redundant;
evaluation removes atoms whose extension is stage-0-known by *running them
at prepare*. `Kind(id: k, mastered == true)` is not a join to plan — it is a
three-element id-set computed before the DP ever sees the query, residual
cost zero.

## Context (decided shape)

A positive occurrence `C` of a closed relation is **foldable** when every
one of these holds (strict; any failure leaves the atom to join against its
virtual image, which is cheap and always correct):

1. Every variable bound by `C` except at most one is *dead outside `C`* (no
   head use, no other occurrence, no residual/anti-probe/point-probe use).
   The at-most-one live variable must be bound at `C`'s **id position** —
   call it the join variable `k`.
2. `C` carries only Eq/range/Allen/membership filters over its own columns
   with constants resolvable at prepare (literals and closed-column values;
   param-bearing filters on `C` defer the fold to a bind-time variant that
   is REFUSED v0 — recorded, trigger: a measured win in the calendar-family
   profile).
3. `C` is not negated (a negated closed atom folds differently — see
   direction 4).

**The fold:** evaluate `C`'s filters against the sealed extension at
prepare, producing the surviving id-set `S`. Then:

- If `k` exists and `|S| ≥ 1`: delete `C` (a `Role::Folded(id_set)` mark on
  the occurrence, sibling of `Role::Eliminated`) and attach `S` to every
  OTHER occurrence binding `k` as a **membership filter** — exactly the
  existing param-set selection machinery (`FilterPredicate` membership /
  set-bound selection levels), except the set is plan-constant. Small `|S|`
  rides selection levels (k probes + survivor union); large `|S|` rides the
  membership filter kernel. The machinery chooses exactly as it does for
  param sets today; nothing new executes.
- If `|S| == 0`: the rule is **statically empty** — mark it dead (rules-IR:
  the clause drops from execution; a query whose every rule is dead produces
  the empty plan — coordinate with PRD 10's `ExecPlan::Empty`).
- If `k` does not exist (the atom was a pure guard, e.g. a nonemptiness
  gate over a subset): `|S| ≥ 1` deletes the atom outright; `|S| == 0` is
  the statically-empty case.
- **Negated closed atoms** (direction 4): `!Kind(id: k, mastered == true)`
  with `k` bound positively folds to membership in the COMPLEMENT set
  (extension minus `S`) — same machinery, complement computed at prepare.

**What does NOT fold, deliberately:** a closed atom with a live non-id
variable (payload escaping to the head — "return each event's severity
rank") keeps its join against the virtual image: the join is L1-resident,
generation-immortal, and the DP prices it honestly. Folding payload
projection would require value substitution into the head — a rewrite class
with real complexity and no measured need. Refused, recorded, trigger: the
calendar family showing vocabulary-join cost above noise.

## Technical direction

1. `chase.rs`: the evaluator runs INSIDE the existing fixpoint, as a new
   rule alongside elimination (folding can expose eliminations and vice
   versa; the fixpoint already iterates). Conditions implemented as
   standalone predicates with the same naming discipline as
   `join_covers_full_key`/`target_otherwise_unused` (one function per
   condition, unit-tested in isolation — weaker-model note: do not inline
   them into one boolean expression).
2. Filter evaluation at prepare: reuse the commit path's value comparison
   over pre-encoded extension values (PRD 04 built it in validate; extract
   the shared helper rather than duplicating — one mechanism, two callers).
   Allen filters on interval payload columns evaluate via the existing
   scalar `classify` reference (not the batch kernel — n ≤ 256).
3. The `Role::Folded` mark: like `Eliminated`, occupancy never moves; stats,
   DP, view binding, image builds all skip it via the one `participates()`
   predicate (verify each site already routes through it — the elimination
   PRD's invariant).
4. Membership attachment: emit into the normalized occurrence's filter list
   as a plan-constant set; `resolve_predicates` treats plan-constant sets as
   pre-resolved (no per-execution work — coordinate with PRD 09's latch so
   the fully-resolved fast path recognizes them).
5. EXPLAIN: folded occurrences reported with the licensing extension, the
   filter, and `|S|` ("folded: Kind{mastered==true} → 3 ids"), the
   Eliminated-reporting precedent.

## Passing criteria

- `[test]` Each foldability condition has a positive and a negative unit
  test against hand-built normalized queries (six tests minimum: live
  payload var blocks; param filter blocks; negation routes to complement;
  dead-guard deletes; empty-set kills the rule; multi-rule query folds
  per-rule independently).
- `[test]` Differential: PRD 06's fold-shaped family, engine vs both
  oracles, with the chase's test-only off-switch extended to cover the
  evaluator — results byte-identical folded vs unfolded across the
  randomized corpus (the fold is never semantic).
- `[test]` The complement fold: a negated subset atom agrees with the naive
  model across the corpus.
- `[shape]` Folded occurrences never build images or bind views (extend the
  elimination PRD's zero-build assertion via obs counters); EXPLAIN carries
  the fold line; no new executor code exists (grep: the fold emits only
  existing `FilterPredicate`/selection shapes).
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

`40-execution.md`: the elimination section becomes "the chase: elimination
and evaluation," with the foldability conditions, the payload refusal, and
the complement rule.
