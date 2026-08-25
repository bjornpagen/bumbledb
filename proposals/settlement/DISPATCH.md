# DISPATCH — orchestrator prompt: run the endgame to the receipt

Paste everything below the line into the orchestrator.

---

You are the **orchestrator** for the endgame of the bumbledb-log
cutover, in the repo at the working directory. The six representational
decisions are landed; the proposals rotated: the old PRD set and
`representation-first-cutover/` are **deleted**, and the one law is now
`proposals/settlement/00-canon.md`. Your mission is
`proposals/settlement/10-endgame.md`, E1 through E4, run to the receipt.

**Standing rules — all proven in the buildout, all still binding:**

- **Run to completion.** No checkpoint waits on the owner; every
  ambiguity has a decision rule; rulings are logged in
  `proposals/settlement/RULINGS.md`, never escalated. The only human
  step is `git push origin HEAD`, after the receipt, and you do not wait
  for it.
- **Massively parallel.** One worker per file or per spec bullet; all
  disjoint lanes concurrent (Rust ∥ TS); recon and verification fan out
  wide. If you have idle capacity and unowned disjoint files, you are
  under-fanned — dispatch more. No two live workers write one file.
- **Chaos in flight, green once.** Mid-flight type errors and red
  suites are expected while E1 lands; do not freeze on them. Green is
  owed exactly once, at E2, and E2 iterates until green without waiting
  on a human.
- **Hard bounds** (violating agents are killed and reverted): never
  `git push` / `npm publish` / `cargo publish` / `git tag` (owner
  ceremony, `ts/PUBLISHING.md`); **no branches, no worktrees** — every
  commit lands directly on `main`, one tree; never weaken, skip, or
  delete a failing test to get green (migrating assertions to the new
  named outcomes is strengthening and required; loosening is a
  violation); do not touch `night-2026-08-22/`, `docs/research/`,
  `lean/conformance/cases/`, or generated C headers except via cbindgen;
  the comment law (comments state what is, present tense; never
  "previously/removed/now/new/refactor" — `scripts/comment-diff-guard.sh`
  enforces it); zero backwards compat — no shim, no dual-read, no
  feature flag, no "legacy" arm, ever.
- **Cancelled by rotation — read this twice:** the parent PRDs
  (`proposals/00-*.md` … `proposals/90-*.md`) and
  `proposals/representation-first-cutover/` are deleted. Any in-flight
  work aligning or referencing those files is moot. Do NOT recreate
  them, do NOT commit pending edits to them, do NOT re-add their
  spellings. The law is `settlement/00-canon.md`; the open spec is
  `settlement/20-one-encoding.md`; the audit target is
  `settlement/90-traceability.md`.

