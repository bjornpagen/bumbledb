# The structural 1.0.0 packet — the SDK made structural, the engine closed, the release earned

One design move, executed to the end: the TypeScript SDK stops emulating Rust's
nominal newtypes with brands and becomes **completely structurally typed** —
bare values, domains carried as string labels in the schema's *type*, the
relational builders checking domains structurally, the engine judging the rest.
On the way the last open engine semantic is closed, the SDK publishes its own
translated cookbook, and everything converges on a hardened `1.0.0` that the
owner tags and publishes.

This directory is an execution-style PRD packet (ordered, strict passing
criteria). Per house convention it is DELETED once shipped. It **supersedes**
`docs/road-to-1.0.0/` (the nominal-hardening packet): that packet's 02/03/09
shipped (the SDK relocated to `ts/`, arch-split-packaged, published as `0.1.0`,
primer cut over); its 04–07 are REPLACED here by the structural rewrite (S1–S4);
its 01/08 are carried forward (A, R3). Delete `docs/road-to-1.0.0/` when this
packet's doc-reconcile PRD (E) lands.

## The design, ratified (no PRD re-litigates these)

1. **Structural, not nominal.** A field's value type is its bare structural
   type: `u64`→`bigint`, `i64`→`bigint`, `str`→`string`, `bool`→`boolean`,
   `bytes<N>`→`Uint8Array`, `interval<E>`→`{ start; end }` (half-open),
   `interval<E,W>` the same shape with a width label. **No brands, no phantom
   tags, no minting casts, no brand-by-assertion.** The old `Brand<>` machinery
   is deleted outright.
2. **Domains are labels in the schema type.** `.as("HolderId")` attaches a
   string domain label to a field's *descriptor type* (mirrors Rust's
   `as HolderId`); it does NOT brand the value. Same-string domains link fields.
   The old `.newtype` spelling is gone (it belonged to the brand era).
3. **The relational builders check domains structurally.** `contained`,
   `mirrors`, `window`, and query joins read the domain labels off the schema
   type and reject mismatched domains as a *compile error*, by structural
   comparison of descriptor shapes — never by a value brand. What the type
   layer cannot state (target-resolves-a-key, etc.) stays a typed `Db.create`
   error — the two-boundary split, engine as final authority. The one thing
   structural-B consciously does NOT catch at compile time is host-variable
   id-mixing on `insert` (a raw `bigint` in the wrong field) — the engine's
   containment judgment catches it at commit; recovering it would require the
   value brands we are deleting.
4. **Field-list positions.** `on(R, "x")` is the common single-field case;
   `on(R, ["a", "b"])` projects the composite/pointwise position (the
   interval-pointwise `==`/coverage recipes). One spelling, arity-generic.
5. **Free-function statements.** `key(R,[…])`, `contained(…)`, `mirrors(…)`,
   `window(…)` are free functions collected in the `schema()` statement array —
   closest to the dependency calculus's own notation. The count vocabulary
   `exactly/none/between/atLeast/atMost` partitions the legal windows; banned
   spellings have no constructor (unwritable, not checked).
6. **Query = Datalog as values, kysely-shaped.**
   `query(S).rule(r => r.match(Rel, { field: r.var("v") }).where(pred).select(…))`;
   string-named vars are domain-typed and join by reuse; params typed by use;
   `program(S, p => { const p1 = p.rec("p1"); … return p.output(…) })` for
   stratified recursion. No string parsing, no type-level string parsing.
7. **The SDK ships its own cookbook**, the 29 recipes in the structural API,
   compile-pinned like the engine's `cookbook.rs`.
8. **The elegance dividend to protect:** structural values make the marshal
   boundary pure both ways — there is no brand to assert on the way out, so the
   SDK becomes **cast-free** (the lone historical exception, the marshal
   brand-assertion, is deleted by this refactor). "Zero casts in product code"
   is now literal, not "except one."

## Frozen operational rulings

- **No release until owner approval.** No PRD bumps `ts/package.json`'s version,
  runs `npm/pnpm publish`, or creates/pushes a git tag. The `1.0.0` version bump,
  the `v1.0.0` tag, and every `publish` are owner ceremony (Wave 3). Pushing CODE
  to `main` is fine and expected (standing push discipline) — a release is not.
