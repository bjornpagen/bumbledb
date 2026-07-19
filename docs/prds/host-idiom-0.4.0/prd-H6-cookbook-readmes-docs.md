# PRD-H6 — Cookbook, READMEs, docs: the drizzle law recorded

Wave H · Repo: bumbledb `ts/` + `docs/` · depends on: H1–H5 (documents the
end state)

## Objective

Every teaching surface speaks the 0.4.0 idiom, and the drizzle law becomes
recorded doctrine so future surface work inherits it without relitigating.

## Work

1. **`ts/COOKBOOK.md`** — sweep all 29 recipes:
   - every closed-handle literal becomes the string
     (`Kind.Savings` → `"Savings"`), selections/membership become arrays,
     and any recipe that dispatched on handles teaches the NATIVE idioms:
     `switch` with `satisfies never` exhaustiveness AND the record-table
     idiom (`Record<Infer<typeof R.fields.kind>, T>`) — show each once, in
     the recipe where it reads best, not in every recipe;
   - the ψ recipes (7/8) keep their statements verbatim — only value
     spellings change;
   - delete every `oneOf`/`fromId`/`match` mention; a `fromId`-shaped
     decode step appearing anywhere in a recipe is a defect (rows arrive
     named).
2. **The compile-pin** (`ts/test/cookbook.test.ts`): update every pinned
   construction to the rewritten text; the pin mechanism itself unchanged.
   **The zero-store-surface proof rides here**: the T5 fingerprint fixture
   (`ts/test/fixtures/cookbook-fingerprints.txt`) must be BYTE-IDENTICAL —
   no regeneration, no value drift; a changed line means the packet touched
   the store surface and is a stop-the-line defect. Same for the CrossHost
   lock constants (both files).
3. **`ts/README.md`**: the Quick start speaks strings-and-switch (it is
   compile-pinned by `readme.test.ts` — the pin forces honesty; update
   both together). The Surface section drops the dead names, gains the
   drizzle law in one sentence.
4. **`docs/architecture/70-api.md`** (the SDK chapter): a short recorded
   ruling — the drizzle law verbatim from the packet README, the
   handle-union texture, the orderable ban (cross-reference the
   data-model "order is an accident" line), and the ruling-9 extension
   (Rust's host enum ↔ TS's literal union; the theory stores ids; the
   marshal owns the bijection).
5. **`docs/architecture/10-data-model.md`**: one sentence where the closed
   vocabulary is described — the TS surface speaks handle names; ids are
   the store's encoding.

## Technical direction

- Recipes must be BETTER text after the sweep — update any prose the new
  idioms falsify (decode steps, "mint the constant", reserved-name
  caveats).
- Guarantee lines citing lean theorems: untouched (run
  `scripts/spec-census.sh` if any citation looks adjacent).
- No engine docs beyond the two named files; the macro chapter is not
  touched (Rust is unchanged).

## Passing criteria

- `grep -n "oneOf\|fromId\|\.match(\|reservedHandle" ts/COOKBOOK.md
  ts/README.md` → zero (allowing `r.match(` — the query atom — refine the
  grep to exclude it and state the exact pattern used).
- `cookbook.test.ts` + `readme.test.ts` green against the rewritten text.
- The fingerprint fixture and CrossHost constants byte-identical
  (`git diff` shows no change to either).
- The two architecture files carry the rulings; `spec-census.sh` clean if
  citations were touched.
- `pnpm exec biome check .` clean on touched files. Push per the wave's
  commit discipline.
