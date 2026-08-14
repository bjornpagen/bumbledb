# audit/ — the representation-finish campaign

The Program → Query cutover landed (main `fef913b6`, all suites green).
This folder is the audit that judged the cut against one law —
representation over control flow — and the process for finishing what
the cut started. The verdict, in one sentence: **the cut renamed
Program but kept its coordinate system**; recursion is still an
`Option`/bool/length-pun instead of a sum, proofs are validated then
discarded, two denotations and two builders survive, and the docs still
teach the deleted language.

Nothing in this folder is a fix. Fixes are forbidden until the Gate
below is passed.

## Files

| File | What it is |
|---|---|
| `00-representation-is-the-essence.md` | The doctrine. Required reading for every agent. SPOV 1–3, Insights 1–16. |
| `CONTRACT.md` | The pinned cross-cutting representation decisions (C1–C8). The single authority every fix implements. |
| `lean.md` | Wave-1 dump: 18 findings (H1–H6, M1–M8, L1–L4) in `lean/`. |
| `engine.md` | Wave-1 dump: 40 findings (F1–F40) in `crates/bumbledb/` + bench oracles. |
| `sdks.md` | Wave-1 dump: 21 findings (#1–#21) in `crates/bumbledb-query/`, `ts/`, `cpp/`. |
| `docs.md` | Wave-1 dump: 28 findings (F1–F28) in `docs/` + lean READMEs. |
| `issues/` | One file per issue (to be produced). The unit of fanout. |
| `issues/INDEX.md` | The ledger: every issue, severity, dependencies, wave, parallel group. |

## Process: A → B → C, in order

### Phase A — finish the audit

1. **Explode.** Every wave-1 finding becomes one file in `issues/`,
   named with its source id for traceability — `eng-F01-<slug>.md`,
   `lean-H1-<slug>.md`, `sdk-01-<slug>.md`, `docs-F01-<slug>.md`;
   wave-2 findings continue each tree's numbering (`eng-F41-…`,
   `proc-01-…`) — in the format below. A partially-refused finding
   (the C5 splits) is ONE file: status `OPEN`, with the refused half
   named `REFUSED-BY-CONTRACT(C5)` inside its Fix section. A finding that CONTRACT.md refuses (the C5
   R-DENSE ruling refuses the Fin-telescope halves of Lean
   H3/H4/H6/M7/M8) still gets a file, with status
   `REFUSED-BY-CONTRACT(C5)` and the surviving dual-coordinate half
   split into its own issue. Duplicates get `DUPLICATE(id)`. Findings
   believed wrong get `DISPUTED` plus one sentence of why.
2. **Hunt (wave 2).** A second adversarial pass over what wave 1
   under-covered, same doctrine, filing new issues directly:
   - engine: `plan/` (fj, ground, selectivity internals), `exec/`
     (wordmap, introspection internals), storage/snapshot seams,
     `obs`, error payload shapes;
   - Lean: `Membership`, `Aggregates`, `Dedup`/`Rewrites` beyond the
     cited theorems, `Bridge` rows, `Countermodels` inhabitants;
   - SDKs: napi marshal internals (`ts/crate`), `raii.cc`, compile-fail
     coverage vs the phase machines, fingerprint files;
   - scripts/CI: `scripts/lean.sh`, `scripts/check.sh`, corpus
     generators — anything that encodes the old coordinate as process.
3. **Index.** `issues/INDEX.md`: id, title, severity, tree, status,
   `blocks` / `blocked-by`, wave number, parallel group.

### Phase B — the Gate (all must hold before any fix commit)

- [ ] Every wave-1 finding maps to ≥1 issue file or a
      `DUPLICATE`/`DISPUTED`/`REFUSED-BY-CONTRACT` entry; counts
      reconcile in INDEX.md against the four dumps (18/40/21/28).
- [ ] Every issue's **Fix** section cites CONTRACT.md by section
      (C1–C8). An issue the contract does not cover forces a contract
      amendment first — the contract is edited, then the issue cites
      it. No fix may cite anything else as authority.
- [ ] Dependencies in INDEX.md form a DAG; every issue has a wave.
- [ ] Acceptance criteria in every issue are mechanical: greps that
      must return empty, compile-fail fixtures named, test commands
      listed, existing tests that must pass **unchanged**.
