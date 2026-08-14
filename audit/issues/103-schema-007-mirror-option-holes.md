# schema-007: dual coordinate — sealed `mirror` vs render's `Vec<Option<StatementId>>`

- **Severity:** medium
- **Tree:** schema
- **Status:** OPEN
- **Source:** audit/storage-schema.md F15
- **Depends on:** none
- **Conflicts with:** none

## The bug

Validate seals `ContainmentStatement.mirror: Option<StatementId>` (`schema.rs:447-463`). `render_rejection` (`schema/render.rs:79`) cannot read it (pure over a possibly-rejected `SchemaDescriptor`), so it rebuilds `mirror_links` (`validate.rs:319-324`) — an Option-padded array, one hole per statement including every FD and one-way containment that cannot have a partner. Two implementations of one pairing; the sealed field is unused on the public render path.

## Why it's wrong

Insight 6 — the sealed witness already parsed the partner; diagnostics re-parse into a hole-padded table. Insight 4 — `Vec<Option<StatementId>>` indexed by all statement kinds, most holes mandatory.

## The fix

`audit/CONTRACT.md` C1 does not freeze this tree.

- `mirror_links` returns containments-only (`BTreeMap<StatementId, StatementId>`, or a slice parallel to the containment subsequence). Option holes for keys/capacities die.
- On a sealed `Schema`, a renderer that has the witness reads `ContainmentStatement.mirror` and does not re-search.
- Rejected-declaration diagnostics still run `mirror_of` over the descriptor (never sealed) — that path is essential.

## Acceptance criteria

- [ ] Gone: `rg -n 'Vec<Option<StatementId>>' crates/bumbledb/src/schema/validate.rs` (or the function no longer returns a hole per FD).
- [ ] Sealed render path (if any takes `&Schema`) reads `ContainmentStatement.mirror`.
- [ ] Unchanged tests: `schema/render/tests.rs` mirror/`==` spelling tests green, assertions untouched.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Pairing identity stays normalized sides (selections sorted, literal sets canonical). `DuplicateStatement` still makes links unique and symmetric.
