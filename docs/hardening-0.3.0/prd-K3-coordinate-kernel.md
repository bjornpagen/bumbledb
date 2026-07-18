# PRD-K3 — The coordinate kernel: derived domains, `ref`, `cites`, the dot-ban

Wave K · Repo: bumbledb `ts/` · depends on: — · blocks K4, K7 · hard break

## Objective

Domains stop being hand-written claims where a declaration exists. Three
changes to the field/relation kernel (statement synthesis is K4's, NOT here):

1. `relation()` mints the coordinate label `` `${Name}.${field}` `` for
   **fresh fields only** (B-min — ratified; B-max is out of scope).
2. `ref(R, "f")` — a field descriptor copying the target field's
   kind/width/element AND its domain, carrying a runtime reference marker.
3. `cites(R, "f")` — the identical descriptor with a distinct marker that K4
   will NOT derive a statement from (the Calendar-recipe case: a domain link
   whose only lawful statement is a selected `mirrors`).
4. `.as` is **dot-banned**, type-level and runtime, making coordinates
   unforgeable through public constructors.

Values stay bare (`bigint` etc.) — nothing here touches value types. This was
prototyped end-to-end against the published 0.2.0 (working code existed in
`/tmp/bdb-ref-lab/`; treat this PRD's text as the authority, the prototype as
evidence it composes).

## Work

1. **`ts/src/fields.ts` / `ts/src/relation.ts`**:
   - `relation(name, fields)`: for each field with `fresh: true` and no
     hand-written domain, the descriptor's domain becomes the template-literal
     type `` `${Name}.${K}` `` and the runtime string `${name}.${key}` (clone
     the descriptor — descriptors are frozen; never mutate a shared
     constructor value). A fresh field WITH a hand `.as` label is a
     construction-time error (one source of truth; pointed message: "fresh
     fields derive their domain — delete the .as").
   - `ref(target, field)`: validates at construction that `target` is a
     relation value and `field` a labelable field of it (u64/i64/bytes/interval
     kinds — same set `.as` accepts); returns a NEW frozen descriptor
     `{ kind, width?, element?, domain: target's domain, refTo: { relation:
     target.name, field } }` — `refTo` an own, frozen property (runtime twin
     of the type claim). A `ref` is never fresh (`.fresh` absent from its
     type; runtime lacks the slot).
   - `cites(target, field)`: identical, marker key `citeTo` instead of
     `refTo`. The two markers are mutually exclusive by construction.
   - Refs to closed relations: `ref(Kind, "id")` yields the `ClosedIdField`
     descriptor shape (domain `` `${Name}Id` `` as today) + `refTo` — K4 uses
     it; the kernel just carries it.
2. **The dot-ban** (`.as`): type level —
   `` Domain extends `${string}.${string}` ? never : Domain `` on every `.as`
   signature; runtime — throw on a dot with the pointed message ("coordinates
   are minted by relation()/ref() — hand labels cannot contain '.'").
3. **Probes** (intrinsic, `ts/test/` — rewrite/extend the kernel probe suites):
   - `Fact<typeof Service>["id"]` is still bare `bigint`; fresh still
     omittable on insert; a `ref` field is mandatory on insert.
   - `Infer` unchanged for every kind.
   - Domain flow: `Service.id`'s descriptor type carries `"Service.id"`;
     `ref(Service, "id")`'s carries the same; a face pairing them compiles; a
     pairing against `"Holder.id"` fails with
     `FaceDomainMismatch<["Holder.id"], ["Service.id"]>`-shaped self-locating
     error (pin the error type, not the message text).
   - compile-FAIL (real): `u64.as("Service.id")` (forged coordinate);
     `.fresh` on a `ref`; `.as` on a fresh field; `ref(Service, "name")` where
     `name: str` (unlabelable kind) — plus the runtime throws for the
     dot-ban and fresh+as, asserted.
   - Structural doctrine: two independent identical `relation("Service", …)`
     declarations produce mutually-assignable types (an `Equal<>`-style probe)
     and — via the existing law — identical fingerprints.
   - Type-lie sweep: `Object.hasOwn(refField, "refTo")` true and frozen;
     same for `citeTo`.

## Technical direction

- Fingerprint neutrality is a LAW here: domains lower to the wire `newtype`,
  which the engine drops ("two specs differing only in newtype names lower to
  identical descriptors" — `spec.rs`). Re-pin it with coordinates: a probe
  asserting a schema's fingerprint is identical with and without a (legal)
  hand label swap. Statement DERIVATION (which does move fingerprints) is
  K4's, and none of it happens in this PRD — `schema()` is untouched here.
- Hovers matter: the coordinate must render as an evaluated literal
  (`U64Field<"Service.id">`), not conditional-type soup — if an implementation
  choice degrades hovers, choose the other implementation.
- Zero casts; `@ts-expect-error` only in tests, each real.
- Ratify the Rust asymmetry in docs: one short paragraph in
  `docs/architecture/70-api.md` (the SDK chapter): TS derives coordinates,
  Rust declares `as NewType`, labels never reach the store, fingerprints agree
  when names/types/statements agree. (The macro is NOT changed.)

## Passing criteria

- Every probe above green; every compile-fail directive real.
- Kernel modules (`fields.ts`, `relation.ts`, `closed.ts` if touched) contain
  zero casts (grep) and no underscore-prefixed functions/params.
- The fingerprint-neutrality probe passes.
- The 70-api.md paragraph exists and states the asymmetry.
- `tsc --noEmit` green for kernel + probes (tree-wide red is expected until
  K8). Push per the wave's commit discipline.
