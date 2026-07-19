# The cleanup-0.5.0 packet — the branchy-code purge, three surfaces reconciled

One move, executed across the engine, the SDK, and the operational shell: every
branch, flag, guard, and special case the three censuses caught is either killed
by representation, ratified as law with its citation, or sent to the Measure
phase to earn its keep with a number. The wave ships as **0.5.0**.

This directory is an execution-style PRD packet (ordered, strict passing
criteria). Per house convention it is **DELETED once shipped** — the durable
rulings must live in `docs/architecture/` before the packet dies (U6's passing
criterion). PR **#11** (branch `worktree-cleanup-050`) is the wave's review
surface: it **stays open; nobody merges** — a serial committer owns git.

## The doctrine — THE REPRESENTATION LAW

The biggest lever is the data representation, not the control flow.

- **The principle.** When you meet a branch, flag, guard, or special case, ask
  FIRST what representation makes the state unrepresentable — change the data,
  the types, and the invariants so the case stops being special or stops being
  expressible.
- **The mechanism.** Parse, don't validate: a check's proof moves into the type
  and happens once, at the boundary. Prefer sum types over independent flags.
  Half-open intervals. Dispatch over tag-switches. Control-flow-as-data where
  an evaluator clarifies.
- **The limit.** Essential complexity stays. Forcing two genuinely different
  cases into one representation just hides the branch in a flag. A kill that
  adds more machinery than it removes is not a kill — abort it and report.

## The register (the census corpus)

Three censuses, taken 2026-07-19 at the PR #10 branch tip (`7ca120aa`),
classifications KILL / KEEP-AS-LAW / OWNER-RULING-NEEDED per item:

- `/tmp/bumbledb-feature-research/cleanup-engine.md` — kind forks, features/cfg,
  dual paths, carve-outs, platform branches (~72 KEEP, 10 kills, 8 rulings).
- `/tmp/bumbledb-feature-research/cleanup-sdk.md` — dual spellings, two-tier
  twins, closed-vs-ordinary forks, packaging, the crate bridge (18 kills).
- `/tmp/bumbledb-feature-research/cleanup-ops.md` — the CI-runner truth, the
  red Miri cron, scripts, branches, docs hygiene.

**Census-tree drift (read before executing).** The censuses cite the PR #10
branch; THIS tree is main + the wave-start marker (`747104c2`). Divergences
that change a PRD's shape are recorded in that PRD; the load-bearing ones:

