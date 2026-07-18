# PRD-K1 — ψ statements: `closed().where()` + the face arms

Wave K · Repo: bumbledb `ts/` · depends on: — · blocks K2, K7

## Objective

Close Hole A of ψ-selection: a closed relation's minted value gets `.where()`,
so `contained(on(Certificate, "kind"), on(Kind.where({ mastered: true }), "id"))`
is spellable. The entire pipeline beneath already works: TS `SideSpec` carries
`selection` generically (`ts/src/spec.ts`), `lowerFace` passes any owner's
selection through (`ts/src/lower.ts` ~line 76), the napi marshal marshals it
(`ts/crate/src/marshal.rs` ~line 476), the engine's spec resolver maps `"id"` →
`FieldId(0)` and shifts payload columns +1 for closed relations
(`bumbledb-theory/src/schema/spec.rs` ~line 453), and validation folds ψ
against the sealed extension into a `MemberSet` bitset
(`crates/bumbledb/src/schema/validate.rs::compile_member_set` ~line 1337). The
macro already accepts the equivalent (`Kind(id | mastered == true)`, pinned in
`crates/bumbledb/tests/schema_macro.rs` ~lines 692, 749–780). Only the TS
surface is missing.

## Work

1. **`ts/src/closed.ts`**: add `where(sel)` to the minted closed value (both
   tiers where payload columns exist; the bare tier has no payload — `.where`
   is a type-level absence there). Returns
   `SelectedClosed = { relation: AnyClosed, selection: SelectionBinding[] }` —
   deliberately the same discriminant shape as the ordinary `Selected`
   (`face.ts::faceParts` discriminates by `"relation" in source` and works
   unchanged). Resolve the input against the runtime `columns` carrier
   (`ClosedColumn` is structurally identical to `RelationField`) by REUSING
   `relation.ts::resolveSelection` — export it; do not fork it.
   Input type: payload columns only, id excluded:
   `{ [C in keyof Cols]?: Infer<Cols[C]> | OneOf<Infer<Cols[C]>> }` (match the
   ordinary `.where`'s literal/one-of vocabulary exactly).
   Add `"where"` to `reservedHandleNames` (a vocabulary with a handle named
   `where` becomes a construction-time error — pointed message).
2. **`ts/src/face.ts`**: add `AnySelectedClosed` to `FaceSource`; one new arm
   each in `FaceFields` and `ProjectedDomain` (reading the closed `columns`
   descriptors — the 0.2.0 review fix made them runtime-real and type-real; a
   `SelectedClosed` cannot structurally collide with `AnySelected` because
   `AnyClosed` lacks the relation shape — keep it that way).
3. Verify (do not modify) the pass-throughs: `renderFace`, `lowerFace`, and
   every `schema()` check. `verifyClosedReferences`/`closedTargetOf` ignore
   target selection — matching the engine's `closed_target_of`
   (`schema/render.rs` ~line 246), so violation paste-back survives. If any of
   these turns out NOT to pass through, fix it in this PRD and say so in the
   commit body.
4. **Probes** (intrinsic to the change, in `ts/test/`):
   - compile-PASS: same-label ψ pairing (`contained(on(Certificate,"kind"),
     on(Kind.where({mastered:true}),"id"))`); ψ in a `window(...)` target
     (`Enforcement::Closed` in cardinality is engine-supported).
   - compile-FAIL (real `@ts-expect-error`): `.where` on a bare-tier closed;
     `Kind.where({ nope: true })` (unknown column); wrong literal type;
     selection on `id` through `.where` (spelled only as handle literals on
     the referencing side — canonical utterance). (Domain coherence for ψ
     faces is schema-level under option 2 — K4's class laws own that probe;
     no construction-time domain check exists.)
   - runtime: a schema with a ψ statement builds; `Db.create` accepts it; a
     violating commit is rejected and the violation's canonical statement
     string equals the manifest spelling byte-for-byte (extend
     `render-golden.test.ts` with a ψ-on-closed case).

## Technical direction

- The SDK must NOT pre-fold the selection into an id set — pass-through
  lowering is what the macro does and what preserves fingerprint parity; the
  ENGINE folds at validate. (The cookbook's old "the host folds" sentence
  described the recipe author folding by hand; do not reproduce that.)
- The 256-row extension cap (`MAX_EXTENSION_ROWS`) is engine policy —
  no SDK-side duplicate check.
- Zero casts; every new type claim carries its runtime twin.

## Passing criteria

- All probes above green; the compile-fail set is real (removing any directive
  breaks compilation).
- `resolveSelection` is exported and shared (grep: one definition, two
  consumers).
- `renderStatement`/manifest byte-equality holds for ψ-on-closed statements
  (the render-golden extension is green).
- `pnpm exec tsc --noEmit` green for the touched modules + probes (whole-SDK
  green is K8's); `biome check` clean on touched files; zero casts in the
  diff. Commit deferred to the wave's committer if fanned out, else in the
  repo's voice; push.
