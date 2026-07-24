## fingerprint_lock TempDir uses a fixed path — concurrent test runs race remove_dir_all against a live locked store

category: bug | severity: low | verdict: CONFIRMED | finder: r2:concurrency-unsafe-ffi

### Summary

The napi bridge crate's hand-rolled `TempDir` in `ts/crate/src/fingerprint_lock.rs` derives its directory as `temp_dir()/"bumbledb-node-{tag}"` with a constant tag (`"fingerprint-lock"`) and begins by `remove_dir_all`-ing whatever is at that path. Two concurrent invocations of the crate's test binary collide on the same absolute path: the second run deletes the first run's live LMDB store out from under its open environment (on unix, unlinking open files succeeds), turning the cross-host fingerprint-lock roundtrip test into flaky ENOENT/corruption failures. The engine crate's own internal helper already carries the fix — a pid suffix — with a doc comment naming this exact hazard; the ts copy dropped it.

### Evidence

- `ts/crate/src/fingerprint_lock.rs:100-106` — the fixed-path constructor:
  ```rust
  fn new(tag: &str) -> Self {
      let path = std::env::temp_dir().join(format!("bumbledb-node-{tag}"));
      let _ = std::fs::remove_dir_all(&path);
      std::fs::create_dir_all(&path).expect("create test dir");
      Self(path)
  }
  ```
- `ts/crate/src/fingerprint_lock.rs:130` — the single call site uses the constant tag: `TempDir::new("fingerprint-lock")`.
- `ts/crate/src/fingerprint_lock.rs:134-146` — the test performs multiple sequential `Db::create`/`Db::open` cycles against that path (descriptor create, macro-twin open, descriptor reopen, twisted-twin refusal), each a fresh open of the on-disk store, so a mid-test wipe by another process lands between any two of them.
- `crates/bumbledb/src/lib.rs:292-298` — the engine's internal `testutil::TempDir` proves the project already recognizes this hazard and its fix:
  ```rust
  /// ... the pid suffix keeps concurrent suite runs (other worktrees,
  /// co-tenant agents) from wiping each other's dirs.
  pub fn new(tag: &str) -> Self {
      let path =
          std::env::temp_dir().join(format!("bumbledb-test-{tag}-{}", std::process::id()));
  ```
- Related latent instance (correction to the original finding's framing): the engine's integration-test twin `crates/bumbledb/tests/common/mod.rs:15-20` (`bumbledb-it-{tag}`) also lacks the pid suffix — its safety rests only on per-test-function tag uniqueness within one repo checkout, not across concurrent suite runs. The pid-suffixed helper is the in-crate `testutil` one, out of reach of external test binaries.

### Failure scenario

Runner A executes `the_bridge_typestate_and_the_macro_twin_open_each_other_s_stores` and has the LMDB env open at `$TMPDIR/bumbledb-node-fingerprint-lock`. Runner B (a co-tenant agent's `cargo test`, or CI racing a dev run on the shared machine — the exact populations the engine's own doc comment names) starts the same test: its `remove_dir_all` unlinks A's `data.mdb` and lock file, then recreates the dir and `Db::create`s its own store. A's next `Db::open` on the recreated path observes a foreign or missing store and fails — a spurious failure in precisely the test whose job is a deterministic cross-host pin, misread as fingerprint drift. Additionally, A's `Drop` then wipes B's live store, cascading the flake.

### Suggested fix

Fold process uniqueness into the path, matching the engine's own `testutil::TempDir` (one line, still dependency-free):

```rust
let path = std::env::temp_dir().join(format!("bumbledb-node-{tag}-{}", std::process::id()));
```

Consider applying the same pid suffix to the integration twin at `crates/bumbledb/tests/common/mod.rs:16`, which has the identical cross-process weakness.
