# PRD-T3 — ts/README rewritten to the real surface, compile-pinned

Wave T · Repo: bumbledb `ts/` · depends on: K8 (documents the FINAL 0.3.0
surface) · ships with V1 (the registry copy only updates on publish)

## Objective

`ts/README.md` is the npm-facing README and its entire Quick start is the
deleted 0.1.0 nominal-brand API: it imports `match`, `type Brand`, `type Scope`
(none exported), calls `u64.newtype(...)` (deleted), uses the pre-structural
query shape, and narrates branded values. Nothing in it compiles. Rewrite it to
the post-K-wave surface and make it UNABLE to rot again by compile-pinning its
code blocks.

## Work

1. Rewrite the Quick start against the final 0.3.0 surface: `relation()` with
   derived fresh coordinates, `ref()`/`cites()`, `.as` only where a shared
   value domain genuinely has no owning declaration, free-function statements,
   ψ-selection where it teaches well, the `query(S).rule(r => ...)` shape with
   `vars()` and free comparisons, `Kind.match`, the 3-arg `closed`. The example
   should be one small coherent theory (reuse a cookbook recipe's shape rather
   than inventing a new one).
2. Purge the brand era from prose: "branded ids", "brands, fields…" — the
   surface is structural; say so in the README's own voice.
3. Sweep every listed export against `ts/src/index.ts` at HEAD: the Surface
   section must list exactly what is exported, no more, no fewer of the
   public names it chooses to show.
4. **Compile-pin the code blocks**: add `ts/test/readme.test.ts` that extracts
   every ```ts fence from `ts/README.md` at test time (read the file, slice
   the fences) and type-checks them as a program (the cookbook pin pattern:
   compile-time inclusion via a generated fixture or an in-test `tsc`
   invocation — follow the existing cookbook.test.ts mechanism; do NOT
   hand-duplicate the code). A README edit that breaks compilation must fail
   `node --test`.
5. Verify the runnable claims (install command, package names, the
   darwin-arm64 optional dep, `Db.exhume`, violations shape) against the
   `package.json`s and `index.ts` at HEAD.

## Technical direction

- The pin test is part of this change (intrinsic type-level assertion — house
  rule), lives in `ts/test/`, and runs in the normal suite.
- Keep the file honest about platform support (darwin-arm64 prebuilt; others
  build from source if that is what the packaging actually does — verify in
  `ts/scripts/build.ts` and the npm manifests before writing it).
- No version-number claims beyond what `package.json` says at the time V1
  stages 0.3.0 (write "0.3.0" only if V1 has landed the bump; otherwise write
  version-neutral prose — the criteria below force re-verification at V1).

## Passing criteria

- Zero occurrences in `ts/README.md` of: `newtype`, `Brand`, `Scope` (as an
  import), the old `query(S, ($) => ...)` shape, or the word "branded"
  describing values (grep-clean).
- `ts/test/readme.test.ts` exists, extracts the fences mechanically (edit the
  README's code → the test fails), and is green.
- Every import in every fence resolves against `ts/src/index.ts`; every listed
  export exists (spot-check by compilation, which the pin enforces).
- `pnpm exec tsc --noEmit`, `pnpm exec biome check .`, and the full test suite
  stay green. Commit in the repo's voice; push.
