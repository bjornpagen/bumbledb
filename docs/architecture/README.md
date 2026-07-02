# Bumbledb Architecture

These documents are the normative design. They are living documents, updated in place —
there are no PRD suites, work packets, or compliance gates. Git history is the changelog.

## Rules for these docs

1. **Every decision records its strongest alternative, why it lost, and what evidence
   would reverse it** — one paragraph. If we can't articulate the alternative, the
   decision isn't made yet.
2. **Every deviation from the Free Join paper gets a `Deviation:` block**: what the paper
   says, what we do instead, why, and the reversal evidence. The paper
   (`docs/free-join-paper/`) is algorithmic authority; these docs are product authority.
   This rule covers the query model's divergences from the paper's §2 CQ assumptions,
   not just the execution doc.
3. **Every specified mechanism must name its reader** — a namespace, cache, statistic,
   or counter with no named consumer is deleted. (The anti-transcription rule; see
   post-mortem §22.)
4. **Undecided things are marked `OPEN` with a closure trigger** (the event or milestone
   that forces the decision) and listed below. An OPEN item is a real state; the failure
   mode is code deciding it silently.
5. **When implementation contradicts a doc**, the doc is amended in the same change, or
   the code change doesn't land. Docs describe the system in the present tense.
6. History: the five discarded implementations (v1–v5) live in git before commit
   `1b65ae8`; the post-mortem that motivates many decisions is
   `docs/history/post-mortem.md` (re-materialized; §-references point there).

## The documents

| Doc | Contents |
|---|---|
| `00-product.md` | Thesis, workload numbers, hardware, durability, success criteria |
| `10-data-model.md` | Set semantics, structural types, identity, constraints, schema |
| `20-query-ir.md` | The pure-data IR, semantics, normalization, validation |
| `30-execution.md` | Access paths, Free Join over COLT, planner, vectorization, allocation |
| `40-storage.md` | LMDB layout, write/delete paths, the columnar image cache |
| `50-validation.md` | SQLite oracle, ledger benchmark protocol, test families |
| `60-api.md` | Embedding surface: lifecycle, transactions, results, errors, ETL |

## OPEN items

- **Aggregate execution phasing**: aggregation is fully specified in the IR and sinks;
  when it lands relative to plain joins is unscheduled. The "beats SQLite" claim is void
  until it does. *Trigger: first benchmark milestone.*
- **Negation** (anti-join atoms) and **recursion** (fixpoint): not in v0; no IR decision
  may assume they never come. *Trigger: first real query that needs one.*
- **Ordering/limit conveniences and ArgMax-style aggregates**: presentation-layer;
  results are sets, the host sorts. *Trigger: owner pain.*
- **Declared range accelerators**: time-range scans are O(n) in v0 by decision;
  accelerators return only with a benchmark that demands them. *Trigger: latency budget
  violation on the time-range family.*
- **Text query language**: none; if one returns it is pure sugar lowering to the IR.
  *Trigger: owner want.*
- **EXPLAIN output format**: mechanism decided (30), surface shape not. *Trigger: first
  debugging session.*
- **Vectorized batch size**: 64–256 starting range decided; the number is
  measurement-owned. *Trigger: the ledger benchmark.*
- **`60-api.md` open sub-items**: see that doc's own OPEN list (bulk-import surface
  details, multi-process future, error-payload shapes).

Closed by ruling (2026-07-02, recorded in the owning docs): nominal typing of any kind
(rejected — hard structural typing); Serial as a type (demoted to field generation
attribute); i128 and narrow-integer types (rejected); field-scoped images (full-width
won); plan invalidation by writes (pin-at-prepare won); queries inside write
transactions (forbidden in v0); multi-process access (out of envelope); intra-query
parallelism (non-goal — the engine owns zero threads; inter-query parallelism is the
scaling axis); 32-bit targets (64-bit only); **constraint enforcement timing**
(commit-time invariants on the final state, via the delta write path — `10-data-model`
/ `40-storage`); **`replace`** (unnecessary: operation order is semantically
irrelevant; at most host-side sugar).
