# The Road to 1.0.0 — SDK relocation, hardening, and publish

One representation move and one quality bar, executed to the end: the
type-theoretic TypeScript SDK (`@bjornpagen/bumbledb`) moves out of primer and
becomes a first-class citizen of the engine's own repo under `ts/`; its builder
surface is hard-broken into end-to-end typesafety with zero back-compat; it is
packaged the modern arch-split way (the Biome/esbuild/napi-rs pattern) and
published to npm at `1.0.0` so coworkers `npm install` it; and primer is cut
over to consume it from the registry, its in-tree copy deleted. On the engine
side, the last open semantic (the fresh-mint panic gap) is closed so the engine
enters 1.0.0 with zero known issues.

This directory is an execution-style PRD packet (ordered, with strict passing
criteria). Per house convention it is DELETED once shipped.

## Reading order and dependency graph

| PRD | Title | Repo | Depends on |
| --- | --- | --- | --- |
| 01 | Engine: the fresh-mint panic gap, closed by a drop-guard | bumbledb | — |
| 02 | Reconcile + relocate the SDK into `ts/` | bumbledb | 01 |
| 03 | Arch-split packaging + the native loader | bumbledb | 02 |
| 04 | Harden: the field & brand kernel | bumbledb | 02 |
| 05 | Harden: the statement algebra & `schema()` | bumbledb | 04 |
| 06 | Harden: the query surface | bumbledb | 04 |
| 07 | Harden: the `Db` runtime, results & rejection wire | bumbledb | 05, 06 |
| 08 | Publish readiness | bumbledb | 03, 07 |
| 09 | Primer hard-cutover to the registry | primer | 08 (+ published) |

PRDs 04–07 (the hardening) all sit on 02 and may proceed in parallel; 07
integrates 05+06. 03 (packaging) is independent of the hardening and can land
any time after 02.

**PRDs are a work-organizational unit, NOT an atomic passing-code-state
benchmark.** Between PRDs the repo MAY not typecheck, MAY have dead imports, MAY
fail lint. No transitional shims are ever written — rip directly to the end
state. Each PRD's passing criteria stand for that PRD's own scope; whole-tree
green is re-established by the last PRD of each subsystem (07 for the SDK, 09 for
primer).

## Frozen rulings (owner-ratified; no PRD re-litigates these)

1. **Target: macOS Apple Silicon only, for now.** Packaging is the modern
   arch-split pattern (how Biome/esbuild/swc/napi-rs ship): a pure-JS main
   package with a per-platform binary package as an `optionalDependency`, `os`/`cpu`
   gated. More targets later = more platform packages + a CI matrix, never a
   redesign. Non-arm64 correctly compiles the engine's scalar fallbacks but
   carries no performance promise (`00-product.md`); today we ship exactly
   `darwin-arm64`.
2. **`@superbuilders/errors` stays a direct public runtime dependency.** It is
   public on npm. Do NOT refactor the SDK off it — the 141 call sites stay.
3. **Language-folder convention.** `ts/` holds the TypeScript SDK; `lean/` holds
   the Lean spec; `crates/` holds the Rust engine. The SDK's napi bridge lives
   at `ts/crate/` and is **NOT** a member of the Cargo workspace — the engine's
   heed+blake3-only dependency law binds the engine crates, and the bridge's
   `napi` dependency must never enter that graph.
4. **Hard break, zero back-compat, no shims.** This is a pre-1.0.0 unstable
   surface with no compatibility contract. Rip to the end state; never preserve a
   spelling for migration. There is one consumer (primer) and it is repointed
   wholesale at the end.
5. **The hardening doctrine: representation over control flow.** Every illegal
   state is made unrepresentable in the types, not guarded at runtime; parse at
   the boundary and carry the proof in the type; the builders thread types from
   schema declaration through query construction to results with zero casts and
   clean hovers. A runtime guard that a type could forbid is a defect the
   hardening exists to delete. (The one sanctioned exception, already in place,
   is literal *typing* at the boundary where TS cannot express it — e.g. an async
   callback probe — matching the engine's two-boundary split.)
6. **No test-only or migration PRDs.** Smoke tests, end-to-end tests, and data
   migrations are human-owned and appear in no PRD. Type-level probes and
   unit-shaped assertions that are an intrinsic PART of a code change (e.g. the
   `expect-error` probes that pin an unwritable spelling) are the code, and stay.
7. **Owner ceremony — NOT PRDs.** The benchmark re-true, chart regeneration, and
   README number-truing (Phase D of `TODO.md`); the `Cargo.toml` bump to `1.0.0`;
   the annotated tag; and the `npm publish` invocations are the owner's, run on an
   idle machine. This packet PREPARES every artifact those steps consume (§ Owner
   close-out below); it never performs them.
8. **Versions lockstep at `1.0.0`.** The main package and the platform package
   ship the same version, pinned exact; the SDK's `1.0.0` corresponds to the
   engine's `1.0.0` tag.
9. **Engine-first ordering law.** bumbledb commits always precede the primer
   commits that consume them. The primer cutover (09) is last and is gated on the
   npm packages actually being published.
10. **The move takes the UNION.** The relocated SDK is primer main's SDK (the 8
    bug-fixes — `__proto__`/async-commit/surrogate/mirrors/reader-leak seam
    refusals + multi-key `get`) UNIONED with PR #70's exhume surface (`exhume.ts`,
    the bridge exhume export, the legacy-store fixture). PRD-02 reconciles both;
    it must not silently drop either lineage's improvements.

## Reconciliation state (read before PRD-02)

As of packet authoring: primer main carries the 8 SDK bug-fixes; PR #70
(`worktree-course-serialization`) carries the exhume/self-describing-stores TS
surface and read as OPEN with `exhume.ts` absent from `origin/main`. PRD-02 is
written to be correct EITHER WAY — if #70 has merged to primer main by execution
time, the union is trivially primer main's SDK and the exhume steps are no-ops;
if not, PRD-02 performs the 3-way union (`db.ts` and `index.ts` are the only
files both lineages touch). The engine side of self-describing-stores is already
on bumbledb main (`c79c2b38` + the fresh fix), so the exhume bridge builds
against the in-repo engine regardless.

## Engine facts the packet builds on (current, verified)

- bumbledb `main` carries: the `bumbledb-theory` extraction + facade, the W-ledger
  (W1/W3/W4/W8 landed, W2/W5/W6/W7 gravestoned/recorded), self-describing stores
  (`descriptor_codec.rs`, `api/db/exhume.rs`, `Error::DescriptorMissing`), the
  SysV→POSIX-sem EINVAL fix, and the unconditional fresh-never-reissue law
  (`lean/Bumbledb/Txn/Fresh.lean`, the collapsed `Reachable.txn`). Gates:
  `scripts/check.sh`, `scripts/lean.sh`.
- The known-issues floor is empty EXCEPT the fresh-mint panic gap that PRD-01
  closes.

## Owner close-out (performed by the owner after the packet lands green)

1. Bench re-true on an idle machine: `bumbledb-bench gen && verify`, 3 durable + 3
   ephemeral `bench` runs + `scenarios`, `scripts/bench_viz.py` regenerate all five
   `assets/*.svg`, and true every numeric claim in `README.md` (W1/W3/W4/W8 moved
   them). Measurement law applies (`scripts/measure.sh`, ±2%).
2. `Cargo.toml` workspace version → `1.0.0`; commit; prep the annotated tag.
3. Push the tag (the owner's ceremony).
4. `npm publish` the platform package then the main package (§ PRD-08 runbook),
   with `--provenance` if a CI publish workflow exists by then.
