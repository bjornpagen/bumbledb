# Bumbledb Architecture

These documents are the normative design. They are living documents, updated in place —
there are no PRD suites, work packets, or compliance gates. Git history is the changelog.

## Rules for these docs

1. **Every decision records its strongest alternative and why it lost** — one paragraph
   each. If we can't articulate the alternative, the decision isn't made yet.
2. **Every deviation from the Free Join paper gets a `Deviation:` block**: what the paper
   says, what we do instead, why, and what evidence would reverse it. The paper
   (`docs/free-join-paper/`) is algorithmic authority; these docs are product authority.
3. **Undecided things are marked `OPEN`** and listed below. An OPEN item is a real state,
   not a failure — the failure mode is code deciding it silently.
4. Docs describe the system we are building, in the present tense, plus explicitly-marked
   history where the old system's failure motivates a choice. The five discarded
   implementations (v1–v5) live in git history before commit `1b65ae8`. The 34-file
   post-mortem review of that code was never committed (it was purged from the working
   tree); its load-bearing conclusions are restated inline in these docs wherever a
   decision rests on them, and `00-product.md` through `50-validation.md` cite them as
   "post-mortem" findings.

## The documents

| Doc | Contents |
|---|---|
| `00-product.md` | Thesis, owner, workload, scale, target hardware, non-goals |
| `10-data-model.md` | Set semantics, types, constraints, identity, schema |
| `20-query-ir.md` | The pure-data query IR; what replaced the text language |
| `30-execution.md` | Free Join over COLT, planner, vectorization, allocation contract |
| `40-storage.md` | LMDB layout, write path, the columnar image cache |
| `50-validation.md` | SQLite oracle, ledger benchmark, differential tests |

## OPEN items (the honest list)

- **Write surface**: FK enforcement timing (commit-time vs per-operation), whether a
  `replace` convenience exists. Deferred until engine internals are settled.
- **Aggregate execution phasing**: aggregation is in the IR and the sink design from day
  one; when its execution lands relative to plain joins is unscheduled.
- **Nominal scalar domains** (`I64 as "UsdCents"`): proposed to close the
  silent-unification hole left by dropping first-class Decimal/Timestamp; undecided.
- **Negation** (anti-join atoms) and **recursion** (fixpoint): not in v0; nothing in the
  IR may assume they never come.
- **Text query language**: none for now. If one returns, it is pure sugar lowering to the
  same IR (Logica's syntax remains the inspiration; its bag/null semantics stay rejected).
- **ArgMax-style aggregates** and **ordering/limit**: presentation-layer questions,
  deliberately unspecified.
