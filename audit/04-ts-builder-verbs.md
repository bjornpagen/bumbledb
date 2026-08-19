# 04 — The TS builder is load + admit only; the engine has the full verb set

- **Status:** **fixed this pass** — `instanceBuilderDelete`/`Reserve`/`Contains`/`Get` plus column `load`/`txInsert`; spent-builder refusal before every native call; tests: `the surface is load, delete, reserve, contains, get, admit, dispose`, `a staged fact can be retracted before admit`, `a fresh range can be minted from TypeScript before admit`, `bulk load via columns allocates no per-row JS array`, `WriteTx.insert accepts the same column transport`, `a spent builder refuses every verb before the native call`
- **Severity:** should-fix.
- **Supersedes:** VER-07.

## Principle

One collection-mutation algebra (the proposal's §Heap construction). The
engine builder and `WriteTx` share `MutationCore`'s verb set; the TS bridge
exposes a quarter of it, so a TS host routes around the representation
(admit → inspect → rebuild from scratch) instead of using the verbs the core
already proves.

## Evidence

- `ts/src/db.ts:1700-1703` — `interface InstanceBuilder` = `load` + `admit`.
- `crates/bumbledb/src/api/db/builder.rs` — `load` (:74), `delete` (:87),
  `delete_dyn` (:113), `reserve` (:127), `reserve_at` (:136), `contains`
  (:159), `contains_dyn` (:168), keyed `get`.
- Proposal §TypeScript surface: the builder offers collection load,
  collection delete, `reserve`, overlay `contains`, keyed `get`; "objects
  and columns are two transports for the same collection load."
- `db.ts:1824-1827` — `load` materializes every row as a JS array before one
  native call; the column transport (the bulk-load answer) does not exist.

## The fix

1. Mirror the verbs over the wire shapes that already exist for `WriteTx`:
   `instanceBuilderDelete` (→ `MutationReportWire`),
   `instanceBuilderReserve` (→ `FreshRangeWire`),
   `instanceBuilderContains`, `instanceBuilderGet` — all sync (staging-arena
   work, no I/O; the temporal law in 02 keeps them on the data plane).
2. Add the column transport for `load` (and `insert` on `WriteTx` for
   parity): one native call taking per-column arrays, lowering into the same
   parse-all-first batch path — the proposal's second transport, which
   avoids materializing a JS array per row.
3. Same spent-builder discipline on every new verb: `rec.spent` refuses
   before the native call, exactly as `load` does today.

## Acceptance

- TS builder surface = `load` (both transports), `delete`, `reserve`,
  `contains`, `get`, `admit`, `Symbol.dispose` — verb-for-verb with the
  engine builder minus `_dyn` twins (the TS object/column transports *are*
  the dyn path).
- A staged fact can be retracted and a fresh range minted from TS before
  `admit`; tests pin both.
- Bulk load via columns allocates no per-row JS array (probe or review
  gate).
