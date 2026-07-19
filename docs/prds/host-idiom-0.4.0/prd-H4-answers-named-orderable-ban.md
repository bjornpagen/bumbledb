# PRD-H4 — Answer rows arrive named + the orderable ban

Wave H · Repo: bumbledb `ts/` · depends on: H3 (same query files) · the one
genuinely-M runtime piece of the packet

## Objective

Two remaining query-tier truths: (1) SELECT results decode closed columns to
handle names — `db.execute(q, {})` returns
`{ a: bigint; k: "DirectPass" | … }[]` with the runtime VALUE being the
string (H1 made the type claim; this PRD is its runtime twin); (2) closed
fields exit the orderable/foldable set — `lt(kindVar, …)`, `sum` over a
closed column, and every order-comparison position REFUSE closed-bound
terms, type-tier and lowering-tier both.

## Context (verified in the texture study)

- `query/run.ts::decodeAnswers` sees only `SelectColumn { name, entry }` —
  NO field descriptor reaches the decode loop today. The classed slot
  already exists SDK-side in `RuleData.varFields` (the lowering's own
  bookkeeping); it must be plumbed into the select columns. This is
  SDK-runtime data only — the wire `ProgramIr` is untouched.
- `query/atom.ts::OrderVarOk` (~line 520) admits any `kind: "u64"` var —
  so today `lt(kindVar, 2n)` and aggregate folds over closed ids typecheck.
  `docs/architecture/10-data-model.md` (~line 110) already rules the
  declaration-id order "an accident, not semantics".

## Work

1. **Plumb the descriptor**: extend the lowering's select-column
   construction (`query/lower.ts`, where `SelectColumn`s are built from the
   rule env) to carry the bound field's descriptor (or the minimal
   `closed?: ClosedRoster` slice) for each selected entry. Source of truth:
   the same `varFields` the domain machinery reads. Head-projected idb
   columns (recursion outputs) carry whatever the head position's field
   carries — trace `RuleValue`/`HeadFieldsOf` to confirm closed descriptors
   survive the rec head; if they do not, the rec-head select of a closed
   column stays bigint and MUST be documented + probed as such (honest
   limitation beats silent wrongness — but attempt the plumb first; the
   K2/K4 machinery carries full descriptors through heads).
2. **Decode** (`query/run.ts::decodeAnswers`): closed columns translate
   `id → roster.handles[Number(id)]`, same pointed out-of-roster throw as
   H2 (share the helper — export it from `marshal.ts`; one bijection, one
   implementation, two call sites).
3. **The orderable ban**:
   - Type tier: `OrderVarOk` (and the aggregate-position equivalents —
     find every order/fold admission judging `kind: "u64"`) excludes
     descriptors carrying `closed`. Compile-FAIL probes: `lt` on a
     closed-bound var; `sum`/`max`/`argMax` folding a closed column;
     an order-comparison param anchored at a closed field.
   - Lowering tier (runtime twin): the lowering refuses the same shapes
     with a pointed error citing the data-model ruling ("declaration order
     is an accident — vocabularies do not order").
4. **Probes** (intrinsic):
   - runtime: a query selecting a closed var returns string values
     (strict-equality asserted against the roster), and the SAME query's
     rows match its 0.3.0 twin modulo the translation;
   - the lowering goldens from H3 stay green (the wire program unchanged —
     re-run, do not re-pin);
   - `count` over closed-atom-filtered rules still works (counting is not
     ordering — pin the distinction);
   - the rec-head closed column: either the string decode probe (plumb
     succeeded) or the documented-bigint probe (limitation recorded).

## Technical direction

- The wire `ProgramIr` and `dbPrepare`/`dbExecute` contracts are untouched
  (any `ts/src/native.ts` or `ts/crate` diff is a scope violation) — the
  descriptor rides the SDK-side `Prepared` value, which already carries the
  Row/Params phantom and the runtime plan handle.
- One bijection implementation total (the exported marshal helper) — grep
  proves no second `handles[Number(` site exists outside it.
- Zero casts; the `Prepared` phantom slot stays never-assigned (the
  type-lie law does not apply to the sanctioned phantom — it is already
  ruled).

## Passing criteria

- All probes green; the ban's compile-FAIL directives real; the lowering
  refusal pinned by message fragment.
- H3's lowering goldens pass UNMODIFIED.
- `grep -rn "handles\[Number(" ts/src` → exactly one definition site.
- `tsc --noEmit` green for the query modules + probes in isolation. Push
  per the wave's commit discipline.
