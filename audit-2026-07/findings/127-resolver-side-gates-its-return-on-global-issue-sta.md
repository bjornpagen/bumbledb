## Resolver::side gates its return on global issue state — an inert branch the crate's own placeholder law erases

category: inappropriate-branching | severity: low | verdict: CONFIRMED | finder: theory

### Summary

`Resolver::side` in `crates/bumbledb-theory/src/schema/spec.rs` returns `Some(Side)` only when `self.issues.is_empty()` — the resolver's *entire* accumulated issue list, not this side's own resolution outcome. The predicate is behaviorally inert: every issue-generating check runs before or independently of it, and `descriptor()` discards the whole `statements` vector whenever any issue exists. Meanwhile the `Fd` arm follows the opposite convention, pushing a statement with a silently partial projection. Two inconsistent failure conventions coexist, both masked by the final all-or-nothing discard — and the crate already documents the representation that removes the branch, one function up: `literal()` returns a `Value::U64(0)` placeholder with the law "placeholders never escape (a nonempty issue list fails the whole construction)".

### Evidence (all verified in crates/bumbledb-theory/src/schema/spec.rs)

- **The guard, spec.rs:697-702:**
  ```rust
  (self.issues.is_empty()).then(|| Side {
      relation: RelationId(u32::try_from(rel_idx).expect("relation count fits u32")),
      projection: projection.into_boxed_slice(),
      selection: selection.into_boxed_slice(),
  })
  ```
  `self.issues` is global resolver state. Issues pushed by *earlier, unrelated* work poison it: `DuplicateHandleNewtype` (spec.rs:788-796) and extension-row lowering (`RowArityExcess` spec.rs:828, row-literal handle issues via `literal()` spec.rs:849) all run before the statement loop, so one bad extension row makes every statement's `side()` return `None`. Likewise a broken statement 0 makes a clean statement 5's sides return `None`.

- **None ⟺ issues nonempty:** `relation()` (spec.rs:499-508) and `field()` (spec.rs:514-533) unconditionally push an issue when they return `None`, so any this-side failure already implies a nonempty issue list. The guard adds nothing per-side; it only widens the failure signal to global state.

- **Inertness:** the only two callers destructure `let (Some(source), Some(target)) = (source_side, target_side) else { continue; }` (spec.rs:890, :915). In both arms `coherent()` runs first (:887, :911), `window()` runs before the sides in the Cardinality arm (:912), and both `side()` calls execute to completion before the destructure — so no issue collection is ever skipped by the `continue`. Its only effect is dropping entries from `statements`, and spec.rs:928-935 returns `Err(SchemaSpecError(...))` — discarding `statements` entirely — whenever issues is nonempty. Every statement the guard drops is on the `Err` path anyway.

- **The opposite convention next door, spec.rs:869-880:** the `Fd` arm pushes `StatementDescriptor::Functionality` with whatever subset of the projection resolved (fields that fail `resolver.field()` are silently skipped at :871-873) — a best-effort partial push, where the containment/cardinality arms do an all-or-nothing skip.

- **The documented law, spec.rs:611-614:** `literal()`'s doc comment: "On an issue the placeholder `Value::U64(0)` stands in; placeholders never escape (a nonempty issue list fails the whole construction)." This is the total-function-with-placeholder representation, already in the same impl block.

- **Doctrine:** docs/design/representation-first.md names this exact pattern — sentinel/placeholder objects that "represent 'nothing' or 'the boundary' as a real object and the checks disappear wholesale," and the guiding question "what representation makes this branch unnecessary?" The guard at :697 is a branch testing information (global validity) that the final gate at :928 already owns.

### Failure scenario

No wrong output today — that is the finding: the branch is load-bearing-looking dead logic wearing the costume of per-side error handling. It becomes a live bug the moment the construction stops being all-or-nothing — e.g. a best-effort/IDE/diagnostics mode that returns the lowered `statements` alongside the issues (clean statements after the first broken one would silently vanish, and Fd statements would surface with truncated projections), or any refactor that moves an issue-producing check after the guard. The Fd/side asymmetry also means two contributors reading adjacent arms learn two contradictory conventions for "a name failed to resolve."

### Suggested fix

Make `side()` total, per the `literal()` law: always return a `Side` built from whatever resolved (placeholder-bearing where needed), delete the `self.issues.is_empty()` guard, and delete the `let (Some, Some) = … else continue` scaffolding at spec.rs:890 and :915. The single final gate at spec.rs:928-935 already enforces "placeholders never escape." This collapses the Fd and containment/cardinality arms onto one convention and removes the branch a better representation makes unnecessary. `side()` is private with exactly these two callers, so the change is local.
