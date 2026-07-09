# PRD 03 — The materialized mirror

**Depends on:** 01 (lands on the shared `Value`; independent of 02 but ordered
here to keep the schema-module churn contiguous).
**Modules:** `crates/bumbledb/src/schema.rs` (the sealed `Statement`),
`crates/bumbledb/src/schema/{validate.rs,render.rs}`, `error/display.rs`
(violation rendering), EXPLAIN's statement rendering if it renders pairs.
**Authority:** `30-dependencies.md` (`==` is two containments — the pairing is
a fact of the declaration), `00-product.md` (parse, don't validate: computed
once at the boundary, read thereafter).

## Context (decided)

`==` lowers to two mirrored containment statements, and the *pairing* is then
thrown away. `render.rs:124-143` re-discovers it at every render with a
`mirrors` closure structurally comparing **adjacent** statements — validation
re-deriving a fact materialization knew. Two defects in one: the reconstruction
branch, and a latent gap — the adjacency assumption means two mirrored
statements that are *not* adjacent (legal for hand-built descriptors) render as
two `<=` lines instead of one `==`.

## Technical direction

1. **Sealing computes the pairing once:** the sealed `Statement` gains
   `mirror: Option<StatementId>`. Computed during validation/sealing by exact
   swapped-sides comparison over **all** containment statements (n² over ≤
   `u16::MAX` statements, in practice tens — no cleverness), not just adjacent
   ones. Both partners point at each other; a statement can have at most one
   mirror (duplicate statements are already rejected by the roster, which is
   what makes the pairing unique — cite that line).
2. **Render reads the fact:** the `mirrors` closure dies. A statement with
   `mirror: Some(m)` renders as `==` once (canonically from the lower id, the
   partner suppressed — same output rule as today); `None` renders `<=`. The
   adjacency gap closes for free.
3. **Every pair-aware consumer switches to the field:** violation `Display`
   (the "direction for `==` statements" wording), EXPLAIN's statement
   rendering, and any test helper that re-detects pairs.
4. **Not fingerprinted:** `mirror` is derived from hashed inputs, exactly like
   `Resolved` — extend the existing "pinned without being hashed separately"
   comment to name it.

## Passing criteria

- `[shape]` No mirror-detection logic exists outside sealing (grep for the
  swapped-sides comparison — one site); render/display consume the field.
- `[test]` Render goldens unchanged for the adjacent `==` pair; a **new**
  golden: a hand-built descriptor with the mirrored pair separated by an
  unrelated statement renders as `==` once (the closed gap, pinned).
- `[test]` Sealing: mirror links are symmetric; a one-way containment has
  `None`; the `==` pair from the macro links correctly.
- `[test]` Fingerprint unaffected by the field (identity golden).
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

None expected — `30-dependencies.md` already states `==` is two statements;
if any chapter line implies pairing is positional/adjacent, amend it to "the
sealed statements carry their mirror links."
