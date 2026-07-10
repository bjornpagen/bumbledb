# PRD 14 — The named theory — LANDED via `docs/prd/22`, one residual

**Status: implemented before this set began execution** (`241ccfc`). This file
is the reconciliation record plus the single residual item.

What landed, verified against this PRD's intended criteria:

- `pub Ledger;` grammar header → unit struct; `Db::create/open(path, Ledger)`
  validate at open with typed `SchemaError`; the magic `fn schema()`, the
  `OnceLock`, and the panic path are gone.
- Typestate: `Fact::Schema` welds structs to their schema; cross-schema
  confusion is a compile error.
- `SchemaDescriptor` implements the definition trait as itself, keeping
  runtime-built schemas (bench/oracle) first-class — a shape this PRD had not
  specified and the implementation got right.

## Residual: the trait's name

The implementation named the trait **`SchemaDef`**. This set's vocabulary
discipline (README) prefers the dependency-theory name: a schema *is* a
presentation of a **theory** — relations plus statements — and a store models
it. The residual work item, folded into PRD 01's rename sweep as one more
line: `SchemaDef` → `Theory` (with `descriptor(self)` unchanged), and the
sentence "`a schema names a theory; a store models it`" lands in
`10-data-model.md` alongside it. Owner may veto the rename at PRD 01 execution
time; either verdict is recorded there.

**Refusals from the original design, still standing** (recorded here since the
implementing PRD predates them): compile-time full roster validation (would
duplicate `schema/validate.rs` into the macro crate — single-source wins);
schema composition/fragments (declaration-order ids need one block that sees
everything); derive-inversion (statements are cross-relation; the single block
is the mental model).
