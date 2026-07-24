## Mirror pairing uses raw side equality while sealed sides and statement identity are canonicalized

category: bug | severity: medium | verdict: CONFIRMED | finder: lean:schema-values
outcome: fixed 7bea2d96

### Summary

The sealed `==` partner link on containments is computed by `mirror_of`, which compares RAW descriptor sides with derived `PartialEq`, but the sides it annotates seal in CANONICAL form (`canonical_side` sorts each disjunctive literal set), and statement identity for duplicate rejection is a third, fully normalized form (`normalize`). A mutual-containment pair whose `Many` literal sets are spelled in different orders is a semantic `==` pair whose sealed sides are exact swapped mirrors — yet both statements seal `mirror: None`. This violates the documented invariant on `ContainmentStatement::mirror` and makes two schemas with IDENTICAL fingerprints render and manifest differently: one prints `==` once, the other prints two `<=` statements.

### Evidence

All verified against the working tree:

- `crates/bumbledb/src/schema/validate.rs:265-283` — `mirror_of` searches `descriptors` for `mirror_source == target && mirror_target == source` using raw `Side` equality (`Side` derives `PartialEq` over order-sensitive boxed slices, `crates/bumbledb-theory/src/schema.rs:269-280`).
- `crates/bumbledb/src/schema/validate.rs:152-155, 167` — sealing stores `source: canonical_side(source)`, `target: canonical_side(target)` (literal sets sorted by `literal_cmp`, validate.rs:443-467) but computes `mirror: mirror_of(&descriptors, idx)` from the pre-canonical descriptors in the same push.
- `crates/bumbledb/src/schema/validate.rs:473-502` — `normalize` (the `DuplicateStatement` identity) additionally sorts selections by `FieldId`: three distinct identity notions now coexist (raw for mirrors, canonical-sealed for fingerprints, normalized for duplicates).
- `crates/bumbledb/src/schema.rs:406-417` — the sealed field's contract: "The `==` partner: the containment whose sides are exactly this statement's sides swapped". The sealed sides of the respelled pair below ARE exactly swapped, yet the link is absent. (The doc's uniqueness argument via `DuplicateStatement` is unaffected — raw-equal implies normalized-equal — but the search identity diverges from the sealed representation it decorates.)
- Reachability: `crates/bumbledb-theory/src/schema/spec.rs:657-702` — `side()` preserves user literal order (`Many` arm at 688-693, no sort); only the `bidirectional: true` spelling (spec.rs:893-901) clones one Side pair swapped. Two separately-declared one-way containments reach validate un-normalized. Hand-built `SchemaDescriptor`s likewise (`materialized_statements`, bumbledb-theory/src/schema.rs:411, appends declared statements verbatim).
- Fingerprint: `crates/bumbledb/src/schema/fingerprint.rs:106-109, 176-209` — `put_side` hashes the SEALED canonical sides ("the sealed side's canonical (sorted, deduplicated) set order makes the stream a function of the set, not its spelling", fingerprint.rs:193-196), so the respelled pair and the canonical `==` pair produce the same fingerprint. The mirror link itself is deliberately unhashed (fingerprint.rs:10-16) on the premise that it is "a deterministic function of the hashed inputs" — false here: two fingerprint-equal schemas seal different mirror links.
- Consumers: `crates/bumbledb/src/schema/render.rs:151-155` and `render.rs:405-428` (sealed render: `==` once when the link is present, `<=` otherwise); `render.rs:195-202` (`render_declared` re-runs raw `mirror_of`); `crates/bumbledb/src/schema/manifest.rs:100` (the Manifest's canonical spelling goes through `render_declared`). `mirror` has no consumer outside the schema module (grepped `src/exec`, `src/storage`, etc.), so enforcement semantics are unaffected.
- Test coverage: `crates/bumbledb/src/schema/render/tests.rs:144` covers only a non-adjacent pair with identical raw spelling; no test respells a literal set across a mutual pair.

### Failure scenario

Declare (via `SchemaSpec` with two one-way `Containment` statements, or a hand-built descriptor):

- C1: `A(x) <= B(y | f in {1, 2})`
- C2: `B(y | f in {2, 1}) <= A(x)`

Both validate (they are not normalized duplicates of each other — sides swapped). `canonical_side` seals both literal sets as `{1, 2}`, so the sealed statements are exact swapped mirrors, but `mirror_of`'s raw compare fails on `{1,2}` vs `{2,1}`; both seal `mirror: None`. `render` and the `Manifest` emit two `<=` lines. The same theory spelled with `bidirectional: true` (or with matching literal order) seals `mirror: Some` and renders one `==`. Both stores carry the same fingerprint, so "same fingerprint ⇒ same manifest/rendering" breaks — a divergence visible to any foreign surface reading the manifest, and to diagnostics.

Correction to the original finding's scope: the *binding-order* variant (σ bindings listed in different field order) does NOT exhibit this — `canonical_side` preserves binding order and `put_side` hashes bindings in statement order, so that pair neither seals swapped-mirror sides nor shares a fingerprint. The confirmed defect is the `Many`-literal-order respelling only.

### Suggested fix

Make the one identity notion (`normalize`) the only identity notion, per the representation-first doctrine (one canonical representation instead of three comparison dialects): hoist the `normalized` vec (already built inside the same sealing loop, validate.rs:102, 215-222) ahead of the loop and have `mirror_of` search swapped NORMALIZED sides; `render_declared` (render.rs:201) normalizes likewise before its diagnostic search. Alternatively — stronger, representation-level — canonicalize descriptors once at the top of `validate` and let sealing, duplicate detection, and mirror pairing all read the same canonical list, so a non-canonical spelling ceases to exist past the boundary (parse, don't validate).
