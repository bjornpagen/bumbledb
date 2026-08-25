# DISPATCH — orchestrator prompt: the lockstep pass, recon to receipt

Paste everything below the line into the orchestrator.

---

You are the **orchestrator** for the lockstep pass on the bumbledb repo
at the working directory: the hard refactor that gives every remaining
duplicated fact one writer, then proves the whole campaign and cuts the
receipt. The law is `proposals/settlement/00-canon.md`; the mission is
`proposals/lockstep/` — five decision docs, executed in dependency
order, run to the receipt. This dispatch supersedes every prior
dispatch in the repository.

**There are no decisions left in this pass.** Every design question is
already ruled in the lockstep docs; where prose could be read two ways,
this dispatch's spelling wins. You never choose between designs, never
present options, never hold a question for the owner — you implement,
and when a spelling genuinely cannot land, the owning doc's invariant
block is the contract, the spelling flexes to the nearest form that
keeps it airtight, and the ruling is appended to
`proposals/settlement/RULINGS.md`. Logged substitutes only; asked
questions never.

**Standing rules — all binding, all proven on the prior passes:**

- **Run to completion.** No checkpoint waits on the owner. The only
  human step is `git push origin HEAD` after the receipt, and you do
  not wait for it.
- **Massively parallel, always.** One worker per file or per spec
  bullet; every disjoint lane concurrent; recon and verification fan
  out to dozens. A lane is a pipeline of many small briefs, never one
  agent grinding. If you have idle capacity and unowned disjoint files,
  you are under-fanned — dispatch more. No two live workers write one
  file; that partition is the only serialization the pass permits
  beyond the L-order below.
- **Chaos in flight, green once.** Mid-flight red is expected while
  representations move; never freeze on it, never spend briefs chasing
  a green the next lane will break. Green is owed exactly once, at
  L5's battery, and that loop iterates without waiting on a human.
- **Hard bounds** (violators are killed and reverted): never `git push`
  / `npm publish` / `cargo publish` / `git tag` (owner ceremony,
  `ts/PUBLISHING.md`); **no branches, no worktrees** — every commit
  directly on `main`, one tree; never weaken, skip, or delete a failing
  test to get green (retyping assertions to `Digest32` and named
  outcomes is strengthening and required; loosening is a violation); do
  not touch `night-2026-08-22/`, `docs/research/`,
  `lean/conformance/cases/`, or generated C headers except via
  cbindgen; the comment law (comments state what is; never
  previously/removed/now/new/refactor —
  `scripts/comment-diff-guard.sh` enforces); zero backwards compat — no
  shim, no dual-read, no fallback to a dead spelling, no compatibility
  window of any kind.
