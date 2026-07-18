# PRD-K3 — The minimal kernel: fields are pure structure, `.as` dies

Wave K · Repo: bumbledb `ts/` · depends on: — · blocks K4, K7 · hard break ·
OWNER RULING 2026-07-18 ("option 2, zero debate"): the laws type the columns —
this PRD strips the kernel down so K4 can compute domains from statements.

## Objective

Delete declared domains from the field kernel entirely. A field descriptor
carries structure only — `{ kind, width?, element?, fresh? }` — and NOTHING
about domains: `.as` is removed from every constructor (not deprecated, not
banned — absent from the surface and the types), and `ref`/`cites`/`dom` are
never built. Values stay bare exactly as in 0.2.0. Domain semantics move
wholesale to `schema()` (PRD-K4).

## Work

1. **`ts/src/fields.ts`**: remove `.as` from `u64`/`i64`/`bytes(n)`/
   `interval(e[,w])` — the method, the `Domain` type parameter plumbing on
   field descriptor types, and the runtime slot. The descriptor types become
   e.g. `U64Field<Fresh extends boolean = false>`-shaped (naming per the
   module's voice); `Infer<F>` is untouched (bare value types). `.fresh`
   unchanged (u64-only, as ratified in the structural wave).
2. **`ts/src/relation.ts`**: relation declarations accept only pure-structure
   descriptors; no domain rewriting, no coordinate minting here (K4 owns
   naming, at the schema level). The runtime field table keeps declaration
   order exposed — K4's class laws depend on relation-declaration ×
   field-declaration order being readable at both the type and value level;
   make sure both carry it (the type level already does via the fields
   object; the value level via the existing frozen field list).
3. **`ts/src/closed.ts`**: the closed id descriptor loses its
   `` `${Name}Id` `` declared domain the same way — a closed relation's id is
   a GENERATOR whose class K4 names `"Kind.id"`; the descriptor itself
   carries only structure + the closed linkage it already has (the typed
   `columns` carrier and roster stay exactly as the 0.2.0 review fixes left
   them). Payload column descriptors: structure only.
4. **Statement constructors keep their STRUCTURAL checks only**
   (`ts/src/statements.ts`, `face.ts`): pairing faces still demands
   positionwise kind/width/element compatibility at construction (those live
   on the descriptors and stay) — but `SameDomains`/`FaceDomainMismatch` and
   every domain-label comparison at construction are DELETED. The domain wall
   moves to `schema()` (K4's one-generator-per-class check) and to query
   joins (class names off the schema type). This is the ratified design, not
   a weakening: at construction there is no domain to compare; the laws are
   self-defining and the schema is where they aggregate.
5. **Probes** (rewrite the kernel probe suites):
   - `Fact`/`InsertFact`/`Infer` unchanged for every kind (Equal-probes);
     fresh omittable on insert; values bare.
   - compile-FAIL (real): `.as` anywhere (the property does not exist — pin
     one per constructor); structurally-mismatched face pairing at
     construction (u64 face against str face; width mismatch on bytes;
     element mismatch on interval).
   - The old domain-mismatch construction probes are DELETED here and
     re-homed by K4 at the schema level (note the movement in the commit
     body so the review can follow the wall).
   - Type-lie sweep: descriptors carry no `domain` property at runtime
     (`Object.hasOwn(f, "domain") === false` for every constructor output).

## Technical direction

- The wire contract is K4's concern (it emits computed class names as the
  spec `newtype`); this PRD must leave `lower.ts` compiling against the
  slimmer descriptors with the newtype slot temporarily fed by `undefined` —
  mid-wave red beyond that is expected and unshimmed.
- Zero casts; no underscore params; hovers stay evaluated-literal clean (the
  descriptor types get SIMPLER — verify one hover probe).

## Passing criteria

- `.as` is absent from `ts/src` (grep zero) and from every descriptor type;
  the runtime `domain` slot is gone (the hasOwn probes).
- Structural face checks still fire at construction (probes); no
  domain-label machinery remains in `statements.ts`/`face.ts` construction
  paths (grep `SameDomains`/`FaceDomainMismatch` — zero, with K4 owning the
  replacement wall).
- Kernel + probes `tsc --noEmit` green in isolation; zero casts in the diff.
  Push per the wave's commit discipline.