- [ ] The audit folder is committed, so fix commits can reference
      issue ids.

### Phase C — fix, in waves

Wave assignments live in INDEX.md; the structural rule is:

- **Wave 0 (the sums):** the core representation issues everything
  else lands on — engine witness + prepared pipeline sums (eng: F1,
  F2, F3/F28, F4, F5, F16 cluster per C2/C3), the Lean `Query` sum +
  typed `LinearRec` (lean: H1, H2, C4). Lean and engine run in
  parallel: separate trees, one contract.
- **Wave 1 (downstream, per tree):** everything that collapses once
  the sums exist — engine scratch/stats/introspection/selectivity/
  naive/querygen/translate; Lean one-rule-list theory (H5), decoder
  unification (M2), `allRules`/`Option` cleanups (M3/M6/M8), renames.
- **Wave 2 (SDKs):** independent of engine *internals* because C1
  pins the boundary IR shape-unchanged. `query!` sums, TS phase
  types + branded `ParsedQuery`, C++ phase machine + variant IR, C
  ABI `has_over` death (bridge + `query_view` move together).
- **Wave 3 (docs):** last, because docs speak the final names
  (including whatever eng render/`predicate()` renames land).

Each fix commit names its issue ids. Each issue flips to
`FIXED(<short-sha>)` in its own file and in INDEX.md.

## Issue file format

```markdown
# <id>: <title>

Severity: high | med | low
Tree: lean | engine | sdk | docs
Status: OPEN | FIXED(sha) | DISPUTED | DUPLICATE(id) | REFUSED-BY-CONTRACT(Cn)
Source: <dump file> <finding id>
Blocked-by: <ids or none>
Blocks: <ids or none>

## Bug
Exact file:line citations with the offending code quoted inline.
Self-contained: the fixer reads only this file, the contract, and the
doctrine.

## Why it is wrong
One paragraph in representation terms (illegal state representable /
proof discarded / special case in the wrong coordinate), citing the
doctrine by insight number.

## Fix
The target representation: type/signature sketches, names, where it
lives. MUST cite CONTRACT.md sections.

## Acceptance criteria
- [ ] Unrepresentability check: grep `<pattern>` over `<paths>`
      returns empty / compile-fail fixture `<name>` added and failing
      for the right reason.
- [ ] Existing tests pass UNCHANGED: <named tests>. Weakening an
      assertion is a fix rejection.
- [ ] New locks: <named new tests>.
- [ ] Commands green: <exact commands>.

## Constraints
Semantics identical. Locked names stay (`DerivedBudgetExceeded`,
`set_derived_budget`, `DEFAULT_DERIVED_TUPLES`, `DEFAULT_REACH_ROUNDS`).
Walls stay walls; OPEN refusals stay refused (mutual, nonlinear,
stacked, named-interior-of-finished-rec). Boundary IR / corpus JSON /
C ABI shapes stay per C1. No `MAX_CTES`/`MAX_INTERIORS`. No Program
vocabulary.
```

## Invariants that never move

1. **C1:** the hostile boundary (`ir.rs::Query`, corpus JSON, C ABI
   `bdb_query`, TS wire type) stays shape-unchanged; the 268
   conformance cases do not regenerate. Sums live in every layer
   *after* a parse.
2. **C5 (R-DENSE):** Lean identities stay dense `Nat`s and
   environments stay total; Fin-telescope/`Vector` rewrites are
   refused. Dual coordinates (flags, bundles, recomputed ids, dead
   screens) still die.
3. Denotations, walls, OPEN refusals, ledger, budget values, locked
   error/API names: unchanged. Representation changes; meaning does
   not.
4. Green = `scripts/lean.sh` (build + battery + census + 268-case
   conformance + three-way comparator), `scripts/check.sh`,
   `cargo test -p bumbledb --lib` + integration,
   `cargo test -p bumbledb-bench --lib`, `cargo test -p bumbledb-query`
   (compile-fail included), cpp/bridge and ts/crate `cargo test`,
   `pnpm test` in `ts/` where node_modules exist. Assertions are never
   weakened to pass.
