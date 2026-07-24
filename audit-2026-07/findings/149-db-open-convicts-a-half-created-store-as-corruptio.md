## Db::open convicts a half-created store as Corruption(MetaMissing) — the state Db::create names, tolerates, and the ephemeral probe classifies correctly

category: incoherence | severity: low | verdict: CONFIRMED | finder: r2:crash-recovery-lifecycle
outcome: fixed 9836d1ee (R18)

### Summary

A crash inside `Db::create` between LMDB env creation (which materializes `data.mdb` with a valid empty root) and `initialize`'s `_meta` commit leaves a half-created store. The codebase names this state explicitly and handles it in two of its three constructors: `initialize` tolerates it and proceeds (create heals the store), and the ephemeral probe classifies it as fresh with a three-way branch. But the durable open path (`verify_and_open`, reached from `Db::open`) collapses the entire no-`_meta` case to `Error::Corruption(CorruptionError::MetaMissing)` unconditionally — a corruption conviction for a store that holds zero data and is one `Db::create` call from healthy. The same function's own doc comment argues against exactly this kind of misnaming. The no-`_meta` classification is written three inconsistent ways across three sibling files in the same module.

### Evidence

All citations verified against the working tree.

- **The unconditional conviction** — `crates/bumbledb/src/storage/env/open.rs:56-59`:
  ```rust
  let meta: Database<Bytes, Bytes> = env
      .open_database(&wtxn, Some("_meta"))?
      .ok_or(Error::Corruption(CorruptionError::MetaMissing))?;
  ```
  No empty-root probe; half-created, foreign-LMDB, and genuinely torn stores all get the same corruption verdict. `Db::open` reaches this directly: `crates/bumbledb/src/api/db/open.rs:37` → `Environment::open` (open.rs:27-31) → `verify_and_open`.

- **The state is named and tolerated 40 lines away** — `crates/bumbledb/src/storage/env/create.rs:58-68`: "A half-created bumbledb store (crash between directory creation and the meta commit) has an empty root and still proceeds." `initialize` probes the unnamed root and only refuses when it is non-empty (foreign environment).

- **The correct three-way classification exists** — `crates/bumbledb/src/storage/env/ephemeral.rs:103-110`:
  ```rust
  let Some(meta) = env.open_database::<Bytes, Bytes>(&rtxn, Some("_meta"))? else {
      if let Some(root) = env.open_database::<Bytes, Bytes>(&rtxn, None)?
          && !root.is_empty(&rtxn)?
      {
          return Err(Error::AlreadyInitialized);
      }
      return Ok(false);   // half-created => treated as fresh, re-initialized
  };
  ```
  The probe's doc (ephemeral.rs:82-84) names `Ok(false)` as "a half-created store (empty root, no `_meta` — the crash window between directory creation and the meta commit)".

- **The doctrine the conviction violates, in the same function's doc** — open.rs:38-41: "convicting it of corruption for lacking `_data` would misname a merely-old store." The identical argument applies to a never-born store lacking `_meta`.

- **No test pins MetaMissing as intended here** — `crates/bumbledb/tests/api.rs:1071-1086` manufactures exactly this state (raw LMDB env, one committed-empty write txn) and asserts only that `Db::create` recovers it ("an empty root is recoverable"). Nothing exercises `Db::open` on that state, so the current diagnosis is untested behavior, not a pinned contract.

- **The spec does not sanction it** — `docs/architecture/70-api.md` § Environment lifecycle names the half-created case solely as create's recovery exception; open's specified failures are version/kind/fingerprint mismatches. `docs/architecture/50-storage.md` never rules that a never-initialized directory is corruption.

- **No typed alternative exists** — `crates/bumbledb/src/error.rs` has `MetaMissing` (line 44) and `AlreadyInitialized` (line 1194) but no NotInitialized/StoreMissing variant: the gap is representational.

### Failure scenario

Process killed inside `Db::create` between `open_env` (LMDB writes the initial meta pages, materializing `data.mdb` with a valid empty root — the exact state the in-repo test manufactures) and `initialize`'s `wtxn.commit()`. A supervisor restarts the app; typical "directory exists → open" logic calls `Db::open`, which returns `Error::Corruption(CorruptionError::MetaMissing)` — an operator-facing "your store is corrupt" for a store that was never born, holds zero data, and heals completely on one `Db::create` call. The recovery-path asymmetry is the incoherence: rerunning create heals; rerunning open misdiagnoses, and nothing in the error names create as the remedy.

### Suggested fix

Extract the ephemeral probe's no-`_meta` classification into one shared function and use it in all three constructors (`verify_and_open`, `probe_ephemeral_kind`, `initialize`): no `_meta` + non-empty root => `AlreadyInitialized` (foreign environment); no `_meta` + empty root => a new typed not-initialized error from `Db::open` (naming `Db::create` as the remedy), the proceed path from `initialize`, and `Ok(false)` from the probe. This is the representation-first move the project doctrine asks for: one classification as data, three callers, instead of the same branch hand-written three inconsistent ways.
