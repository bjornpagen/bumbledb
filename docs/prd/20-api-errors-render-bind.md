# PRD 20 — API: errors, rendering, binding

**Depends on:** 07, 08, 09, 11, 17.
**Modules:** `crates/bumbledb/src/error.rs`, new `crates/bumbledb/src/schema/render.rs`, `crates/bumbledb/src/api/prepared/` (bind surface), `crates/bumbledb/src/api.rs`.
**Authority:** `docs/architecture/70-api.md` (§ errors, § facts and results), `30-dependencies.md` (statements are anonymous — errors cite the rendered algebra).

## Goal

The public error taxonomy matches `70-api.md`; violations render their statement
back in the algebra notation; execution bind accepts scalar values and set slices.

## Technical direction

1. **Error taxonomy sweep** (`error.rs`): confirm PRDs 07–09 landed
   `FunctionalityViolation { statement, fact }` and
   `ContainmentViolation { statement, direction, fact }`; schema errors carry
   `StatementId` + positions (PRD 03). Hot-path payloads stay ids and byte
   boxes — no formatted strings constructed at raise time (allocation contract;
   the existing rule).
2. **Statement rendering** (`schema/render.rs`): `fn render(schema, StatementId) -> String`
   producing exactly the macro notation:
   `Account(holder) <= Holder(id)`,
   `Grading(id | kind == Deterministic) == DeterministicGrading(grading)` — a
   bidirectional *pair* renders as `==` when its mirror statement exists adjacent
   (detect: the other direction with swapped sides; otherwise render `<=`), FDs as
   `SavingsTerms(account) -> SavingsTerms`. Selection literals render through one
   value formatter (enum ordinals resolve to variant names via the schema;
   intervals as `start..end`). Used by the `Display` impls of the two violation
   errors (Display allocates — that is fine, Display is never the hot path) and by
   schema-error diagnostics.
3. **Bind surface:** the public params type accepts, per `ParamId`, either a
   scalar `Value` or a set slice — concrete shape:
   ```rust
   pub enum ParamArg<'a> { Scalar(Value), Set(&'a [Value]) }
   ```
   with bind-time checks: arity/density, scalar-vs-set matching the query's usage
   (validation recorded which — thread it through the prepared query), element
   type checks, dedup into pooled storage (PRD 17's internal representation).
   Errors are the existing bind-error family extended, precise per position.
4. **`api.rs` exports:** `Interval`, `ParamArg`, `StatementId`, the renamed
   errors; delete dead re-exports flagged by the compiler after phases A–C.

## Out of scope

Result ordering/limit conveniences (OPEN). Multi-key typed `get` sugar (OPEN).

## Passing criteria

- `[shape]` No error payload constructs a `String` at raise time on write or
  query paths; rendering lives only in `Display`/diagnostic contexts.
- `[test]` Render goldens: an FD, a one-way containment with selection, a
  bidirectional pair (renders `==` once), an interval selection literal
  (`0..86400`), enum variant names resolved.
- `[test]` A `ContainmentViolation` raised by a commit test `Display`s containing
  the rendered statement and the direction.
- `[test]` Bind matrix: scalar where set expected / set where scalar expected /
  wrong element type / non-dense ids — each the precise error; a valid mixed bind
  (two scalars, one set) executes.
