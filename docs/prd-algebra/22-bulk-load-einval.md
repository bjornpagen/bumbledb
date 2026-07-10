# PRD 22 — The bulk-load EINVAL: kill the flake, type the boundary

**Depends on:** nothing (independent bug unit; may run any time, and early —
it is the only known flake in the gate suite).
**Modules:** `crates/bumbledb/src/storage/env/` (the commit/fsync boundary),
`crates/bumbledb/src/api/db/write.rs` (`bulk_load` chunk loop),
`crates/bumbledb/src/error.rs` (typing the boundary),
`crates/bumbledb-bench/src/corpus.rs` + `driver/tests.rs` (the observing test).
**Authority:** `50-storage.md` (durability: fsync per commit; typed corruption
doctrine), `00-product.md` (fsync-per-commit is the durability contract — any
fix that weakens it is refused before it is proposed).
**Representation move:** the trust-boundary rule, applied to the OS. An errno
crossing the commit boundary is *parsed once into a typed fact* (which call
failed, doing what, retryable or fatal) or made impossible — never passed
through as a stringly `Lmdb(Io(...))` that a test can only describe as
"flaky." An error type that cannot name its mechanism is the null of error
handling: it forces every caller to guess.

## The observation (2026-07-10, verbatim facts)

- **Test:** `driver::tests::bench_refuses_without_a_stamp`
  (`crates/bumbledb-bench/src/driver/tests.rs:81`), failing inside
  `ensure_corpus` → `load_bumbledb`.
- **Failure:** panic
  `load bumbledb: BulkLoad { committed: 65536, error: Lmdb(Io(Os { code: 22, kind: InvalidInput, message: "Invalid argument" })) }`.
- **Conditions:** `cargo test -p bumbledb-bench` (debug, default parallel test
  threads) **alongside a concurrent cargo build**; passed in the subsequent
  `cargo test --workspace` and in an isolated rerun. Seen once.
- **Shape:** `committed: 65536` = exactly 16 × `BULK_CHUNK` (4096) — the
  failure is at a chunk-commit boundary, i.e. inside LMDB's commit path
  (write/sync), not in fact encoding or judgment.

## Hypotheses, pre-triaged (record of the dead end included)

- **Eliminated already:** `mdb_env_set_mapsize` racing active readers — the
  engine sets `MAP_SIZE` once at open (`storage/env.rs:39`,
  `open_env.rs:21`) and never calls set_mapsize; no resize exists to race.
  Do not re-investigate.
- **Prime suspect:** the macOS fsync path. LMDB's commit on `__APPLE__` issues
  `fcntl(fd, F_FULLFSYNC)` (`MDB_FDATASYNC`); `F_FULLFSYNC` is documented to
  fail on filesystems that don't support it and has been observed returning
  transient errors under I/O pressure on others. The test corpus lives under a
  temp directory — confirm which filesystem/volume the harness actually uses
  and whether `F_FULLFSYNC` on it can EINVAL under load.
- **Second suspect:** any other `pwrite`/`msync`/`ftruncate` in the LMDB commit
  path returning EINVAL under concurrent-process I/O contention.

## Technical direction

1. **Reproduce before fixing.** A stress harness (test-only, not shipped):
   loop the S-scale-ish bulk load N times against a temp store while saturating
   the machine with concurrent I/O/CPU (spawned compile-like load or dd loops);
   instrument the errno site — run with an `strace`-equivalent
   (`dtruss`/`fs_usage` on macOS) or an LMDB build with the failing call
   logged — until the exact syscall is identified. If 500 iterations under
   worst-case contention cannot reproduce it, say so in the PRD's landing
   commit and proceed on the prime suspect's evidence (the fcntl semantics are
   documentable without the repro).
2. **Fix by mechanism, not by retry-blindly:**
   - If `F_FULLFSYNC` on the *test* volume is the cause: the durability
     contract binds real stores, not throwaway test corpora — but the fix must
     not fork the engine into sync modes (refused: modes). The honest shapes,
     pick by evidence: (a) if the volume genuinely doesn't support
     `F_FULLFSYNC`, LMDB's own fallback behavior is the question — surface it
     as a typed, *named* open-time capability check (probe once at
     `Db::create`/`open`, fail loudly with a typed error naming the volume's
     capability, so the failure moves from a mid-load flake to a deterministic
     boundary rejection); (b) if it is a transient EINVAL under pressure on a
     supporting volume, a bounded retry at the commit boundary is legitimate
     *only* with the mechanism documented and the retry observable (obs event),
     never silent.
   - If another syscall: same rule — type it, name it, decide retry-vs-fail on
     its documented semantics.
3. **Type the boundary regardless of root cause:** `BulkLoadError` (and the
   commit error path generally) gains enough structure that this failure, if it
   ever recurs, names its syscall and phase — extend the LMDB error mapping at
   the conversion site (`error/convert.rs`) rather than sprinkling context.
   The test's panic message should have made this PRD writable without a
   follow-up interrogation; after this PRD, it would.
4. **The observing test stops being load-sensitive:** whatever the mechanism,
   `ensure_corpus`-style test loads must either be immune (fix landed) or the
   test documents its isolation requirement structurally (serial-test-style
   gating is a last resort and needs the mechanism named to justify it —
   naked `#[ignore]` or blind retries are refused).

## Passing criteria

- `[shape]` The dead-end hypothesis (set_mapsize) is recorded at the fix site
  or in the landing commit; no resize-related code was touched.
- `[shape]` The errno's crossing point is typed: the failure mode, if
  provokable, produces an error naming mechanism and phase — no bare
  `Lmdb(Io(EINVAL))` reaches `BulkLoadError` for this class.
- `[test]` The stress harness exists (test-only), runs N configurable
  iterations, and passes at N ≥ 100 under synthetic contention on the fix; the
  landing commit reports the repro outcome honestly (reproduced-and-fixed, or
  not-reproducible-with-evidence and fixed-on-documented-semantics).
- `[test]` If the fix is the open-time capability probe: a test asserts the
  typed rejection shape (mockable or volume-dependent — skip-with-reason if
  the dev machine cannot express an unsupported volume, and say so).
- `[gate]` fmt/clippy/test workspace green; `scripts/check.sh` green; the
  previously-flaky test passes under `cargo test -p bumbledb-bench` with
  default parallelism alongside a concurrent build (best-effort demonstration
  recorded in the landing commit).
