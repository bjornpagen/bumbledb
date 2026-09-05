# Final qualification checklist (permanent)

This is the remaining qualification checklist after permanent contracts
live here. The disposable `final-solution/` packet and root `PROMPT.md`
are **not** inputs to this checklist once the coordinator retires them.

The earlier “review → checks → retire → commit” sequence is withdrawn.
Retirement can change qualified inputs. Capture the candidate **after**
retirement, then run required checks against that candidate.

Verification of this tree: **NotRun**. Do not populate
[release-results.json](release-results.json) until real cells execute.
A successful `scripts/battery.sh` exit is not all-platform qualification.

## Source barrier (coordinator, before product checks)

1. Every mandatory producer, consumer and deletion in L01–L21 is
   implemented; no “future tightening” exception.
2. Coordinator has reviewed each lane’s source and all seven integration
   journeys in the (then already retired) chapter 60 record, preserved in
   coordinator state if needed.
3. Public Rust/TS syntax, native declarations/exports, internal
   cross-package imports, formats, proof premises and examples agree.
4. Workers are quiesced before measurement.
5. These permanent pages carry every needed semantic/API/performance
   contract, D01–D29, 68 audit IDs, 220 child behaviors and 78
   prior-review obligations. Executable inventory no longer derives from
   disposable Markdown.

## Retirement, then candidate, then checks

Coordinator-owned order — L21 does not delete the packet or commit:

1. Confirm no executable or doc link still depends on disposable packet
   chapters. Inventory `generatedFrom` lists only `docs/reference/**`.
2. Preserve remaining coordinator state outside the repo if resumption
   needs it.
3. Retire **only** `final-solution/` and root `PROMPT.md`. Preserve
   `audit/` permanently.
4. Capture the **post-retirement** candidate:

   ```sh
   node scripts/release-results.mjs --candidate-digest
   node scripts/release-results.mjs --specification-revision
   node scripts/release-results.mjs --inventory
   ```

5. Stage tarballs with matching `pack-provenance.json`:

   ```sh
   node ts/scripts/stage.ts --out <dir>
   node ts-log/scripts/stage.ts --out <dir>
   ```

6. Execute the runner in [release-gates.md](release-gates.md) order
   against that candidate. Repair failures without restoring shims.
   Rerun all evidence affected by changed inputs.
7. Populate `docs/reference/release-results.json` only from actual
   validated results. Never create a placeholder Passed manifest.
8. `node scripts/release-results.mjs pre-promotion docs/reference/release-results.json <exact-candidate-digest>`
9. Review staged inputs, make the sole final integrated commit, verify
   the committed tree maps to the same qualified inputs, then push the
   authorized existing branch.

`PKG-07B` stays pending until separately authorized publication. Missing
real S3/IAM/Graviton or other advertised evidence remains **NotRun**.

## Exact commands by owner

### Coordinator

- After all source obligations and seven journeys: retire packet and
  root `PROMPT.md`; then run the capture/qualify/commit sequence above.
- Do not treat nonempty evidence arrays or battery exit as qualification.
- One final commit/push after verified candidate equality. No package
  publication or live-tenant mutation from this checklist.
- Drop duplicate package-test rebuilds in coordinator-owned manifests:
  `ts/package.json` and `ts-log/package.json` `test` scripts still run
  `pnpm run build` before `node --test`. Battery and CI already perform
  exactly one current-addon build; those hooks should become
  `node --test 'test/**/*.test.ts'` only.

### L05-delivery — requested (L21 does not edit `gates.rs`)

Replace these claim-mismatch tests in
`crates/bumbledb/src/api/prepared/tests/gates.rs`. Keep the D08/D09
ids. L21 packed-consumer will not grow a tiny keyed query or
`WorkUnits > 0` stand-in:

- `d08_successful_execute_retains_work_charges` — retained COLT /
  working / scratch pool must stay charged after a production execute
  that actually grew capacity. `used(WorkUnits) > 0` is not that gate.
- `d09_fallback_opens_nonresident_text_and_agrees` and
  `d09_spill_opens_via_exhausted` — D09 is derived-pipeline
  boundedness (aggregate → join → negation → recursion, RAM below
  intermediate cardinality, compare with `staged.rs`). Not a tiny
  fallback query plus `store.resolve`.

### L18 — packaging / packed consumers

- Keep Notes, native-ledger, core-ts, log-ts and Rust consumers on the
  public API in [api.md](api.md). No private imports or handwritten
  plans. Specimens do not self-provide `NativeRuntime.layer`.