| census says (PR #10 branch) | this tree has |
|---|---|
| per-kind map_size split (32 GiB durable / 4 GiB ephemeral), `StoreKind::map_size()`, prd-G1 packet | ONE `MAP_SIZE = 4 << 30` (`storage/env.rs:168`), no per-kind split, no `docs/prds/incremental-images/` |
| `lineage-off` feature + Wave-M A/B twin | absent — never merged |
| `bench-out/waveM-*`, README bench conflict | PR #10's problem, not this wave's — out of scope, flagged to the serial committer |
| `scan_from` + the whole I1 copy-on-append machinery (U2 kill 6's target) | absent — the fork predates PR #10's merge; kill 6 is DEFERRED-TO-RECONCILIATION (recorded in prd-U2) |

Every other cited site was re-verified present here (the capacity contract,
WRITEMAP ephemeral flags, the eager-alloc pin, `covers`, `SameArity`, the
remaining engine kills' sites, the SDK kill sites). Line numbers are
census-time — re-locate before editing, never trust them blind.

**The copy-on-append reconciliation directive (binds the merge of this
branch, recorded 2026-07-19).** This worktree forked from main BEFORE PR #10
merged (merged 2026-07-19T17:19:41Z, 47 s before PR #11 opened), so the
entire I1 copy-on-append feature — the measured 2.54× — exists on merged
main but NOT here: `image/cache/advance.rs`, `get_or_build`'s
append/carry/corruption arms, `storage/read/scan.rs::scan_from`,
`storage/read/row_id_high_water.rs`, the delta accessors'
`dirty_relations`, `api/db/image_oracle.rs`, `api/db/append_tests.rs`,
fuzz oracle 6, and the `api/db/write.rs` commit epilogue
(`dirty_relations()` → `ImageCache::advance(new_generation, &dirty)` where
this branch still calls `cache.evict_older_than(report.new_generation)`).
Every test pinning the feature was dropped with the fork, so a
wrong-direction reconciliation reverts it with ZERO failing tests. The
three-way reconciliation MUST treat main's I1 file set as the base for
`api/db/write.rs`'s epilogue, `image/cache/**`,
`storage/read/{scan.rs,row_id_high_water.rs}`,
`storage/delta/accessors.rs`, `api/db/{image_oracle.rs,append_tests.rs}`,
and `fuzz/src/lib.rs` oracle 6, then re-apply this wave's U2 collapses on
top; verify by grepping the merged tree for `ImageCache::advance` and
running `append_tests` under `--features trace,image-oracle`. U2 kill 6
(scan delegation) executes in the same post-merge pass.

## The rulings, ratified (no PRD re-litigates these)

Fourteen rulings, taken at the censuses' presented defaults:

1. **Ephemeral goes lazy; WRITEMAP dies.** One `MAP_SIZE = 32 << 30` for both
   kinds (the owner's standing "32 GiB is the hard limit" ruling, unified).
   The capacity contract is retired: the eager preallocation machinery
   (`open_env.rs` `preallocate`/`preallocate_blocks`, two sanctioned-unsafe
   sites, the libc justification) is deleted; capacity refusal reverts to the
   filesystem's own lazy behavior. The ephemeral flag set drops `WRITE_MAP`
   and becomes `NO_SYNC`-only — the recorded fallback
   (`50-storage.md` § the ephemeral kind). The kind itself STAYS (on-disk
   identity, probe-first open, cross-open matrix — all law). U1.
2. **The kill class is blanket-approved.** Every item the three censuses
   classified KILL executes without further per-item ruling — subject to the
   limit clause above. U2 (engine), U3 (SDK), U4 (ops).
3. **CI gets an ubuntu engine lane; the Miri cron is fixed.** The "CI is
   linux-only" premise was false — CI is macOS-only. A linux runner starts
   executing the engine (check + test) so the linux arms stop being
   honest-but-untested fiction; `scripts/miri-cross-cc.sh` learns to
   stub-compile `.S` inputs under a stripped foreign target (the cron has been
   red 6/6 since inception). Both proven green by `workflow_dispatch`. U4.
4. **`lineage-off` dies.** The PRD-C1 gravestone precedent governs measurement
   twins: the knob never lands on main. In this tree there is nothing to
   delete; the ruling binds any future merge of PR #10 — the feature and the
   Wave-M A/B twin die in or immediately after that merge, gravestoned. U2
   records the gravestone.
5. **`covers` dies.** `pointIn(t, w)` is the one spelling of
   `ir::CmpOp::PointIn`; the name `covers` returns to `ALLEN.covers`
   exclusively. U3.
6. **Leaf elision: measure or merge.** The single-subatom leaf fast path +
   `run_leaf_pinned` (~55 unmeasured hand-inlined lines) gets one isolated
   measurement in M; a recorded win becomes law, no win merges it into the
   generic batch machinery. Until M rules, the code stands untouched.
7. **All-words finalize: measure or merge.** Same protocol
   (`api/prepared/finalize.rs` `fill_word_answers`). M.
8. **Permuted-identity determinant: measure or merge.** Same protocol
   (`storage/keys.rs` `determinant_image` vs `permuted_determinant_image`). M.
9. **`SameArity` gets its runtime twin.** The one type wall whose runtime seat
   is the engine alone becomes a construction-time check like its siblings —
   an untyped caller's arity-mismatched containment fails at the statement,
   not at `Db.create` after silent truncation. U3.
10. **The unsafe allowlist is reconciled.** `00-product.md`'s sanctioned-module
    list drops the stale `exec/run.rs` entry and gains `storage/env/open_env.rs`
    (as amended by ruling 1 — verify what unsafe survives U1),
    `alloc_counter.rs`, and bench `clockproxy.rs`. U6.
11. **Test-scaffolding unsafe becomes a named category** in the policy — the
    six inline-reasoned sites stop being an unnamed carve-out. U6.
12. **`ts/crate` enters the lint regime.** `unsafe_code = "deny"` +
    per-site `#[expect(unsafe_code, reason)]` over the ~35 napi FFI sites —
    the same wall the workspace crates already live behind. U4.
13. **The wave ships as 0.5.0.** Version bumps are staged by U4; publish and
    tag remain owner ceremony (the standing release law) — no PRD publishes.
14. **PR #11 stays open.** The convergence branch is a standing review
    surface; the serial committer owns git; no agent merges, stashes, or
    force-pushes anything, ever.

## The dependency graph

```
WAVE 1 — build (parallel)
  U1 ephemeral-lazy (engine)          U3 SDK kills (+ covers dies, SameArity twin)
  U2 engine kills (serialize with U1 in storage/env/*)
  U4 CI/ops (miri fix, ubuntu lane, hygiene) — its FFI-lint half AFTER U3

WAVE 2 — reconcile (after Wave 1)
  U5 lean reconciliation (the three surfaces agree, or findings)
  U6 architecture-docs alignment (retractions, allowlist, packet-survival check)

WAVE 3 — measure (idle machine only; owner go)
  M  the twins (rulings 6–8: measure or merge) + the ephemeral re-earn
```

| PRD | Title | Wave | Depends on |
| --- | --- | --- | --- |
| U1 | Ephemeral-lazy: one map, no contract, no WRITEMAP | 1 | — |
| U2 | The engine kills | 1 | U1 (shared files only) |
| U3 | The SDK kills + the two ratified adds | 1 | — |
| U4 | CI/ops + the FFI lint regime | 1 | U3 (lint half only) |
| U5 | Lean reconciliation | 2 | U1, U2 |
| U6 | Architecture-docs alignment | 2 | U1–U4 |
| M  | The Measure phase | 3 | U1 (re-earn), owner go |

## The gates (every PRD proves its own)

- Engine: `scripts/check.sh` and `scripts/lean.sh` both exit 0. The alloc gate
  stays green; no test, theorem, pinned margin, or probe is weakened — a pin
  that must die (ruling 1) dies with a gravestone, never a retarget.
- SDK (`ts/`): `pnpm run build`, `pnpm exec tsc --noEmit`,
  `pnpm exec biome check .`, `node --test` 100% green; zero casts in `ts/src`;
  `@ts-expect-error` only in tests, each real; every type claim keeps its
  runtime twin.
- Renames sweep citations: `scripts/spec-census.sh` green after every unit.
- Errors carry facts, never row ids. No new unsafe outside sanctioned modules.
- Perf claims obey the landing bar: numbers or explicit pending-measurement
  marks, never assertions. The Measure phase alone owns timing; check
  `ps`/`uptime` for foreign heavy processes before any cargo build.
- A code-vs-lean disagreement is a FINDING with evidence of which side
  strayed — never a silent fix of whichever is easier.
- Blocked honestly beats hacked green.
