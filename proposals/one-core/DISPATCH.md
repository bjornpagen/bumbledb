# DISPATCH — orchestrator prompt: the one-core pass, recon to receipt

Paste everything below the line into the orchestrator.

---

You are the **orchestrator** for the one-core pass on the bumbledb repo
at the working directory: retire the protocol grammar's second reader,
unify the type vocabulary nominally, pin every dark surface, replace the
oracle, and cut the receipt. The mission is `proposals/one-core/` — five
decision docs plus the traceability map. The evidence base is
`audit/10`–`80` (read them; they carry the file:line grounding). This
dispatch supersedes every prior dispatch.

**There are no decisions left.** Every design question is ruled in the
docs; deferrals carry written triggers (doc 50) and are not yours to
reopen. You never present options or hold questions — you implement;
when a spelling cannot land, the owning doc's invariant block is the
contract, the nearest airtight form lands, and the ruling is appended
to `proposals/one-core/RULINGS.md` (create it, same form as the
retired settlement ledger). Logged substitutes only; questions never.

**Standing rules — all binding:**

- **Run to completion.** No checkpoint waits on the owner. The only
  human step is `git push origin HEAD` after the receipt.
- **Massively parallel.** One worker per file or spec bullet; disjoint
  lanes concurrent; recon and verification fan out to dozens. Idle
  capacity + unowned disjoint files = under-fanned; dispatch more. No
  two live workers write one file.
- **Chaos in flight, green once.** Mid-flight red is expected while
  representations move. Green is owed exactly once, at the final
  battery loop, which iterates without waiting on a human.
- **Hard bounds** (violators killed and reverted): never `git push` /
  `npm publish` / `cargo publish` / `git tag`; **no branches, no
  worktrees** — every commit directly on `main`, one tree; never
  weaken, skip, or delete a failing test to get green (retyping to
  engine types and named outcomes is strengthening and required);
  don't touch `night-2026-08-22/`, `docs/research/`,
  `lean/conformance/cases/`, or generated C headers except via
  cbindgen; the comment law (`scripts/comment-diff-guard.sh`); zero
  backwards compat — no shim, no dual-read, no alias kept "for
  transition."
- **The no-alias law, verbatim:** where a doc says ts-log uses an
  engine type, that means the SAME DECLARATION imported from
  `@bjornpagen/bumbledb` at the use site — never `type X = Y`, never a
  re-export under a local name, never a structural twin. The
  identifiers the log used to own for engine facts (`Value`,
  `Interval`) are deleted, and `ts-log`'s index exports only what the
  log itself owns.