- **Read before any edit**: `proposals/lockstep/README.md` →
  `00-thesis.md` → the five decision docs 10–50 (their invariant blocks
  are the contracts) → `settlement/00-canon.md` and
  `settlement/RULINGS.md` (you will amend both) →
  `settlement/90-traceability.md` (L4's audit target).
- **Commit discipline**: house style — long-form prose stating what the
  representation now *is* and what died, ending with
  `Named deletions:` and an itemized tally. Study `git log` for the
  voice. Nothing rides uncommitted between lanes.

## L0 — Recon (dozens of readers, minutes not hours, no edits)

Fan readers over: the two `[workspace]` stanzas and both lockfiles;
all fourteen version-bearing manifests plus `ts-log`'s peer range;
every spelling of the battery (CI YAML, scripts, settlement prose);
every `hex32`/`digest32FromHex` call in both drivers with its boundary
classification; every `ckpt_json_key` site; every token of 50 §1's
absence list and the current census; `check.sh`'s line-by-line overlap
with the battery lanes. Output: the work map — every site → owning doc
→ worker brief. Roll straight into L1.

## L1 — One workspace (`10-one-workspace.md`) — unconditional

Merge `crates/bumbledb-log` into the root workspace. Delete its
`[workspace]` stanza and its `Cargo.lock`; move `.config/nextest.toml`
to the repo root and **commit it**; the crate adopts
`version.workspace = true`, `edition.workspace = true`, and the root
`workspace.lints` with **no per-crate overrides** — lint fallout is
fixed in the code, not allowed around. Collapse CI's per-manifest
whole-suite invocations to `--workspace` forms; the S3 smoke keeps its
targeted run as a nextest filter, not a second manifest. Re-derive the
one root `Cargo.lock`. Sweep every currently-dirty file and the
untracked crate README into this lane's commits. **The merge has no
fallback design.** Friction moves the code and the lockfile until it
lands; the structure does not change back.

## L2 — One version (`20-one-version.md`) — after L1

Root `[workspace.package] version = "0.19.0"`; every workspace crate
moves to `version.workspace = true`. Write the roster file — the two
excluded Cargo.tomls (`ts/crate`, `crates/bumbledb-c`),
`ts/package.json`, `ts/npm/darwin-arm64/package.json`,
`ts/npm/linux-arm64/package.json`, `ts-log/package.json` — and rewrite
the lockstep gate to three checks: every roster manifest equals the
workspace version; a tree sweep proves the roster complete (a
version-bearing manifest off-roster is a gate failure); `ts-log`'s
peer range equals `^0.19.0` exactly. Re-derive lockfiles. Rewrite
`ts/PUBLISHING.md`: the 0.19.0 section leads with the breaking-store
banner — **0.19.0 reads nothing 0.18.0 wrote; binary v:3 documents;
`manifest`/`ckpt/{digest}`/`chain` keys; no migration path; re-checkpoint
from a 0.19.0 writer** — then the publish order with peer `^0.19.0`.

## L3 — One battery ∥ one identity (concurrent; disjoint files)

**Battery lane (`30-one-battery.md`)**: write `scripts/battery.sh` as
the single definition of green — fail-fast, in order: `cargo fmt --all
--check` → `cargo clippy --workspace --all-targets -- -D warnings` →
`cargo nextest run --workspace` → the reduced `check.sh` remainder →
`scripts/lean.sh` (0 disagreements) → `scripts/spec-census.sh` → `ts/`
trio → `ts-log/` trio (each package's own scripts, `pnpm`). The script
self-provisions its one tool with the one line
`cargo nextest --version || cargo install cargo-nextest --locked` — no
artifact URLs, no platform arms. CI's correctness job becomes an
invocation of the script; any CI step or document naming a correctness
script other than `scripts/battery.sh` is deleted. Reduce `check.sh` to
only what battery lanes 1–3 do not cover (the comment guard, the
census); every duplicated line dies, and if nothing unique remains,
`check.sh` dies whole. Move `tiny_end_to_end_measures_both_engines`'s
duration comparison to the bench lane; a structure assertion (both
engines exercised, output well-formed) stays in the battery. Ruled, not
optional. Nothing in the battery reads a clock to decide pass/fail.

**Identity lane (`40-one-identity.md`)**: TS `Manifest`/`Checkpoint`
values carry branded `Digest32` (32-byte `Uint8Array`) for fingerprint,
checkpoint, hash, catalog, prev — `hex32`/`digest32FromHex` leave every
protocol parse/encode path. Hex survives at exactly four boundaries:
`duty inspect` output, refusal text, the key grammar's one
digest-to-key function, test metadata. A fifth caller is a census
failure. Rename `ckpt_json_key` → `ckpt_doc_key`, every site. Land the
binary/text boundary paragraph in `settlement/00-canon.md` §6:
machines write binary; humans write text — the theory file and inspect
output are the text half. Retype every test that touched the hex
surface; that retyping is the strengthening this pass ships.

## L4 — Proof as gate (`50-proof-as-gate.md`) — after L3

1. Write the banned-token roster — the full absence list in 50 §1
   (`pid_alive`, `pidAlive`, `applied_pending`, `kill(0)`, `kill -0`,
   `refresh_braid`, `upsert`, `Ok(status.success())`, `ESRCH`,
   `gc fodder`, `serde_json`, `document.ts`, base64 pending,
   JSON-`number` u64, `manifest.json`, `chain.json`, `.json` store
   keys, BOM/whitespace/leading-zero/duplicate-key arms, quoted-decimal
   u64, hex on protocol parse/encode paths, `_json` in identifiers, raw
   regex in the TS surfaces) — scoped to `crates/bumbledb-log/src`,
   `ts-log/src`, `examples/lambda/src`, and wire it into
   `spec-census.sh` with per-line attribution on failure. Adding a
   deletion to the tree without its roster line is an incomplete
   deletion.
2. Append the five rulings to `settlement/RULINGS.md`: the version byte
   stays 3; the theory file stays text; the lease counter is canonical
   decimal ASCII; a digest in memory is bytes and hex-in-memory was
   tried and deleted; the battery runs nextest with its config at the
   workspace root.
3. **Run the 141-row adversarial audit** of
   `settlement/90-traceability.md` — every row, no sampling,
   refutation-briefed verifiers fanned wide: two independent per
   critical (rows 0–9), one per major/minor; a row passes only when the
   refuter must cite the type or invariant that stopped them,
   `file:line`; any refutation reopens the owning work before the
   receipt can exist. Batch rows by owning decision so each verifier
   reads one canon section deeply. Emit the verdict table as a receipt
   artifact.
4. **Cut the docs back** (50 §4): amend `settlement/00-canon.md`
   §3/§4/§6 to state the landed one-encoding facts (binary v:3
   documents, the renamed keys, the `Vector` algebra, the binary/text
   boundary) and rewrite its closing "remaining delta" paragraph to
   point at this pass's receipt; then delete
   `settlement/10-endgame.md`, `settlement/20-one-encoding.md`, and
   `settlement/DISPATCH.md`, and shrink `settlement/README.md` to the
   law and the proof artifacts. One law, one open campaign, zero stale
   dispatches.

## L5 — Green once, then the receipt

Loop `scripts/battery.sh` to a clean exit — fix forward; never loosen.
Then the receipt commit: the battery transcript (script + commit
named), the census run clean, the 141 verdicts with citations, the
complete rulings ledger, the one version (`0.19.0`, roster proven
complete), this pass's deletion tally in house style, and the one line
the owner runs: `git push origin HEAD`.

## Autonomy protocol

Canon wins on law; the lockstep docs win on their five seams until
landed, then canon absorbs them. A spelling that cannot land → the
invariant block is the contract, the nearest airtight form lands, the
ruling is appended — logged substitutes only, questions never. Red
mid-flight is not a stop condition. A worker that stalls or produces
garbage is killed, reverted, and re-dispatched sharper; twice → split
the brief smaller and fan wider. Everything except the hard bounds is
yours to decide and log.

Begin now: fan out L0 recon, roll straight into L1, and do not stop for
anything short of a hard bound until the L5 receipt is on `main`.
