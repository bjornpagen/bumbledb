## A stale `ephemeral.dirty` marker makes `Db::ephemeral` silently destroy a committed DURABLE store

bug | high | CONFIRMED | storage-v7
outcome: fixed 15342f00

### Summary

The R18 crash-wipe in `Environment::ephemeral` deletes `data.mdb`/`lock.mdb` whenever `<dir>/ephemeral.dirty` exists — before `probe_ephemeral_kind` ever reads the store's kind byte — and then skips the probe entirely. The marker can be orphaned over a store it was never minted for, because no durable constructor consults or clears it. A durable store born at a path carrying an orphaned marker is destroyed wholesale by the next `Db::ephemeral` call; the documented `StoreKindMismatch` refusal is unreachable. Reproduced empirically: `Db::ephemeral` succeeded over a committed durable store and re-initialized it empty (`data.mdb` 147456 → 65536 bytes), with zero error.

### Evidence (verified against v0.9.0 on branch `bugbash-perf`)

- `crates/bumbledb/src/storage/env/ephemeral.rs:64-74` — `let crashed = marker.try_exists()?; if crashed { remove data.mdb, lock.mdb }` runs immediately after `acquire_lock`, before any kind check. Line 81: `let has_meta = if !crashed && path.join("data.mdb").try_exists()? { Self::probe_ephemeral_kind(...)? } else { false };` — a wipe forces `has_meta = false`, so line 97 re-initializes an empty ephemeral store. The kind probe (line 132, where `StoreKindMismatch` lives at lines 140-145) never runs on the `crashed` path.
- The marker is read nowhere else. `crates/bumbledb/src/storage/env/create.rs:106`, `open.rs:106`, `exhume.rs:92` all set `dirty_marker: None`; none reads `dirty_marker_path`. `env.rs:266-278` (Drop) early-returns when `dirty_marker` is `None`, so no durable handle's close clears an orphan either. The comment at `env.rs:250` ("`Some` only on an ephemeral environment") and at `ephemeral.rs:60-63` ("only ephemeral opens mint one, and only a proven-synced close clears it — so the wipe destroys nothing the kind promised to keep") state an invariant — marker ⇒ this-store-is-ephemeral — that nothing maintains across the store's death and rebirth.
- The orphaning window is crash-shaped and real: `ephemeral.rs:91-92` creates and fsyncs the marker (plus dirent chain) BEFORE `open_env`/`initialize` at lines 93-97. A SIGKILL/power loss — or any error return from those lines, which perform no marker cleanup — leaves the marker with no committed store.
- The repurpose step succeeds: `create.rs:71-73` — `classify_meta_block` on the marker-only (or half-created) directory yields `MetaBlock::HalfCreated`, which `initialize` deliberately heals ("creation heals it") into a DURABLE store. The marker sits beside it, unexamined, for the store's entire durable life (confirmed by test: marker still present after durable create + commit + clean close).
- Empirical reproduction (scratch integration test, since deleted): plant `ephemeral.dirty` in an empty dir → `Db::create(dir, Staging)` succeeds, commit a fact, close → `Db::ephemeral(dir, Staging)` returns `Ok`, `data.mdb` drops from 147456 bytes to 65536 bytes (a fresh empty ephemeral store). No `StoreKindMismatch`; the durable store is gone silently.
- Contracts contradicted verbatim: `ephemeral.rs:29` "REFUSAL NEVER MUTATES"; `api/db/open.rs:28` "Production open never destroys data" and `:57-58` "a mistaken fresh store at a typo'd path destroys nothing durable"; `docs/architecture/50-storage.md:587` and `docs/architecture/70-api.md:431` "never destroys data it promised to keep".

### Failure scenario / impact

1. `Db::ephemeral(p, S)` on a fresh dir dies (SIGKILL, power loss, or any error) after the marker fsync at `ephemeral.rs:91-92` and before `initialize` commits. Disk: `ephemeral.dirty` + `bumbledb.lock`, no (or a half-created) `data.mdb`.
2. The path is repurposed: `Db::create(p, T)` succeeds — `HalfCreated` heals, the marker is ignored — and the application commits durable, fsynced facts for weeks.
3. Any later `Db::ephemeral(p, _)` — a typo, a config revert, the original staging job restarting — hits `crashed = true`, deletes `data.mdb` at `ephemeral.rs:68`, and re-initializes an empty ephemeral store. The durable store is destroyed with zero error; the expected `StoreKindMismatch` never fires because the wipe precedes (and suppresses) the probe.

Severity is high: silent, unrecoverable destruction of durable data through the public API, in direct contradiction of four in-tree laws.

### Suggested fix

Restore the marker's claim ("only ephemeral opens mint one, and it describes THIS store") by making durable births/opens erase orphans: after `Environment::initialize` / `verify_and_open` succeeds under `OpenLane::Write(StoreKind::Durable)` (i.e., in `Environment::create` and `Environment::open`), `remove_file(dirty_marker_path(path))` best-effort + `sync_dirent_chain` — a durable store's successful birth or verified open proves any resident marker refers to nothing. Belt: narrow the ephemeral wipe to the case where `data.mdb` exists; when it does not, there is nothing torn to wipe and the probe/fresh-init path handles the directory as-is. Land with the crash-window regression test: plant marker in empty dir → `Db::create` + commit → `Db::ephemeral` must refuse `StoreKindMismatch` with `data.mdb` byte-identical (the scratch repro in this verification is a ready template).