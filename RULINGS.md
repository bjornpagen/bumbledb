# Rulings

The durable decision ledger. Architecture docs are gone; the laws live in
the code at the site each governs. This file records OPEN items with their
closure triggers, closed-by-ruling entries, feature-register verdicts, and
the standing census rulings. Relitigate only when a recorded trigger fires.

Doctrine: [`REPRESENTATION-FIRST.md`](REPRESENTATION-FIRST.md).

## OPEN

- **Scale-L claims are informational only.** The oracle stamp, ALL-WIN, and
 pinned denominators are earned at scale S. No L corpus exists, so the
 10 ms budget and every L-scale claim bind nothing yet.
 *Trigger: generating the L corpus.*
- **The chain-window class** (interval intersection along paths). Outside
 the landed recursion surface: the intersected window is a created head
 value. Cookbook recipe 24 carries the window in the host frontier.
 *Trigger: a real workload dominated by interval-intersection-along-paths.*
- **Stacked sequential linear lfps** (A finishes; B reads A as a finished
 set). Two least fixpoints, two drivers, or one Query with `List Rec`.
 *Trigger: a workload where host two-prepares is the pain.*
- **Mutual-linear** (several names, each rule ≤1 rec atom). Admitting it
 is a new IR, not a second rec on `Reach`.
 *Trigger: a sighted query that is unnatural as one name and still linear.*
- **Named interior of a finished rec.** This cut inlines into main.
 *Trigger: two main-shaped queries over one rec that want a shared named
 projection without a second prepare, or an inline image that exceeds the
 main pool.*
- **Nonlinear rec at L** (`P(x,z) ← P(x,y), P(y,z)`). Semi-naive still
 agrees; worse TC algorithm at 10⁷ / 10 ms.
 *Trigger: a measured L-scale query where the linear encoding is unnatural
 and `|Δ ⋈ Acc|` still fits 10 ms / `DEFAULT_DERIVED_TUPLES`.*
- **During-walk anti-join of finished tables.** Refused so `NegationInRec`
 covers the one rec. `reachOp_mono` does not witness against a constant
 negated source.
 *Trigger: a workload whose during-walk exclusion cannot be written
 positively.*
- **Declared range/stabbing accelerators.** Time-range, point-membership,
 and overlap scans are O(n) by decision.
 *Trigger: latency budget violation on a range/interval family.*
- **Grounding interval-pair elimination.** Interval-typed statement
 positions refuse grounding elimination.
 *Trigger: a census-style query that would benefit from interval-pair
 elimination.*
- **Dictionary GC.** Interned values are never reclaimed (accepted leak).
 *Trigger: measured dictionary growth dominating store size on a real
 text-churn workload.*
- **The dictionary contraction.** Str-only; a re-expansion is a format
 change.
 *Trigger: a real schema surfacing variable-width binary with genuine reuse.*
- **`M`-key width.** Membership keys carry the full 32-byte blake3.
 *Trigger: a measured write-path or store-size violation attributable to
 `M`-key width.*
- **Incremental image maintenance.** CLOSED — insert-only commits
 extend images copy-on-append; delete-bearing commits rebuild per
 relation. Tombstone/validity-mask route parked.
 *Reopen: a real delete-heavy, latency-sensitive workload.*
- **Vectorized batch size.** Ships 128 (`exec/run.rs` `BATCH`).
 *Trigger: a dedicated batch-size A/B sweep.*
- **Unit-slot determinant halving.** Measurement-owned.
 *Trigger: a measured write-path or store-size violation attributable to
 determinant width.*
- **The multi-process future.** One process per store stands.
 *Trigger: a second process with a legitimate claim on one store.*

## Closed by ruling

- Invariants are statements about queries (functionality, containment,
 capacity). Unique / referential / primary key / check / exclusion /
 cascade / restrict / trigger / deferrable are deleted vocabulary.
- No sugar — the schema surface is raw statements (`->`, `<=`, `==`).
- Interval is the last type; order operators and Min/Max refused on it;
 uuid rejected.