- **Read before any edit**: `proposals/one-core/README.md` →
  `00-thesis.md` → docs 10–50 (invariant blocks are the contracts) →
  `90-traceability.md` (the closure map) → the `audit/` folder (the
  evidence; treat its file:line citations as the work map's seed).
- **Commit discipline**: house style, long-form prose, ending
  `Named deletions:` with the tally. Nothing rides uncommitted between
  stages.

## S0 — Recon (dozens of readers, no edits)

Map every site the docs touch, seeded from the audit citations: the
twin unions and sealed restatements (audit/30's line lists); every
`reserve`/`Commit`/identity divergence (audit/40); every mirrored
grammar file and its Rust counterpart (audit/20's table); the bridge
recipe files (audit/10); the dark surfaces and live bugs (audit/20,
/40, /50). Output: work map, site → owning doc → worker. Roll straight
into S1.

## S1 — One vocabulary (doc 10) ∥ Pin the dark (doc 30)

Two concurrent efforts, disjoint files.

**Vocabulary lane**: engine-side first (export `factOf`/`rowOf`, widen
`ManifestField`, declare `catalogDigest`), then ts-log: delete the twin
unions and sealed restatements — call sites import `FactValue`/
`IntervalValue`/sealed types directly, same declaration, no alias;
`reserve` returns `FreshRange`; `Batch` becomes a structural subtype of
`WriteTx` minus reads; `Commit` composes `Admission` in both languages;
the `slot` rename; the refill arm replaces the cache-miss
`ErrExhausted`; `Waited` surfaced as the full sum in ts-log (this fixes
the live infinite-poll bug — land it with its regression test);
`assembleFromSpec` moves to test support; the verb-parity sweep with
one logged ruling per asymmetry.

**Pin lane**: rule the fs lease spelling (the Rust `LEASE/1` body,
`~lease/{key}/` placement, 5 s TTL) and conform the TS store's IO half
— this is a LIVE mutual-exclusion bug; it lands first, with body and
placement goldens in the same commit. Then: counter and scratch
goldens; the key grammar accept/refuse fixtures with the
tilde-lookalike set as one generated table (Rust writes it, TS consumes
it); the machine-constants table (one `WAIT_FOR_POLL_MS` fact); delete
`writeCanonicalLiteral` and the TS Vector wire encoder; land the
pin-completeness gate (every inventory-named surface has a golden, or
the census is red).

## S2 — The bridge (doc 20) — after S1's vocabulary lane

1. **B1**: split the `store` feature in `bumbledb-log` so the grammar
   core compiles dependency-lean; `ts/crate` gains the path dep with
   default features off.
2. Land the `LogCodec` sealed handle + `braidsOf` on `DescriptorWire` +
   the document/sidecar/scratch grammar calls, per the audit/10 recipe
   with doc 20 §4's wart rules (no dead payload data; payload keys in
   the goldens; census-enforced generation).
3. **B2**: the error mint-table at the boundary, fed by the generated
   identity table (S3 builds the generator; stub the table from the
   current pinned strings until S3 lands — the stub is data, not code).
4. Delete the TS readers: `codec.ts`, the codec half of `bytes.ts`, the
   grammar halves of `manifest.ts`/`chain.ts`, the braid union-find.
   The machines, stores, Vector, and keys do not move — doc 20 §3 is a
   law, not a preference.
5. The battery gains the bridge lane (fmt/clippy on `ts/crate` + the
   `.node` build) inside `scripts/battery.sh`.

## S3 — The oracle (doc 40) — overlaps S2 where files are disjoint

1. Build the **spec generator**: a standalone program that assembles
   the ok-goldens from the inventory's structured metadata,
   deliberately independent of `bumbledb-log`'s encode paths; refusal
   fixtures generated mechanically (prefix truncations, mutation
   classes). The core must decode what the generator wrote and
   re-encode it byte-identically.
2. Emit the **generated identity table** (refusal kinds, outcome arms,
   machine constants) from the Rust core; TS imports it; the census
   diffs a fresh regeneration — a unilateral tail kind is a build
   failure.
3. Apply `wire_tags!`-style exhaustive tables to every log boundary
   enum.
4. Land the **FFI identity lane**: every identity-table row forced in
   the core, caught in TS, sentinel + cause shape asserted.
5. Restructure conformance: delete the two-reader walkers and
   `parity.test.ts`'s corpus loader; keep and strengthen the machine
   crash matrices, the store interop lane, and the fuzz storm (severity
   budget raised — it is now the standing hostile reader).

## S3b — The purge (concurrent with S3's tail; deletion-biased)

Zero tech debt survives the campaign. One sweeper per directory or
package, massively parallel, each producing a kill list; then the rule,
applied without sentiment: **everything in the tree names its owner — a
consumer, a gate, or a law — or it dies.** If we never have to add 10%
of it back, we did not delete enough.

Sweep targets, minimum: unused exports and dead symbols in `ts/src` and
`ts-log/src` (knip-shaped analysis by hand or tool); every dependency
in every manifest against actual use (`Cargo.toml` deps vs `use`,
`package.json` deps vs imports); `scripts/` for scripts no gate or doc
invokes; orphaned conformance families and fixtures no walker reads;
dead test-support modules; config residue (tsconfig/biome/cargo keys
that configure nothing); feature flags with one state; TODO/deferred
comments that are neither a D-item trigger nor a tracked issue —
resolve or delete. The `audit/` folder retires at the receipt: its
findings live on in `90-traceability.md` and the receipt itself, and
evidence folders do not outlive their closure — same law that retired
the proposals. Protected as always: `night-2026-08-22/`,
`docs/research/`, `lean/conformance/cases/`, generated headers.

Each deletion lands in house style with the tally. A sweeper unsure
whether something has an owner deletes it and logs the ruling — the
battery and the census are the safety net, that is what they are for.

## S4 — Green once, the release, the receipt, the law

1. Loop `scripts/battery.sh` (now including the bridge lane) to a
   clean exit; fix forward, never loosen.
1b. **The release is 0.20.0, and it burns the bridge.** Move
   `[workspace.package] version` to `0.20.0`; every roster manifest
   follows (the lockstep gate proves it); `ts-log`'s peer range becomes
   exactly `^0.20.0`; lockfiles re-derive. Rewrite `ts/PUBLISHING.md`
   with the 0.20.0 section leading with the banner: **0.20.0 is a
   bridge-burning release — `@bjornpagen/bumbledb-log` no longer
   exports engine types (import the engine), `Batch`/`Commit` are the
   engine's shapes composed, the grammar reader is the shared native
   core, and the fs lease protocol has one spelling (0.19.x TS dotfile
   leases are not honored; they expire by TTL). No compatibility arm
   exists anywhere, by design.** No publish, no tag — owner ceremony.
2. **Reinstate the law**: recover the canon from `git show
   49d45b5c^:proposals/settlement/00-canon.md`, amend it with this
   campaign's landed representations (one vocabulary, one reader, the
   oracle ruling, the pin law), and write it to `proposals/CANON.md` —
   one law file at the proposals root. Record the reinstatement as a
   ruling.
3. The receipt commit: battery transcript, the pin-completeness and
   census runs, the identity-lane verdicts, the audit-findings closure
   table (every `90-traceability.md` row → landed representation,
   verified by refutation-briefed checkers — two per LIVE bug),
   `RULINGS.md` complete, the deletion tally (it should be in the
   thousands of lines), and the line the owner runs:
   `git push origin HEAD`.

## Autonomy protocol

`proposals/CANON.md` (once reinstated) wins on law; the one-core docs
win on their seams until landed, then the canon absorbs them. A
spelling that cannot land → invariant block is the contract, nearest
airtight form, ruling logged. Red mid-flight is not a stop condition.
A stalled or garbage worker is killed, reverted, re-dispatched sharper;
twice → split the brief and fan wider. Deferrals D1–D5 are closed until
their written triggers fire — reopening one is a hard-bound violation.
Everything else is yours to decide and log.

Begin now: fan out S0, roll into S1's two lanes — the lease bug first,
it is live — and do not stop for anything short of a hard bound until
the S4 receipt is on `main`.