- `scripts/packed-import.sh` installs staged tarballs outside the
  workspace, typechecks those consumers, and runs them under
  `ManagedRuntime.make(NativeRuntime.layer(...))`. D07 tiny collect
  must fail; D12 same-cursor retry must still deliver. `typeof` /
  `type_name` / `size_of` are not gates. D27 addon-unavailable pure
  import (`scripts/packed-pure-authoring.ts`) is a separate cell. Rust
  consumer and Notes `specimens.test` + `routes.test` run in this
  path; missing migrations fail, never skip green.

### L19 — refinement integration

Permanent scope (L19 authored; L21 binds qualification):
`lean/Bumbledb/Bridge.lean`, `lean/proof-bridge-ledger.md`,
`lean/correspondence.md`, `scripts/lean.sh`, `scripts/spec-census.sh`.

- `scripts/battery.sh` calls `scripts/lean.sh` (kernel + conformance +
  constructor census). `lean.sh` is not a cargo-test owner. No
  exact-`dyn` counts, wording bans, or deleted-path census. Do not
  restore a scraper of `final-solution/`.
- Gates: G03/G04/G07 and D04/D05/D19/D26. Independent oracles are
  `judge_final_state`, `crates/bumbledb-bench/src/naive/successor/staged.rs`,
  and `crates/bumbledb-bench/src/closure/history_model.rs` — not the
  production planner.
- Identity/surface goldens: `python3 scripts/spec-gen.py --check`.
  Census no longer runs them.

### L20 — measurement integration

L21 ingested `appperf::plan::{render,l21_semantic_checks,script_steps,hardware_prerequisites}`.
Meanings stay in [performance.md](performance.md). Do **not** start G15
until writers are frozen and the post-retirement candidate exists.
This lane is not Integrated.

- Daily (after source barrier, still not G15): `verify-oracle`,
  `scorecard-semantic` (`bumbledb-bench app-perf --plan`),
  `storage-census`, `hash-blake3`.
- G15 timing, quiet host only, via `scripts/measure.sh` lock:
  `app-perf-warm`, `app-perf-cold`, `app-perf-tenants`;
  `large-populated` on real Graviton with >40 GiB allocated blocks;
  `hosted-decision` on real S3/IAM (emulator cannot close G15).
- Optional: `hash-aegis-optional` — missing KAT is NotRun.
- Evidence must name the 13 scorecard cell ids and the
  `l21_semantic_checks` rows, including the seven
  `correspondence::OWNED_CASES` cargo tests in `bumbledb-bench`:
  `C-D04-collision-bytes`, `C-D19-cancel`, `C-D19-mean-once`,
  `C-D19-merge-not-idemp`, `C-G03-mutable-support`, `C-G03-add-wins`,
  `C-G03-raw-commute`. Those tests live in the bench crate, not
  `scripts/lean.sh`. Oracle is `judge_final_state` / the rational
  fold — not the production planner.
- Battery/check renderer selftests are not G15.
- Missing `apple-silicon-macos-arm64`, `graviton-linux-arm64`,
  `linux-x86-64-node`, `real-s3-iam`, or `large-populated-disk`: exact
  cell stays **NotRun**.

## Required qualification cells

From [obligation-inventory.json](obligation-inventory.json):

| Cell | Role |
| --- | --- |
| `check-macos-arm64` | Everyday/static/fault battery on Apple Silicon |
| `check-linux-x64-al2023` | Same battery on Amazon Linux 2023 x86-64 |
| `sdk-macos-arm64` | Native/SDK/declaration/fingerprint lane |
| `sdk-linux-x64-al2023` | Same SDK lane on AL2023 x86-64 |
| `miri-macos-arm64` | Supported unsafe/Miri components only |
| `packed-import-empty-project` | Fresh staged tarballs; ManagedRuntime consumers; D07 fail; D27 addon-unavailable; Rust + Notes |
| `real-s3-iam` | Actual S3/IAM; mocks cannot qualify |
| `graviton-arm64-runtime` | Real Graviton Linux ARM64 runtime/ABI |

Unsupported substrates are documented rather than pretending Miri
executes S3/LMDB. A container on x86 does not qualify ARM.

## Candidate identity

`scripts/release-results.mjs` recomputes membership, path, kind, mode,
bytes or symlink target. Link identity hashes the link target, not
dereferenced content. Deleted tracked inputs frame as `kind=deleted`
without crashing enumeration. Permanent contracts are inputs.
`docs/reference/release-results.json` is a declared non-input exclusion.
No caller-supplied path list, digest override or subset can omit
production inputs. Changed source invalidates affected evidence.
Stage/commit must match the qualified candidate.