- `bytes<N>` replaced variable `bytes`; six pure value types; dictionary
 is str-only.
- The IR carries negation, point membership, param sets, and Pack. The
 outer join is a documented decomposition, never a node.
- The query surface is the IR, permanently — pure data. Sugar is
 downstream; `schema!` speaks the theory language, never the query
 language.
- WriteTx point reads (`contains`/`get`) against the delta-overlaid
 final-state view; full queries in write transactions are forbidden.
- Plan introspection is harness-only, not embedding API.
- The naive model is required infrastructure — the second oracle.
- No prior on-disk format opens; no migration path exists — ETL is the
 story.
- Non-key and conditional FDs rejected.
- Nominal typing rejected everywhere.
- Fresh is a generation attribute, not a type.
- Dependency enforcement is commit-time, final-state, only.
- No `replace` operation — delete+insert is the idiom.
- Full-width images, pin-at-prepare plans, one process, zero engine
 threads, intra-query parallelism a non-goal, 64-bit only.
- The engine judges satisfaction, never implication (the decidability
 firewall).
- Statements quantify over stored relations, permanently.
- A created value never re-enters a derivation (the creation quarantine).
- Queries stay query-shaped — `Cq | Reach`, budgeted. A deductive
 database is a named non-goal.

## Feature-register verdicts

| # | Feature | Verdict | Trigger |
| --- | --- | --- | --- |
| 1 | Aggregate comparisons (HAVING) | Strong form REJECTED (creation quarantine). Weak form DEFER. | (a) materialize-then-filter budget violation; (b) host-fold register outgrowing one module; (c) any agg-vs-PARAM sighting. |
| 2 | Disjunctive containment | REJECT | An untagged sum pointer over one shared id space whose references must hold at every commit. |
| 3 | Mintable pins | RECOMMEND, sequenced behind primer lattice-cutover | Primer schema rewrite lands; owner flag-vs-funeral ruling. |
| 4 | Graph read-models | REJECT | Long-lived linux worker + shipped linux package + graph workload too big or write-hot to materialize (all must hold). |
| 5 | Tagged-template query notation | REJECT (owner 2026-07-20) | Direct owner reversal only. |
| 6 | Destructured variable mint | RULED AND ADOPTED | — |
| 7 | Measure-keyed Arg | WITHDRAWN | — |
| 8 | Condition trees in `query!` | RULED IN | — |
| 9 | `abandon()` in `db.write` | RULED IN (K12 withdrawn-stay) | — |
| 10 | `Tx.insert` returns changed | RULED IN | — |
| 11 | Resource lifetimes are disposables | RULED IN | — |
| 12 | TS `explain()` | WITHDRAWN as embedding API | — |
| 13 | Closed-column const accessors | RULED IN | — |
| 14 | Estimator precision | DEFER | Post-009 benches showing plan-choice misses that dynamic cover cannot absorb. |
| 15 | Min/Max in capacity law position | REFUSED | A censused workload demanding an extremal per-group law, via `AdmissibleForm`. |
| 16 | Balance laws | RECORDED, unbuilt | A real host asking for a balance constraint. |
| 17 | Capacity laws | RULED AND ADOPTED | Family triggers live on 15, 16, 18. |
| 18 | Temporal capacity | RECORDED, unbuilt | A real host asking for a concurrency or coverage law. |

Keyed get shipped 2026-07-19. Answer ordering/limit withdrawn as engine
surface — hosts sort and slice.

## Standing census rulings

- **audit/17 — snapshot token.** Public API words are `ReadInstance` and
 `Witness`. `Snapshot` / `ForeignSnapshot` cannot return on the named
 surfaces. LMDB/backup/WAL homonyms stay on the allowlist.
- **audit/27 — zero-dyn.** The engine crate carries no `dyn` except the
 pinned `Error::source` exemption (three lines). The census counts them.
- **audit/40 — purged store-and-value tokens.** The retired store-and-value
  API spellings cannot return as live API. History lines may name them
  under the purged/add-back allowlist.
