# bumbledb — instructions for Claude

## Governing design document (MUST follow)

Read and follow [docs/design/representation-first.md](docs/design/representation-first.md)
before changing code. Its rule: **the data representation is the main lever, not the
control flow.** When a new case shows up, change the types, data, and invariants so the case
stops being special or cannot be expressed. Add branches, flags, or guards only for essential
differences. The bumbledb-specific rules are at the end of that document.

## Working rules

- **Hard cutover; cull all legacy.** bumbledb and bumbledb-log have no users. On-disk
  formats, wire formats, and the Rust/TS APIs may break freely. Old formats are thrown away
  with zero backward compatibility. Do not add readers, refusals, migrators, or detectors
  for prior layouts. Do not keep compat aliases. Do not keep code or comments that narrate
  history ("successor", "retired", "transitional", "0.x", ticket IDs). One format per
  artifact: the current one, guarded by a single magic/version equality check.
- **Canonical extension is `bdb` (D20).** Every file, format, and artifact name that used
  "bumbledb" as an extension or namespace uses `bdb`, everywhere (code, tests, docs, notes,
  scripts):
  - database directories and checkpoint images are `<name>.bdb`; the notes data dir is `.bdb/`;
  - the lock file is `bdb.lock`;
  - format family tags are `bdb.<kind>.v1` (e.g. `bdb.result.v1`, `bdb.evidence.v1`, every log
    frame);
  - the native addon file is `bdb.<platform>.node` / `bdb.node`;
  - TS brands and symbols are `bdb.*`.

  Crate names, npm package names, the `bumbledb` CLI, and env var names stay `bumbledb`.
- **Nightly Rust is mandatory.** Never move to stable. Bump the nightly pin and every
  dependency all the way to the latest versions.
- **Work directly on `main`.** No worktrees, no feature branches. Do not run adversarial
  review or verification agent waves. Use the repo's own tests and oracles as the gate.
- **The active plan** is [docs/cutover-plan.md](docs/cutover-plan.md).

## Comments (strict)

Source carries **no history and no process**. A comment earns its place only by stating
something the code and types cannot.

- **Keep:**
  - doc comments that state a public item's contract in 1–3 sentences;
  - a complete `// SAFETY:` comment on every `unsafe` block;
  - the *why* behind a non-obvious invariant, unit, or performance choice;
  - algorithm citations (a paper or URL).
- **Delete:**
  - history and narration ("successor", "retired", "previously", "now", "cutover", "0.x");
  - process and ticket IDs (chapter N, C07, P12, D27, G06, F3, ENG-/CORE-/HASH-, "owner
    ruling", "verdict", "re-earn", "dividend");
  - comments that restate the code or a name;
  - truncated fragments and `//! .` remnants;
  - empty or boilerplate `# Errors` / `# Panics` sections;
  - commented-out code and banners;
  - module essays (compress to at most 5 lines: what the module owns plus its invariants).
- **Prefer the type over the comment.** If a comment explains an invariant, first try to encode it
  in a type (design rules R2/R3).
- The same rules apply to TS (JSDoc only on exported API), TOML, YAML, and shell.

## Tests (strict)

No flaky or brittle tests in the suite. A test must be deterministic: no wall-clock sleeps or
timing races, no machine-speed-dependent rosters, no scanning cargo's internal build-dir layout,
no exact allocator-layout transcriptions, and no "returns early when unconfigured = PASS". Delete
a flaky test rather than tolerating or retrying it, then cover the behavior deterministically if
it matters. Timing measurements are non-asserting bench reports, never `cargo test` gates.

## Swarm protocol (parallel agents on the one `main` tree)

- **Ownership.** You edit only the paths you were assigned. If something outside them blocks
  you, stop and report it; do not fix it.
- **Build and test** only through the sandbox, which runs committed HEAD plus your owned paths
  so other agents' uncommitted edits cannot interfere:
  `scripts/swarm/sandbox.sh <label> <owned paths…> -- cargo nextest run -p <crate>`.
- **Commit** each plan item separately, after its sandbox gate passes, with
  `scripts/swarm/commit.sh "<ID>: <summary>" <owned paths…>`.
  - Never `git add -A`, `git commit -a`, `stash`, `reset`, `checkout`, or `restore` on
    paths you do not own.
  - Never commit `Cargo.lock`; the coordinator owns it.
  - Never push.
- **Dependency changes:** edit your crate's `Cargo.toml` and report the change. The
  coordinator regenerates and commits `Cargo.lock` at the wave gate.
