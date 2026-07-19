# PRD-H1 — The precise roster type: `ClosedIdField<Handles>` and the `Infer` arm

Wave H · Repo: bumbledb `ts/` · depends on: — · blocks everything · the type
foundation: every downstream surface reads the handle union through `Infer`

## Objective

Stop widening the roster. `ClosedIdField` (declared `ts/src/fields.ts`
~lines 133–136) carries `closed: ClosedRoster` where `ClosedRoster`
(~line 55) is `{ name: string; handles: readonly string[] }` — the literal
union `"DirectPass" | "Failed"` exists at runtime but is erased at the type
tier. Make it precise, and make `Infer` yield it. After this PRD, a
closed-referencing column's VALUE TYPE is the handle union everywhere
`Infer` is read: `Fact`, `InsertFact`, match records, select rows, params,
selections. (The runtime VALUES are still bigints until H2 lands the
bijection — the tree is expected red/lying in between; no shims.)

## Work

1. **`ts/src/fields.ts`**:
   - `ClosedRoster<H extends string = string>`: `{ readonly name: string;
     readonly handles: readonly H[] }`.
   - `ClosedIdField<H extends string = string>`: `{ readonly kind: "u64";
     readonly closed: ClosedRoster<H> }` (keep the exact property layout —
     `kind: "u64"` is load-bearing for the class map and JoinOk, which
     compare kind/class/width/element and MUST NOT change).
   - `Infer<F>` (~lines 147–159): add one arm BEFORE the `kind: "u64"` arm:
     `F extends { closed: { handles: readonly (infer H extends string)[] } }
     ? H : …`. Every other arm untouched.
2. **`ts/src/closed.ts`**: the minted `id` descriptor becomes
   `ClosedIdField<Handles>` — the roster array is already built in
   declaration order at mint time; only the TYPE gains precision. The
   descriptor stays frozen with own properties (the type-lie law: the
   precise type's runtime twin is the same frozen array that was always
   there — pin it).
3. **The trusted-seam guard**: the SDK's checkable-facts predicates (the
   `refsComplete` pattern, `ts/src/relation.ts` ~line 29, and the query
   layer's `isTypedScope` family) must admit the precise descriptor —
   verify each seam that pattern-matches on field descriptors still narrows
   correctly with the generic parameter present; extend the predicate if it
   matched on the old wide shape.
4. **Probes** (intrinsic, `ts/test/`):
   - `Infer<typeof Kind.id>` equals the exact union (Equal-probe);
     `Fact<typeof Certificate>["kind"]` equals the union when the field is
     declared `kind: Kind.id`; `InsertFact` accepts `"DirectPass"` and
     compile-FAILS (real directive) on `"DirectPas"` (typo) and on `0n`
     (bigint no longer assignable).
   - Two DIFFERENT vocabularies sharing a handle name: the unions are
     structurally assignable where they overlap — pin the honest fact with
     a probe and a comment (this is the structural doctrine, and it is
     strictly better than today's any-bigint-assigns-anywhere).
   - The class map is UNCHANGED: the law-typing fixture probes
     (`ts/src/law.ts`'s runtime/type diff check and the K4 goldens) stay
     green with zero edits to `law.ts` — a `law.ts` diff in this PRD is a
     scope violation.
   - Hover probe: the descriptor type renders as an evaluated literal
     (`ClosedIdField<"Checking" | "Savings">`), not conditional soup.

## Technical direction

- Do NOT touch marshal/lowering/runtime values here — H2/H3 own the
  bijection; this PRD makes the types tell the truth the runtime will be
  made to honor. Mid-wave, `Fact` claims strings while `factOf` still
  produces bigints: that is the sanctioned red state, not something to
  paper over.
- Zero casts; the `Infer` arm must not degrade any existing kind's
  inference (re-run the full kernel Equal-probe suite).

## Passing criteria

- All probes above green; the compile-FAIL directives real.
- `grep -n "readonly string\[\]" ts/src/fields.ts` shows the roster no
  longer widens (the generic default `string` remains only as the unbound
  fallback).
- `law.ts` untouched (`git diff --stat` proves it).
- `tsc --noEmit` green for `fields.ts`/`closed.ts` + the kernel probes in
  isolation (whole-tree red is expected). Zero casts in the diff. Push per
  the wave's commit discipline.
