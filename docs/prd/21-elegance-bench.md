# PRD 21 — Elegance: bench crate

**Depends on:** 20. Final PRD; the campaign closes after it with the
orchestrator's re-bench (hot paths moved across 12–16) and chart regeneration —
which are not PRD content.
**Binding constraints:** the README's elegance-pass block, plus one crate-local
law: **the naive model's independence is inviolable** — no engine algorithm may
be shared into `naive/`, and no `naive/` logic may migrate out to be shared. Its
only permitted imports remain schema/IR/value *types*. Deduplication findings
that would merge model logic with engine or translator logic are refused by
design; record them as refused in the findings list.
**Modules:** `crates/bumbledb-bench/src/` — all of it: naive, translate, sqlmap,
sqlite_run, querygen, families, gen, corpus, verify, driver, harness, cli,
report, compare, tripwires, clockproxy, json, writebench, scenarios.

## Subsystem-specific hunt list (verify, don't assume)

- **Three eras of family definitions:** the ported ten, the five new, and the
  chase-coverage shapes (PRD 12) — converge the family-definition
  shape (IR constructor + SQL golden + rotation + index DDL) into one table-
  driven registration if the current registration repeats boilerplate per
  family; the family list appears in families, driver, report, tripwires, and
  viz ordering — count the places the list is spelled and reduce to one.
- **Hand-rolled utilities:** randomness, stats, JSON emission, and arg parsing
  are hand-rolled by policy — check for *multiple* hand-rollings (two RNG
  helpers, two percentile implementations) across querygen/gen/harness/report;
  one of each.
- **SQL rendering:** the translator's rendering helpers vs sqlmap's DDL
  rendering vs report's markdown tables — alias bookkeeping and identifier
  quoting should each exist once within the SQL lane.
- **Verify lanes:** the SQLite lane, the naive lane, and the empty-store pass
  accreted sequentially — check `verify/` for per-lane duplicated corpus
  loading, stamp plumbing, and comparison scaffolding; the lanes should share
  the run harness and differ only in the executor pair.
- **Scratch-harness residue:** the rebuild-era agents left pruned-scratchpad
  workarounds and comments referencing tree states that no longer exist —
  delete every such comment; test helpers that existed only to dodge
  then-broken modules get inlined or removed.
- **CLI/help drift:** help text, README recipe, and actual flags — one pass to
  confirm they agree (the help text is user-facing surface; PRD 10
  added a subcommand).

## Passing criteria

As PRD 16's, applied to this crate. Additionally:
- `[shape]` The family list is spelled in exactly one place (or the findings
  list justifies each additional spelling).
- `[shape]` `naive/` imports nothing beyond schema/IR/value types (grep), and
  any refused dedup findings are recorded.
- `[gate]` Workspace gates green; `bumbledb-bench verify` runs green
  (orchestrator-executed, recorded here as the campaign-closing gate together
  with the re-bench and chart regeneration).
