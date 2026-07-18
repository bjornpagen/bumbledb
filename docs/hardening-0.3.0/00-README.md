# The hardening-0.3.0 packet — truth repinned, domains derived, ψ shipped, every consumer cut over

The pre-1.0.0 hardening wave, end to end, once and for all. Six investigations
(2026-07-18, all grounded in code or working /tmp prototypes) produced the design
memo this packet executes. It ships as **0.3.0** — another deliberate hard break
over the published structural `0.2.0`. Per house convention this directory is
DELETED once shipped.

## Ratified rulings (no PRD re-litigates these)

1. **Nominal refs are dead on mechanics, not taste.** TypeScript cannot mint
   per-call `unique symbol` identity (calls unify — prototyped). Domains stay
   STRINGS in descriptor types; values stay bare. No brands anywhere, ever.
2. **Domains are DERIVED, not declared, where a declaration exists.**
   `relation()` mints the coordinate label `` `${Name}.${field}` `` for **fresh
   fields only** (B-min). `ref(R, "f")` copies the target descriptor + domain and
   marks the reference; `cites(R, "f")` is the same link with NO derived
   statement (mandated by the Calendar recipe, where the only lawful link is a
   selected `mirrors`). `.as` survives for shared value domains with no owning
   declaration (`"Minor"`, `"Digest"`) and is **dot-banned** (type-level and
   runtime) so coordinates are unforgeable through public constructors.
   B-max (coordinates on all unlabeled fields) is explicitly NOT in this packet.
3. **Derived containments yield to hand-written twins** (dedupe by rendered
   string; the hand copy stays). This deliberately diverges from the
   fresh-implied-key law ("redundant — rejected") because statement ORDER is
   fingerprint-hashed and the yield rule is what keeps migrated stores stable.
   Genuinely new derived statements TAIL-APPEND in relation-declaration ×
   field-declaration order. Both choices are pinned forever once shipped.
4. **ψ-selection is pure SDK.** The engine folds selections against sealed
   extensions at validate (`compile_member_set`); the macro already accepts
   `Kind(id | mastered == true)`; the napi marshal passes selections through.
   Only two TS surface holes exist (statements: `closed().where`; query atoms:
   `.match` over closed) and this packet closes both.
5. **The key arrow is canon — OWNER RULING (2026-07-18, overrules the earlier
   memo).** `R(a, b) -> R` stays: it is the dependency-theoretic spelling — the
   key projection determines the tuple, the arrow closing over its own relation
   is what makes a key a key. It is never respelled, in either host's renderer.
   M1 RATIFIES and documents this reading instead of changing it.
6. **The semantic-parity law (the core goal).** The TS SDK and the Rust macro
   express EXACTLY the same theory semantics — same statements, same lowered
   descriptors, same fingerprints, same query IR — while each speaks its host's
   idiom (Rust: declared `as NewType` + the notation; TS: derived coordinates +
   value builders). Flavor may differ; meaning may not. Every capability lands
   in both hosts or is explicitly ratified as host-idiomatic sugar over
   identical lowering (`ref`/`cites` are sugar — the derived statements are
   ordinary statements Rust writes by hand). The mechanical referees: the T5
   per-recipe fingerprint goldens, the M4 notation⇄IR corpus, the render-golden
   byte-pin, and the CrossHost lock. A capability with no referee is not done.
7. **Canonical utterance holds everywhere**: when a spelling changes, the old
   spelling is DELETED, not dual-accepted. Ordered-dense idb heads (`reach(m)`)
   replace `reach(0: m)` for the dense case; the curried tier-2 `closed()()`
   dies when the 3-arg form lands.
8. **Store safety is proven, not assumed**: fingerprints hash canonical
   descriptor bytes (`bumbledb-schema-v4`) — never syntax, never spellings,
   never domain labels. Every syntax PRD carries a fingerprint-pin-unchanged
   criterion as its proof.
