# PRD 14 — fuzz target: crash (durability under torn commits)

**Depends on:** 12 (the ops runner is the substrate — crash is ops with
a kill switch).
**Modules:** `fuzz/fuzz_targets/crash.rs` + runner;
`crates/bumbledb/src` gains a `crashpoint` FEATURE (cfg-gated, zero
cost when off — the `chase-off` precedent) with hooks at the commit
pipeline's phase boundaries.
**Authority:** the durability claim is currently inherited from LMDB's
single-`mdb_txn_commit` atomicity plus our write ordering — believed,
never adversarially exercised. A database asking to be dogfooded proves
its crash story. SQLite's torn-write testing is the model, scaled to our
architecture: we don't fault-inject the filesystem (LMDB owns that
layer); we kill OURSELVES between logical phases and prove every
observable outcome is all-or-nothing.
**Representation move:** commit-phase boundaries become NAMED (the hook
points), which is documentation with teeth: the set of crashpoints IS
the claimed atomicity structure, reviewable in one grep.

## Context (decided shape)

- **The `crashpoint` feature:** when on, each named point in the commit
  pipeline (after staging, mid-namespace-write per F/M/U/R/S family,
  before judgment, after judgment/before `mdb_txn_commit`, after
  commit/before memo update, after memo update) consults
  `BUMBLEDB_CRASHPOINT=<name>` and, on match, `std::process::abort()` —
  a real unclean death, not a panic (no unwinding cleanup allowed to
  tidy up).
- **The runner** (parent process): generate an ops prefix + one victim
  commit from fuzzer bytes; spawn a child (the same binary re-entered,
  env-var-steered — the libFuzzer-compatible fork pattern) that runs
  the prefix then the victim commit with a drawn crashpoint armed; child
  aborts; parent then:
  1. reopens the store — must succeed (no wedged env);
  2. `verify_store` — must pass;
  3. full-content compare against the naive model at the PREFIX state
     (crashpoint before `mdb_txn_commit`) or the POST-COMMIT state
     (crashpoint after) — the all-or-nothing oracle; the boundary
     crashpoint's expected side recorded per point in a table in the
     runner;
  4. re-run the victim commit on the reopened store — must succeed and
     land the post-state (recovery is complete, not merely clean).
- **Off means off:** with the feature disabled the hooks compile to
  nothing; the default build's asm gates and benches are unaffected
  (grep-provable: the hook macro expands empty without the feature).

## Technical direction

1. Land the feature + hooks first with a table in the commit module
   naming each point and its expected recovery side; the hook is one
   macro (`crashpoint!("after-judgment")`) so the census is
   `grep -rn "crashpoint!"`.
2. The child-process plumbing lives in the fuzz harness (spawn self
   with env), NOT in the engine.
3. Every crashpoint × a small ops-prefix matrix must be hit at least
   once by a deterministic unit sweep in the fuzz crate (not left to
   fuzzer luck) — the fuzzer then explores prefixes around them.
4. Smoke: the deterministic sweep green + 10k fuzz runs (child spawns
   are expensive; lower budget than the in-process targets, recorded).

## Passing criteria

- `[shape]` `grep -rn "crashpoint!" crates/bumbledb/src` enumerates the
  commit pipeline's phase structure — every phase boundary named, the
  table in the commit module matches the grep exactly.
- `[shape]` Feature off → hooks expand to nothing; asm gates pass
  untouched; the feature is not in default features.
- `[test]` The deterministic crashpoint sweep green: every point ×
  matrix cell recovers per its expected side, `verify_store` green,
  victim-commit replay lands.
- `[test]` 10k-run fuzz smoke finding-free (or trophies fixed +
  recorded).
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

The storage chapter gains the crashpoint table (the named atomicity
structure) and the recovery claim it proves; the fuzzing charter gains
the crash target's line.

## Results (2026-07-13)

**Landed.** The `crashpoint` feature (`crates/bumbledb/Cargo.toml`, not
in default features), the hook macro + table + `crashpoint_hit` in
`storage/commit.rs`, ten call sites — `grep -rn "crashpoint!"
crates/bumbledb/src` returns exactly the table's rows: `after-staging`,
`before-judgment`, `after-judgment`, `after-commit`, `mid-write-s`
(`commit/write.rs`), `mid-write-m/f/u/r` (`commit/applier.rs`,
insert path), `after-memo-update` (`api/db/write.rs`). Off, the macro
expands to nothing; the table is `pub` (`bumbledb::CRASHPOINTS`) so the
harness consumes the engine's own claim. Generation lives in
`corpus_gen::opgen` (`random_crash_scenario`, `crash_matrix_scenario` —
whole-world-replacement victims, pinned accepted + state-changing
against the naive model in co-located tests); the runner in
`fuzz/src/crash.rs`, the sweep + corpus replay in `fuzz/tests/crash.rs`.

- **Feature unification, noted as directed:** `fuzz/Cargo.toml` enables
  `crashpoint` for the ONE bumbledb build every target shares (the
  chase-off/fold-off precedent). Safe: the hooks are inert without
  `BUMBLEDB_CRASHPOINT`, which only the crash child ever sets — and only
  between its prefix and its victim commit (prefix commits never trip).
- **Child re-entry mechanics, recorded:** under `cargo fuzz` the child
  is the fuzz binary in single-input mode with `-handle_abrt=0` (raw
  death, no child artifacts); under `cargo test` it is the ignored
  `crash_child` test body (the `crates/bumbledb/tests/crash.rs`
  precedent). One wrinkle found and fixed in the harness: libFuzzer
  tests the callback with an EMPTY input once at startup, so the child
  reads its bytes from an env-pointed file under a once-guard instead of
  trusting `data` (otherwise the probe pre-creates the store and the
  real pass dies `AlreadyInitialized`). Classification is the marker
  line the hook prints before aborting — an abort without it (a child
  panic) is a finding.
- **Deterministic sweep:** all 10 crashpoints × 3 prefix cells (first
  commit ever; one committed world; two commits deep), every combination
  REQUIRED to abort (clean exit fails the sweep) and recover per its
  side, plus victim replay — green, ~3 s under plain `cargo test`.
- **One semantics ruling, recorded (not a bug):** a prefix-side death
  during the very first commit recovers to the EMPTY store, which
  `verify_store` flags for unsatisfied domain quantifications by design
  (closed-source statements hold only once their backings land — the
  naive model documents the same division of authority). The oracle
  compares that case's findings against a fresh store's, exactly;
  every non-empty expected state asserts plain green.
- **Smoke:** `cargo fuzz run crash -- -runs=10000 -fork=2` — 10,213
  iterations, finding-free (0 oom/timeout/crash, exit 0), ~9–10 exec/s
  aggregate (child spawn + LMDB-fsync commit rounds per iteration
  dominate; contended with concurrent fuzz sessions), 1393 s ≈ 23 min
  wall. The recorded rate is the budget's justification: crash stays a
  lower-`-runs` target than the in-process lanes.
- **Zero default-build impact:** `scripts/check-asm.sh` on a fresh
  default-features release build — all gates green, no new failures
  (the pre-recorded `position_matches` false positive did not
  reproduce on this build; zero failures total). `cargo test -p
  bumbledb` (default features) green; fmt + `clippy --workspace
  --all-targets -- -D warnings` green.
