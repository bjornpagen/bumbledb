# PRD 06 — Magnitude-first cover cost model

Authority: `30-execution.md` (dynamic cover choice, Free Join §4.4), suite
README finding 3 (the choice half). Depends on PRD 05 (with O(capacity)
iteration gone, the *choice* is the remaining defect); benefits from PRD 02
(post-selection cursors give exact selected counts).

## Purpose

`choose_cover` (`exec/run.rs`) prefers **label over magnitude**: "an Exact
always displaces an Estimate," so a forced `Exact(500)` beats an unforced
`Estimate(7)` and the executor iterates 500 keys probing 7 instead of 7
probing 500. That is the measured wrong-cover in balance. Replace the rule:
**compare magnitudes; the label is only a tie-break.**

## Technical direction

- `Colt::key_count`'s numbers are iteration-cost bounds and the docs must say
  so honestly:
  - `Exact(map.len)` — the true key count (forced).
  - `Estimate(view.len)` at an unforced root and `Estimate(chunk count)` at an
    unforced node are **exact position counts** and therefore *upper bounds*
    on distinct keys — admissible for cost comparison. Rewrite the `KeyCount`
    doc comment: the label means "keys-exact vs positions-upper-bound," not
    "trustworthy vs guess."
- New rule in `choose_cover`:

  ```text
  choose the cover with the smallest count;
  on equal counts, prefer Exact over Estimate;
  on a full tie, lowest subatom index (deterministic).
  ```

  Rationale, documented at the decision site: iterating a cover costs
  O(its keys) (post-PRD 05) plus a probe into every other subatom per key;
  choosing the smaller magnitude minimizes the dominant term regardless of
  label, and an upper bound that is *still* smaller than an exact count is
  still the better side.
- `CoverStats` already records `chosen_exact`/`chosen_estimate` per subatom —
  no schema change; the numbers just start telling a better story.
- Audit the one other consumer of `KeyCount` semantics (batching size logic in
  `run.rs`, if it reads counts) and confirm magnitude use is already correct
  there; note the audit in the commit message.

## Non-goals

Cost models beyond key counts (probe-cost weighting, output estimation — the
DP owns order, this owns per-node iteration). Forcing a side *just* to get an
exact count (never force for bookkeeping; counts must come free).

## Passing criteria

- Unit test at the `choose_cover` level (extend the existing `run.rs` test
  module): a node with subatom A = forced map of 500 keys and subatom B =
  unforced view of 7 positions chooses **B**; swap magnitudes (forced 7 vs
  unforced 500) chooses the forced 7; equal counts choose the Exact side; the
  full tie is deterministic.
- Integration, counters-based (machine-independent): the balance shape —
  `Q(a, Sum(amount)) :- Posting(account = a, amount), Account(id = a,
  holder = ?0)` — over a hand-built corpus of 10,000 postings, 500 accounts,
  and a holder owning 7 accounts of ~20 postings each:
  - `profile()` reports the Account subatom as the chosen cover at the account
    node (`CoverStats` of the Posting subatom shows 0 choices);
  - node `entries` for the account node == 7, and total `batch_entries`
    across nodes ≤ 2 × the holder's posting count — the O(selected) pin that
    was 220 µs of capacity-walking before.
- The eight-family verify suite green (results identical; only work changes).
- `scripts/check.sh` green.
