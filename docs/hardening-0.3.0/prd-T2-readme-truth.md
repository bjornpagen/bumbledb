# PRD-T2 — README truth pass (non-numeric)

Wave T · Repo: bumbledb · depends on: — (T1 owns every number; touch none here)

## Objective

Fix every verified non-numeric staleness in the root `README.md`. Each item
below was confirmed against the tree on 2026-07-18; re-verify each against
HEAD before editing (something may have moved), then fix exactly these.

## Work (the enumerated findings)

1. **Cookbook count**: "twenty-eight worked schemas" → **twenty-nine**
   (`docs/cookbook.md` ends at "## 29. The zone ledger").
2. **Repository layout section**: add the missing entries —
   `crates/bumbledb-theory` (the shared schema library: descriptors, SchemaSpec,
   the one lowering), `ts/` (the published npm SDK `@bjornpagen/bumbledb` —
   currently mentioned NOWHERE in the README), `lean/` (the Lean spec +
   conformance corpus), `fuzz/` (referenced in prose but absent from the
   layout), `docs/reference/`. List all ten `scripts/*` entries (currently 4 of
   10; missing `fuzz.sh`, `lean.sh`, `miri.sh`, `miri-cross-cc.sh`,
   `ramdisk.sh`, `spec-census.sh`) with one-line descriptions each.
3. **Dependency sentence** ("heed + blake3 are the only deps"): true to
   `crates/bumbledb/Cargo.toml` — name `libc` (with its no-graph-node
   justification) and the in-house `bumbledb-macros`/`bumbledb-theory`.
4. **Bench crate subcommands**: "(gen/verify/bench/trace)" → add `scenarios`
   and `verify-store` (the README's own recipe invokes `scenarios`).
5. **The gate-suite block**: describe what `scripts/check.sh` ACTUALLY runs at
   HEAD (read the script; at last audit it had grown doc tests, the
   ground-off/fold-off feature matrices, fuzz-crate clippy, deterministic
   crashpoint sweeps, the WRITEMAP kill smoke, the bench obs lane, the x86-64
   cross-check; the alloc gate needs `-- --test-threads=1`). Add
   `scripts/lean.sh` — the Lean gate is currently absent from the README
   entirely.
6. **Two new paragraphs**, in the README's voice: one acknowledging the
   TypeScript SDK (what it is, that it lowers through the same shared schema
   library and the same engine, pointer to `ts/README.md` + the npm package
   name); one acknowledging the Lean layer (the spec, zero-sorry law, the
   conformance three-way). Keep each under ~8 lines; the README's center of
   gravity stays the engine.

## Technical direction

- Do NOT touch any number, chart reference, or the tails sentence — T1 owns
  the entire numbers section and may land before or after this PRD.
- Match the README's existing register exactly (lowercase headers, the "the X —
  the Y" cadence, no marketing language).
- Re-verify each claim you write by opening the file it describes; a truth pass
  that introduces a new staleness has failed its own point.

## Passing criteria

- Each of the six items above is fixed, and `grep` confirms: no
  "twenty-eight", `bumbledb-theory`/`ts/`/`lean/`/`fuzz/` all present in the
  layout, all ten scripts listed, `lean.sh` mentioned, `scenarios` and
  `verify-store` named.
- Zero numeric edits: `git diff README.md` contains no changed digit outside
  the section-2 layout/count fixes above (the "29" is the one permitted digit).
- Every sentence added is verifiable against HEAD (spot-check each file path
  and script name you name).
- Commit in the repo's voice; push.