**Ground truth, read in order before any edit:**
`proposals/settlement/README.md` → `00-canon.md` (the law; its six
invariant clauses are binding) → `10-endgame.md` (your mission) →
`20-one-encoding.md` (E1's spec, implement as written; its invariant
block is the contract) → `90-traceability.md` (E3's audit target) →
`RULINGS.md` (append-only rulings log).

Code geography: Rust driver `crates/bumbledb-log/src/` (own cargo
workspace — root `--workspace` commands do NOT reach it; its battery
runs from inside the crate). TS driver `ts-log/src/` + `ts-log/test/`.
Lambda example `examples/lambda/`. Shared conformance inventory
`crates/bumbledb-log/conformance/v3/`.

## E1 — One encoding, one coordinate (the last representational change)

Implement `20-one-encoding.md` exactly:

1. **Documents go binary.** Manifest, checkpoint document, and sidecar
   become binary records over the existing `u64le`/`u32le`/
   length-delimited/`[u8;32]` primitives, batch-codec style: leading
   version byte = **3** (the binary format IS v:3; the JSON interlude
   never shipped; anything without the binary magic is refused, which
   subsumes the v:2 refusal). `blake3(bytes)` with no canonicalization
   clause. Keys rename: `manifest.json` → `manifest`,
   `ckpt/{digest}.json` → `ckpt/{digest}`, local `chain.json` → `chain`;
   `.mdb` keeps its suffix.
2. **Delete the second grammar**: `serde_json` from `bumbledb-log` and
   every `json!`/`from_str` site; `ts-log/src/document.ts`; the JSON
   halves of `manifest.rs`, `sidecar.rs`, `manifest.ts`, `chain.ts`;
   every BOM/whitespace/leading-zero/duplicate-key/quoted-bigint/
   hex-width arm (~79 sites) — they guard states bytes cannot express.
3. **`Vector` owns its algebra**, both drivers: `sum() -> u64|Overflow`
   (the ONLY overflow site), `dominates(o)`, `order(o) ->
   CheckpointOrder`, `at(braid)`/`advance(braid)`. Retarget every
   hand-rolled loop (`gc.rs`, `replica.rs`, the `checkedAddU64` parse
   sites) into calls. One encode function for `Vector` in the one
   grammar.
4. **Re-render the corpus**: document goldens become binary; hex dumps
   may live in `inventory.json` as test metadata; refusal fixtures
   shrink to what bytes can refuse (truncation, bad magic, trailing
   bytes, unknown braid, overflow). Both drivers walk the re-rendered
   inventory.
5. **Inspectability is a tool**: `duty inspect <key>` renders any
   document to text.
6. **E1b — no raw regex** (after E1's deletions, never before): add the
   `arkregex` package (pnpm) and replace every raw regex literal and
   `new RegExp` in `ts-log/src`, `ts/src`, and `examples/lambda/src`
   with its typed patterns. Do not port a regex E1 deletes (the
   `CKPT-SCRATCH` text capture, hex-shape checks). Survivors to port:
   the lease/temp name grammars in `store.ts`, the key prefix trim in
   `keys.ts`, the `.id` newtype suffix in `descriptor.ts`, and the
   engine-SDK sites in `ts/src`. Refusal identities unchanged; zero raw
   regex literals remain (grep proof joins E3.1).

The diff must come out net-negative by hundreds of lines; if it does
not, something was implemented instead of deleted.

## E2 — Reconciliation: green once

Finish every test migration to the landed shapes, then loop the full
battery until green (the battery is enumerated in `10-endgame.md` §E2 —
note item 2: the log crate's own-workspace battery runs from inside
`crates/bumbledb-log/`; a green that skipped it is fake). Attach the one
green transcript to the receipt.

## E3 — Proof

1. **Grep-for-absence transcript** over `crates/bumbledb-log/src`,
   `ts-log/src`, `examples/lambda/src` — the full list in
   `10-endgame.md` §E3.1, including the E1 absences (`serde_json`,
   `document.ts`, hex digests in document paths, quoted-decimal u64,
   `.json` StoreKeys, BOM arms). Zero hits or the endgame is not done.
2. **The 141-row adversarial audit** of `settlement/90-traceability.md`:
   every row, refutation-briefed verifiers, two per critical (rows
   0–9), one per major/minor, file:line citations of the type or
   invariant that stopped the refuter. Any refutation reopens the owning
   work. Batch rows by decision so each verifier reads one canon section
   deeply.
3. **Lockstep version bump**: one new number across every manifest the
   lockstep gate compares (root workspace crates, napi crate,
   `bumbledb-c`, `ts/` main + both platform packages, `ts-log/` peer
   range); lockfiles re-derived; one commit. No publish, no tag.

## E4 — The receipt

One receipt commit: per-stage hashes, battery transcript, grep
transcript, 141 verdicts, the rulings log, the deletion tally, and the
one line the owner runs: `git push origin HEAD`. Commits throughout are
house style — long-form prose stating what the representation now *is*
and what died, ending with `Named deletions:` and an itemized tally;
study `git log` for the voice.

## Autonomy protocol

Document conflict → `00-canon.md` wins on everything (there is no other
law); between canon and `20-one-encoding.md`, 20 wins on the encoding
seam until it lands, then canon absorbs it. A spec that cannot land as
written → the invariant block is the contract, the spelling flexes, the
ruling is appended to `RULINGS.md`. Red → fix forward; never wait. A
worker stalls or produces garbage → kill, revert its files, re-dispatch
sharper; twice → split the brief. Everything except the hard bounds is
yours to decide and log.

Begin with E1 recon (map every JSON site and every hand-rolled Vector
loop to its worker), fan out, and do not stop for anything short of a
hard bound until the E4 receipt is on `main`.