- **Fable only.** Every subagent runs Fable; no Opus.
- **Primer is untouched.** The hard break is unpublished; primer stays on
  `@bjornpagen/bumbledb@0.1.0` (a dev-dep) and its Vercel build is unaffected
  until the owner-approved republish (Wave 3). No PRD here touches primer.
- **No shims, hard break, rip to the end state.** PRDs are a work-organizational
  unit, NOT an atomic passing-code-state benchmark: between PRDs the tree MAY not
  typecheck, MAY have dead imports, MAY fail lint. S4 restores whole-SDK green as
  its own passing criterion; A stands alone green.
- **No test-only or migration PRDs.** Smoke/e2e tests and data migrations are
  human-owned and appear in no PRD. Type-level probes and unit-shaped assertions
  that are intrinsic to a code change (the `@ts-expect-error` unwritability pins,
  the panic-burn `#[test]`, the cookbook compile-pin) are the code and stay.
- **Engine-first ordering.** Engine (`crates/`) commits precede any consumer;
  the SDK build rebuilds its `.node` against the in-repo engine.

## The dependency graph

```
WAVE 1 — build (now, Fable fanout, parallel, no release)
  A  engine fresh panic-gap ─────────────────────────────┐  (parallel, standalone-green)
  E  doc reconcile ──────────────────────────────────────┤  (parallel, cheap)
  S1 field & domain kernel  ── foundation                │
        │                                                │
        ├── S2 statement algebra ──┐                     │
        └── S3 query surface ──────┤                     │
                                    └── S4 runtime+results+rejection (integrate, restore whole-SDK green)
                                              └── S5 SDK cookbook (needs S1–S4 real)
   → Gate (engine + SDK green) → Review (adversarial doctrine) → Land (commit+push code; NO version/publish/tag)

WAVE 2 — measure (idle machine only; owner go)
  C1 heed flags (NO_MEM_INIT + bulk APPEND)      C2 all-cores fuzz hunt

WAVE 3 — the 1.0.0 close (idle machine + owner ceremony; owner go)
  R1 bench re-true + charts + README  →  R2 version 1.0.0 + tag (owner pushes)  →  R3 republish SDK + primer bump
```

| PRD | Title | Wave | Repo | Depends on |
| --- | --- | --- | --- | --- |
| A  | Engine: fresh panic-gap drop-guard | 1 | bumbledb | — |
| S1 | The field & domain kernel (structural) | 1 | bumbledb `ts/` | — |
| S2 | The statement algebra & `schema()` | 1 | bumbledb `ts/` | S1 |
| S3 | The query surface | 1 | bumbledb `ts/` | S1 |
| S4 | The `Db` runtime, results & rejection (+ restore green) | 1 | bumbledb `ts/` | S2, S3 |
| S5 | The SDK cookbook | 1 | bumbledb `ts/` | S4 |
| E  | Doc reconciliation | 1 | bumbledb | — |
| C1 | heed flags: NO_MEM_INIT + bulk APPEND | 2 | bumbledb | (idle) |
| C2 | The all-cores fuzz hunt | 2 | bumbledb | (idle) |
| R1 | Bench re-true + charts + README | 3 | bumbledb | S4 (numbers stable) |
| R2 | Version 1.0.0 + annotated tag | 3 | bumbledb | R1 (owner) |
| R3 | Republish SDK + primer bump | 3 | both | R2 + owner-approved |

## The gates (every PRD proves its own)

- Engine: `scripts/check.sh` (fmt, clippy -D, workspace tests, alloc gate,
  crashpoint + kill sweeps, feature matrices) and `scripts/lean.sh` (build,
  zero-sorry/axiom, spec-census, conformance, three-way) both exit 0.
- SDK (`ts/`): `pnpm run build` (cargo bridge + tsc + both package trees green),
  `pnpm exec tsc --noEmit`, `pnpm exec biome check .`, and
  `node --test $(find test -name '*.test.ts')` 100% green. **`test/fixtures/*.ts`
  are spawned-child helpers, NOT tests** — never run them as tests.

## Open decision the owner holds (Wave 3)

Republish the structural SDK as **`0.2.0`** (hands teammates the structural API +
cast-free surface immediately; 0.x churn is expected) or **hold** the republish
until the `1.0.0` close. Either way it waits for the owner's explicit "publish."