9. **Rust stays declared AND unchanged — owner ruling 2026-07-18 ("option
   1").** The macro's notation is untouched: `as NewType`, `fresh`, and every
   statement form stay exactly as written. Macro-side `ref`, projection
   sorts, signature blocks, and "structural Rust" (deleting newtypes) are all
   EXPLICITLY REJECTED — newtypes are Rust's native wall and the notation is
   the owner's dependency-theoretic canon. What lands instead is M5: the
   taxonomy becomes CHECKED — newtype coherence across every statement's
   paired faces, one check in the shared lowering, surfaced as a spanned
   macro error and as the engine twin of TS's `SameDomains` wall. TS keeps
   K3/K4 derivation as host sugar over ordinary statements; labels never
   reach the store, so twins fingerprint identically when
   names/types/statements agree — and twin COOKBOOK recipes follow the tail
   convention for statement order so T5's per-recipe equality holds.
10. **No 1.0.0 anywhere in this packet.** The owner has explicitly deferred it.
   No agent bumps to 1.0.0, tags, or publishes; 0.3.0 staging is agent work,
   the publish itself is owner ceremony.

## Frozen operational rulings

- Code changes only. No smoke-test or e2e-test PRDs (humans test); no migration
  PRDs (humans migrate; where a schema change moves a fingerprint the PRD SAYS
  so and stops). Type-level probes and unit-shaped assertions intrinsic to a
  code change ARE the code and stay.
- No transitional shims, no deprecation windows, no dual-accepted spellings.
  Cut to the end state. The tree MAY be red between PRDs; PRDs are a
  work-organization unit, not an atomic passing-code checkpoint. K8 restores
  whole-SDK green; the final gate is the only place everything runs.
- Measurement law: T1 (bench repin) requires a genuinely idle machine and owner
  go; absolute numbers under co-tenancy are void.
- Engine-first ordering where it applies; every commit in the repo's voice.

## The dependency graph

```
PARALLEL START
  T2 readme-truth   T4 todo-collapse   T5 fingerprint-goldens   T6 ci-lane
  M1 key-arrow-ratified  M2 idb-heads  M5 coherence-check
  K1 psi-statements      K3 coordinate-kernel
THEN
  M3 cookbook-queries-compiled   (after M2 — queries written in the final notation)
  M4 notation-corpus             (after M2 — pins the final notation)
  K2 psi-query-atoms             (after K1)
  K4 derived-statements          (after K3)
  K5 vars-and-comparisons  K6 closed-ergonomics   (independent; anytime)
THEN
  K7 cookbook-and-lock   (after K1 K2 K3 K4 K5 K6)
  K8 sdk-green           (after all K)
  T3 ts-readme-rewrite   (after K8 — documents the final surface; ships at V1)
THEN
  P1 primer-schema → P2 primer-sweep   (after K8)
T1 bench-repin: any time the machine is idle and the owner says go; exclusive.
V1 release-staging: last; owner publishes.
```

| PRD | Title | Depends on |
| --- | --- | --- |
| T1 | Bench repin — one rev, matching artifacts, honest numbers | idle + owner go |
| T2 | README truth pass (non-numeric) | — |
| T3 | ts/README rewritten to the real surface, compile-pinned | K8 |
| T4 | TODO.md collapsed to reality | — |
| T5 | Cross-language fingerprint goldens, per recipe | — |
| T6 | The CI lane that runs the locks | — |
| M1 | The key arrow ratified — the FD reading documented, never respelled | — |
| M2 | Ordered-dense idb heads | — |
| M3 | Cookbook queries compiled — comments become `query!` | M2 |
| M4 | The notation conformance corpus | M2 |
| M5 | The coherence check — newtype agreement across statement faces | — |
| K1 | ψ statements: `closed().where()` + the face arms | — |
| K2 | ψ query atoms: `.match`/`not` over closed relations | K1 |
| K3 | The coordinate kernel: derived domains, `ref`, `cites`, the dot-ban | — |
| K4 | Derived statements: synthesis, dedupe, tail order, closed refs | K3 |
| K5 | `vars()` + free comparison exports | — |
| K6 | Closed ergonomics: `Kind.match` + the 3-arg `closed` | — |
| K7 | The SDK cookbook rewritten + the cross-host lock extended | K1–K6 |
| K8 | Whole-SDK green restored | all K |
| P1 | Primer: the store schema goes derived | K8 |
| P2 | Primer: the full 0.3.0 cutover sweep | P1 |
| V1 | 0.3.0 release staging (owner publishes) | everything |

## The gates (run in full only at the end)

- Engine: `scripts/check.sh` and `scripts/lean.sh` both exit 0.
- SDK (`ts/`): `pnpm run build`; `pnpm exec tsc --noEmit`;
  `pnpm exec biome check .`; `node --test $(find test -name '*.test.ts')` —
  100% green. `test/fixtures/*.ts` are spawned-child helpers, never tests.
- Primer: `pnpm typecheck` (all turbo tasks), `pnpm knip`, and the
  graph-builder test files green.
- Mandates swept: zero casts in `ts/src` (`as`-cast/`any`/`!`/unknown-launder);
  `@ts-expect-error` only in tests and each real; no underscore-prefixed
  functions or dead underscore params; every type claim has its runtime twin.
