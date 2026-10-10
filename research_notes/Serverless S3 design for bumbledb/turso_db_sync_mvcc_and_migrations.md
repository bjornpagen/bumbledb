# Turso Database (ex-Limbo): storage, sync and MVCC, plus schema migrations in Drizzle, Turso/libSQL and Expo (as of 2026-10-09)

Method note: most claims below were checked against primary sources fetched on 2026-10-09: Turso blog posts, docs.turso.tech `.md` pages, raw source files from `tursodatabase/turso@main`, `drizzle-team/drizzle-orm@main` (v0 stable line) and `@rc5` (v1 RC line), and libsql-client-ts. Vendor benchmark numbers are labelled as vendor claims. "Inference" means my reasoning, not a sourced fact.

---

## 1. Turso Database: status and version in 2026, architecture (async I/O, io_uring, page cache), and how it differs from libSQL

### Takeaway
Turso Database is Turso's MIT-licensed Rust rewrite of SQLite. It is still pre-1.0: the latest stable release is v0.8.2 (2026-10-06), and v0.8.3-pre.1 came out on 2026-10-08. It replaces libSQL (the C fork) as the company's main direction. Its distinguishing features are async I/O built from explicit state machines (io_uring on Linux), Hekaton-style MVCC behind `BEGIN CONCURRENT`, built-in CDC, and a logical-push / physical-pull sync engine. On 2026-10-02 Turso announced that Supabase is acquiring it. The engine stays open source.

### Cited Findings
- **Releases:** GitHub releases show `v0.8.0` (2026-09-29), `v0.8.1` (2026-09-29), `v0.8.2` (2026-10-06, the latest non-prerelease) and `v0.8.3-pre.1` (2026-10-08). The repo description reads "A SQL database in Rust: SQLite-compatible, now also speaking Postgres (experimental). The LLVM of databases." — [GitHub releases API / repo](https://github.com/tursodatabase/turso/releases)
- **Production readiness:** the README FAQ says Turso "powers production applications today at multiple organizations, including Turso Cloud, the Kin AI assistant, and Spice.ai". It also says "we have not yet reached 1.0", that some features are explicitly experimental, and it recommends keeping independent backups. Testing uses native Deterministic Simulation Testing and Antithesis. — [Turso README](https://github.com/tursodatabase/turso/blob/main/README.md)
- **Compatibility:** Turso is compatible with SQLite's SQL dialect, file format and C API, tracking SQLite 3.50.4. Existing SQLite files work as-is, but the README says "We are not at 100% yet", and full compatibility is a requirement for 1.0. — [README](https://github.com/tursodatabase/turso/blob/main/README.md), [COMPAT.md](https://github.com/tursodatabase/turso/blob/main/COMPAT.md)
- **Feature list (README):**
  - `BEGIN CONCURRENT` (MVCC)
  - CDC
  - Bindings for Go, JS, Java, .NET, Python, Rust and WASM
  - "Asynchronous I/O support on Linux with `io_uring`"
  - Vector support
  - "Improved schema management including extended `ALTER` support and faster schema changes"
  - Experimental: Postgres dialect and wire-protocol frontend, encryption at rest, DBSP incremental view maintenance, Tantivy full-text search, and multi-process WAL via a `.tshm` sidecar. — [README](https://github.com/tursodatabase/turso/blob/main/README.md)
- **Turso vs libSQL (README FAQ):** libSQL evolves SQLite "through a fork rather than a rewrite". The FAQ says the rewrite "replaces libSQL as our intended direction. Both run in production today: libSQL has been battle-tested for longer, while Turso Database is where our development effort is focused". — [README FAQ](https://github.com/tursodatabase/turso/blob/main/README.md)
- **Async I/O model:** "Turso uses cooperative yielding with explicit state machines instead of Rust async/await." Functions return `IOResult<T> = Done(T) | IO(IOCompletions)` and must be called again until they return `Done`. A `CompletionGroup` aggregates several I/O completions. — [async-io-model.md](https://github.com/tursodatabase/turso/blob/main/docs/agent-guides/async-io-model.md)
- **v0.7 (2026-07-13) changes:** "The core engine no longer blocks the calling thread on I/O, no longer aborts when it runs out of memory, and yields the CPU during long operations so a busy statement cannot starve other connections sharing the same runtime." This was done for embedding into Turso Cloud. The release also moved to fallible allocation so that "one tenant [can] hit a memory limit without taking down a server shared with others". — [Turso v0.7.0 blog](https://turso.tech/blog/turso-0.7.0)
- **v0.8 (2026-09-29):** "Turso's architecture is designed for high concurrency with asynchronous I/O, but we haven't fully taken advantage of it yet." The release focus is `BEGIN CONCURRENT` performance, adding group commit. — [Turso 0.8 blog](https://turso.tech/blog/turso-0.8.0)
- **Hosting on Turso Cloud:** Turso Cloud hosts both engines: "Turso — a ground-up rewrite of SQLite" with concurrent writes, and "libSQL — a fork of SQLite, battle-tested in production on Turso Cloud for years". "Turso databases on Turso Cloud are in early preview"; you create one with `turso db create --tursodb`. — [docs.turso.tech/turso-cloud](https://docs.turso.tech/turso-cloud)
- **Supabase acquisition (2026-10-02):** "Turso … is being acquired by Supabase." The post promises that "Turso keeps running", that "Turso Database remains open source and actively developed", and "a clear graduation path" to Postgres and Multigres. Glauber Costa becomes Head of Agentic Services at Supabase. — [Turso is joining Supabase](https://turso.tech/blog/turso-is-joining-supabase)
- **Turso Cloud's diskless design (relevant to bumbledb on S3):** the cloud is "built on a diskless, WAL-on-S3 architecture". — [Supabase post](https://turso.tech/blog/turso-is-joining-supabase)
  - "Commits are only acknowledged once data is safely stored in either S3 or S3-express. Compute nodes can come and go at any time, and local disks act as a local cache." — [Turso durability docs](https://docs.turso.tech/cloud/durability)
  - New commits go to S3 Express One Zone before they are acknowledged. The WAL is periodically folded into the main DB file stored on S3. Commits are batched across many tenant databases and in time. — [Turso durability docs](https://docs.turso.tech/cloud/durability)
  - Added commit latency ceilings by plan: Free up to 100 ms, Developer 50 ms, Scaler 25 ms, Pro and above 10 ms. — [Turso durability docs](https://docs.turso.tech/cloud/durability)
- **Diskless architecture announcement (2025-04-07):**
  - The local disk is a write-through cache: "Once data is on S3 Express, we also write it locally."
  - fsync costs about 2 ms, so S3 Express adds "only about 4ms in the 4kB case".
  - Per-database PUT cost becomes viable only through batching across around 100 active databases per node. — [Turso Cloud Goes Diskless](https://turso.tech/blog/turso-cloud-goes-diskless)
- **AWS Storage Blog (2026-07-23):** a 4 KB PUT to S3 Express One Zone averages 6.4 ms, with a 7 ms P99. The platform "accumulates WAL writes from dozens of databases over a few milliseconds, then uploads them together in a single PUT". "Every few minutes, the WAL is folded into the full database file through a checkpointing process", and the result is stored in S3 Standard as the recovery baseline. — [AWS Storage Blog](https://aws.amazon.com/blogs/storage/how-turso-built-a-transactional-database-using-amazon-s3-express-one-zone/)
- **Closed-source server:** in January 2025 Turso said its new "massively multitenant version of our server, using Deterministic Simulation Testing … [will be kept] closed source". — [Upcoming changes to the Turso platform (2025-01-21)](https://turso.tech/blog/upcoming-changes-to-the-turso-platform-and-roadmap)
- **SAVEPOINT support (Turso engine):** the current COMPAT.md marks `SAVEPOINT` and `RELEASE SAVEPOINT` as supported (✅), and the transactions reference documents nested savepoints. It also states "Each connection can have at most one active top-level transaction at a time." — [COMPAT.md](https://github.com/tursodatabase/turso/blob/main/COMPAT.md), [Transactions docs](https://docs.turso.tech/sql-reference/statements/transactions)
  - Conflict: a 2026 search summary claimed "Turso does not support SAVEPOINT". That claim appears superseded.

### Inferences
- Turso Cloud's storage design (WAL frames durably in S3 Express before ack, periodic checkpoint to an S3 Standard base file, local disk as cache, cross-tenant batching of PUTs) is a close precedent for an S3-hosted bumbledb. The two key levers are the batching window and the checkpoint cadence. The closed-source server means the exact segment/manifest layout is not public.
- The Supabase acquisition (one week before this note) adds roadmap uncertainty for Turso Cloud features such as concurrent writes, sync and multi-DB. The engine itself stays MIT.

### Gaps
- I found no primary-source description of Turso's page cache (sizing, eviction, sharing between connections). The README and manual mention only io_uring and the async model.
- I did not verify the exact date or announcement of the Limbo → Turso rename. Repo artifacts still say "limbo", for example `docs/contributing/limbo_architecture.png` and the `LimboError` type in the sync engine.
- I found no statement on whether io_uring is now on by default. The 2025 MVCC post said they were "working towards enabling io_uring by default" ([blog](https://turso.tech/blog/beyond-the-single-writer-limitation-with-tursos-concurrent-writes)).
- The search summary mentioned 128 kB segments and "generations" for the diskless storage. I could not confirm those details in the fetched text of the diskless post, so treat them as unverified.

---

## 2. The Turso sync engine: push/pull, offline writes, logical vs physical representation, conflicts, protocol, bootstrap, target environments

### Takeaway
Turso Sync (`@tursodatabase/sync`, Go `tursogo`, Python `turso.sync`) is local-first, and the two directions use different representations:
- **Push** sends logical, row-level changes derived from CDC, replayed on the server as SQL over the Hrana `/v2/pipeline` endpoint. Push is idempotent through a per-client `turso_sync_last_change_id` table on the server.
- **Pull** fetches physical pages over a protobuf `/pull-updates` endpoint, or the MVCC logical log since v0.7. Local unpushed changes are rolled back and replayed on top of what was pulled: a rebase.

The server is the source of truth. Conflict resolution is "last push wins", with an optional per-mutation `transform` hook. There is no peer-to-peer sync. A new client either bootstraps the full DB, starts empty, or (experimentally) does a partial bootstrap that lazily fetches pages.

### Cited Findings
- **Launch and API (Oct 8, 2025):** Turso Sync launched with `push()`, `pull()` and a sync-aware `checkpoint()`. Browser use needs `@tursodatabase/sync-wasm`. — [Introducing Databases Anywhere with Turso Sync](https://turso.tech/blog/introducing-databases-anywhere-with-turso-sync)
- **Hybrid representation (verbatim):** "Local changes are pushed to the remote as logical mutations, allowing for flexible conflict resolution. Remote changes are pulled as physical pages, ensuring the local replica eventually becomes byte-for-byte identical to the remote database state. The remote database acts as the source of truth … fully distributed, peer-to-peer synchronization between devices is not supported." — [Turso Sync launch](https://turso.tech/blog/introducing-databases-anywhere-with-turso-sync)
- **WAL retention:** "The database's WAL is retained to extract local changes and 'rebase' them after a successful pull." Auto-checkpoint is disabled for sync DBs, so you must call `checkpoint()` yourself. It compacts the WAL "while preserving sync state". — [Turso Sync launch](https://turso.tech/blog/introducing-databases-anywhere-with-turso-sync), [Checkpoint docs](https://docs.turso.tech/sync/checkpoint)
- **Conflict semantics (docs):** "Turso sync uses a **last push wins** strategy." During a pull with unpushed local changes, three steps run, and "This rollback-and-replay happens atomically — if anything fails, your database remains in its previous state.":
  - (1) the local DB is rolled back to the last synced state;
  - (2) remote changes are applied;
  - (3) unpushed local changes are replayed on top. — [Conflict Resolution docs](https://docs.turso.tech/sync/conflict-resolution)
- **Push behaviour:** "Under the hood, logical statements are sent, and on conflicts the strategy is 'last push wins'." Pull returns a boolean "changed". "If you pushed earlier, a subsequent pull can still return that something changed due to server-side conflict resolution frames." — [Sync usage docs](https://docs.turso.tech/sync/usage)
- **Custom merge logic:** "if an application requires more sophisticated logic, the sync package exposes a custom transform hook that is applied to all mutations before they are sent to the remote". The hook can rewrite each mutation, for example turning "set value=7" into "increment by 3". — [Turso Sync launch](https://turso.tech/blog/introducing-databases-anywhere-with-turso-sync), [Local-first guide (2026-07-27)](https://turso.tech/blog/building-local-first-apps-the-complete-guide-to-offline-first-database-sync)
  ```ts
  const db = await connect({ path, url, authToken,
    transform: m => ({ operation: 'rewrite',
      stmt: { sql: `UPDATE counter SET value = value + ? WHERE key = ?`,
              values: [m.after.value - m.before.value, m.after.key] } }) });
  ```
  - The source shows the transform results `Skip` and `Rewrite(replay)` (`DatabaseRowTransformResult`). — [database_sync_operations.rs](https://github.com/tursodatabase/turso/blob/main/sync/engine/src/database_sync_operations.rs)
  - The guide also says: "There is no built-in CRDT support", and "create your schema up front on a connection without a transform". — [Local-first guide](https://turso.tech/blog/building-local-first-apps-the-complete-guide-to-offline-first-database-sync)
- **Offline writes:** "write locally and call `push()` when the connection is available. All changes are safely stored in the local database file until they can be synced." `bootstrapIfEmpty: false` lets the app start offline on first launch, with an empty DB. This is described as "the modern equivalent of the `offline: true` flag from `@libsql/client` Embedded Replicas". — [Sync usage docs](https://docs.turso.tech/sync/usage)
- **Bootstrap on first connect:** "On the first run, the local database is automatically bootstrapped from the remote — so the remote must be reachable during the initial connect", unless you pass `bootstrapIfEmpty: false`. `longPollTimeoutMs` makes the server hold a pull open until changes arrive. — [Sync usage docs](https://docs.turso.tech/sync/usage)
- **Partial sync (experimental, `partialSyncExperimental`):** "The client lazily fetches pages of the database file from the Turso Cloud when a query touches data that is not present locally." Writes still apply locally first and are pushed as logical statements. There are two bootstrap strategies:
  - **Prefix:** the first N bytes, e.g. `{kind:'prefix', length:128*1024}`.
  - **Query:** pages touched by a server-side SQL query, e.g. `SELECT * FROM messages WHERE user_id='u_123' LIMIT 100`. — [Partial sync docs](https://docs.turso.tech/sync/partial)
- **Stats:** `stats()` exposes `cdcOperations`, `mainWalSize`, `revertWalSize`, `networkReceivedBytes`, `networkSentBytes`, `lastPullUnixTime`, `lastPushUnixTime` and `revision`. — [Sync usage docs](https://docs.turso.tech/sync/usage)
  - The "revert WAL" is part of the rollback-and-replay machinery. The source comments mention "stale revert frames". — [database_sync_operations.rs](https://github.com/tursodatabase/turso/blob/main/sync/engine/src/database_sync_operations.rs)
- **Protocol, from source:**
  - **Pull:** protobuf `POST /pull-updates` (`PullUpdatesReqProtoBody`) with:
    - page encoding (raw or zstd);
    - a stream kind: `Pages(0)` or `MvccLogicalLog(1)`;
    - `server_revision` and `client_revision`;
    - a long-poll timeout;
    - `server_pages_selector` as "bytes for RoaringBitmap with bits set for pages to return";
    - a server query that selects pages.
  - **Push:** goes through Hrana `POST /v2/pipeline` batches (`BatchStreamReq` with `BatchCond`).
  - **Legacy endpoints:** WAL-frame endpoints `GET /sync/{generation}/{start_frame}/{end_frame}`, plus `GET /info` and `GET /export/{generation}`, used by a `bootstrap_db_file_legacy` path. — [server_proto.rs](https://github.com/tursodatabase/turso/blob/main/sync/engine/src/server_proto.rs), [database_sync_operations.rs](https://github.com/tursodatabase/turso/blob/main/sync/engine/src/database_sync_operations.rs)
- **Push idempotency / exactly-once replay (from source):** the server-side DB keeps a bookkeeping table, read before pushing ("fetch last_change_id from the target DB in order to guarantee atomic replay of changes and avoid conflicts in case of failure"):
  ```sql
  CREATE TABLE IF NOT EXISTS turso_sync_last_change_id (client_id TEXT PRIMARY KEY, pull_gen INTEGER, change_id INTEGER);
  INSERT INTO turso_sync_last_change_id(client_id, pull_gen, change_id) VALUES (?, ?, ?)
    ON CONFLICT(client_id) DO UPDATE SET pull_gen=excluded.pull_gen, change_id=excluded.change_id;
  ```
  — [database_sync_operations.rs](https://github.com/tursodatabase/turso/blob/main/sync/engine/src/database_sync_operations.rs)
- **How DDL travels over sync (from source):**
  - CDC captures schema changes as row changes to `sqlite_schema`, and the replay generator re-executes the stored `sql` text (flagged `is_ddl_replay`).
  - On push, CREATE statements are rewritten with `IF NOT EXISTS` "so it can be replayed on a remote which may already have the object (e.g. another client pushed its own version of the DDL first)".
  - The replay path tolerates CDC records with "fewer columns than the current schema (e.g. records captured before ALTER TABLE ADD COLUMN)", and generates `ALTER TABLE … RENAME COLUMN` for renames. — [database_sync_operations.rs](https://github.com/tursodatabase/turso/blob/main/sync/engine/src/database_sync_operations.rs), [database_replay_generator.rs](https://github.com/tursodatabase/turso/blob/main/sync/engine/src/database_replay_generator.rs)
- **Sync auth for offline-first:**
  - Fine-grained token permissions: `data_read`, `data_add`, `data_update`, `data_delete`, `schema_add`, `schema_update` and `schema_delete`, scoped per table, e.g. `turso db tokens create <db> -p all:data_read -p table:data_update`.
  - External JWKS/OIDC (Clerk and Auth0 during beta). — [Turso Sync launch](https://turso.tech/blog/introducing-databases-anywhere-with-turso-sync)
- **CDC table schema:** `turso_cdc(change_id, change_time, change_txn_id, change_type [1=INSERT, 0=UPDATE, -1=DELETE, 2=COMMIT], table_name, id, before BLOB, after BLOB, updates BLOB)`. It is enabled per connection with `PRAGMA capture_data_changes_conn('full')`. Only committed transactions are recorded. "CDC cannot be used together with MVCC … mutually exclusive on the same connection." — [CDC docs](https://docs.turso.tech/tursodb/cdc)
- **Syncing MVCC databases (v0.7):** "After an initial page bootstrap, it pulls the MVCC logical log directly, decodes the portable change metadata … stable table names, schema changes, rowids, and record bytes — and replays those changes through the existing replay path instead of shipping pages (#7500)." — [Turso v0.7.0 blog](https://turso.tech/blog/turso-0.7.0)
  - A search snippet of the Go driver docs shows `PushOperationsThreshold` (0 means push the whole change set in one batch) and a `LogicalMvccPull` flag, which auto-detects the remote protocol from the first pull response by default. — [pkg.go.dev tursogo (search snippet only)](https://pkg.go.dev/turso.tech/database/tursogo)
- **Why physical replication failed in libSQL Embedded Replicas (Glauber Costa, 2026-04-24):**
  - "There is no good way to have a logical stream of changes in SQLite. So we built our replication protocol on physical pages."
  - Problems: page-level waste; "no visibility on what changed" (so writes went to the remote by default); "the local database often diverged, and had to be re-bootstrapped".
  - The post recommends Turso Sync over Embedded Replicas "regardless of the situation". — [Turso Sync benchmark post](https://turso.tech/blog/sync-benchmark)
  - Turso's docs say: "For new projects that need sync, we recommend Turso Sync." — [Embedded Replicas docs](https://docs.turso.tech/features/embedded-replicas/introduction)
- **Superseded: libSQL offline-write beta (2025-03-31).** Embedded Replicas offline writes shipped with "Conflict detection (but resolution is not yet implemented)". — [Offline Sync Public Beta](https://turso.tech/blog/turso-offline-sync-public-beta)
- **Edge replicas discontinued:** edge replicas were discontinued for new users in January 2025 because "70% of Turso users never create geographical replicas … syncing the SQLite file itself to your API or device is often a better approach". — [Upcoming changes (2025-01-21)](https://turso.tech/blog/upcoming-changes-to-the-turso-platform-and-roadmap)
- **Target environments:** front-end offline-first, ML pipelines, and "Backend applications can achieve microsecond-level read and write latencies" through a local synced copy. Mobile integration was on the roadmap in October 2025. A later post lists "Introducing React Native Bindings for Turso" (Jan 29, 2026). — [Turso Sync launch](https://turso.tech/blog/introducing-databases-anywhere-with-turso-sync)
- **Local sync server:** the CLI includes one that "implements the same sync protocol as Turso Cloud": `tursodb ./server.db --sync-server 0.0.0.0:8080`. — [Local Sync Server docs](https://docs.turso.tech/sync/local-sync-server)
- **Recent fix (search snippet, Sep 2026):** the `cdcOperations` stat counted internal rows and commit markers, so it "never dropped to zero after a push". The fix filters the count to match what push actually sends. — [Mirror PR #550 (search snippet)](https://github.com/Mu-L/turso/pull/550)

### Inferences
- Patterns worth copying for bumbledb:
  - **Logical push vs physical pull:** clients push intent (logical ops) to an authority, and pull authoritative state (pages, frames or a log), then rebase local pending ops.
  - **Server-side `(client_id → last_applied_change_id)` table:** this makes retries of a push idempotent.
- Schema changes plus sync is a sharp edge. DDL made on a client is pushed as raw DDL text (CREATE made idempotent with IF NOT EXISTS, but ALTER is not obviously idempotent). On pull, a server-side schema change arrives physically, and pending local row ops are replayed against the new schema. That is safe only for additive changes, which suggests expand-only migrations for synced databases.
- Last-push-wins at row level means that, for migrations which rewrite data (backfills), a slow offline client could overwrite migrated values with stale row images when it later pushes. A transform hook or app-level versioning would be needed.

### Gaps
- I found no documentation of what happens when replaying local changes fails after a pull because the schema changed incompatibly (for example, a column was dropped on the server). The docs say only that rollback-and-replay is atomic.
- The server-side conflict detection detail ("server-side conflict resolution frames") is not documented beyond last-push-wins.
- The search summary claimed React Native/Expo integration specifics for sync. I did not verify them beyond the blog title "Introducing React Native Bindings for Turso" (Jan 29, 2026).

---

## 3. MVCC / BEGIN CONCURRENT in Turso Database (and libSQL): design, status, throughput, interaction with sync

### Takeaway
Turso's MVCC follows Hekaton: optimistic, row-versioned, with snapshot isolation and row-level write-write conflict detection at commit. It is layered over the SQLite B-tree, pager and WAL, with its own logical log that is checkpointed into the database file. The status is mixed: the manual calls MVCC "a supported journal mode" but marks `journal_mode = mvcc` "not production ready". It is an "early preview" on Turso Cloud (since 2026-08-03). DDL such as CREATE INDEX does not run inside concurrent transactions. CDC is mutually exclusive with MVCC on a connection, so sync of MVCC databases uses the MVCC logical log instead (v0.7+).

### Cited Findings
- **Enabling it:** `PRAGMA journal_mode = 'mvcc';` then `BEGIN CONCURRENT; … COMMIT;`. "If two transactions touch the same rows, one will receive a conflict error and must roll back and retry." "Your application must detect conflict errors and retry." Errors surface as `Busy`/`BusySnapshot` or as messages containing "conflict". — [Concurrent Writes docs](https://docs.turso.tech/tursodb/concurrent-writes)
  - Superseded: the Oct 2025 preview used the `--experimental-mvcc` CLI flag. — [Beyond the single-writer limitation (2025-10-06)](https://turso.tech/blog/beyond-the-single-writer-limitation-with-tursos-concurrent-writes)
- **Design:**
  - "MVCC enables concurrent transactions to make progress by maintaining an in-memory index that tracks row versions … checking for row-level conflicts only at commit time."
  - "If a row does not exist in the MVCC in-memory index, we read it through the pager from WAL and B-Tree … We also eventually checkpoint the MVCC log into the SQLite database file via the WAL."
  - The design is inspired by Hekaton (Larson et al., VLDB'12). — [Beyond the single-writer limitation](https://turso.tech/blog/beyond-the-single-writer-limitation-with-tursos-concurrent-writes)
- **Data structures and commit protocol (Sep 2026):**
  - Rows live in `SkipMap<Rowid, Mutex<RowVersions>>`, a lock-free skip map with per-row version lists ("MVStore").
  - Transaction states are Active → Preparing → commit or abort. Snapshot isolation applies.
  - On commit: check for write-write conflicts, then "Write the transaction's rows to the logical log. Fsync the logical log."
  - "The logical log records only the rows a transaction changed: new rows, updates, and deletes (written as tombstones), with schema changes tracked separately. On restart, Turso replays the logical log."
  - Past a threshold, a checkpoint moves data into the SQLite-format DB file, and GC frees invisible versions.
  - v0.8 added group commit, so that concurrent committers share one fsync. — [Group commit post (2026-09-29)](https://turso.tech/blog/turso-group-commit)
- **Persistent log format:**
  - Transaction frames carry `payload_size, op_count, commit_ts, extension_size …`, a body of extension records plus recovery ops, and a crc/auth-tag trailer.
  - Recovery ops include table/index upserts and deletes, header updates, and "`sqlite_schema` row upserts/deletes for DDL".
  - A "portable" extension (string table plus object map) lets raw-log consumers such as sync resolve table IDs to names "outside the original database instance". — [PORTABLE_FORMAT.md](https://github.com/tursodatabase/turso/blob/main/docs/internals/mvcc/PORTABLE_FORMAT.md), [MVCC DESIGN.md](https://github.com/tursodatabase/turso/blob/main/docs/internals/mvcc/DESIGN.md)
- **Manual status and semantics:**
  - "MVCC is a supported journal mode." Its listed limitations include: `PRAGMA wal_checkpoint(TRUNCATE)` blocks readers and writers, and non-blocking passive checkpoint is still behind `--experimental-mvcc-passive-checkpoint`.
  - "If a database is written to using MVCC and then opened again without MVCC, the changes are not visible unless first checkpointed."
  - The journal-mode table still marks `mvcc` "not production ready".
  - "concurrent transactions never acquire locks … rely on MVCC's snapshot isolation and conflict detection at commit time."
  - "Only use `BEGIN IMMEDIATE` or `BEGIN DEFERRED` when you need exclusive write access that prevents any concurrent commits." — [Turso manual](https://github.com/tursodatabase/turso/blob/main/docs/manual.md)
- **Limitations (Aug 2026):**
  - "Concurrent writes are an early preview requiring dashboard opt-in and a tursodb database, and DDL such as CREATE INDEX does not run inside a concurrent transaction."
  - Hot rows still conflict ("that's just life under ACID").
  - The conflict error "can be raised on the conflicting write statement itself rather than at commit". — [Concurrent Writes in Practice (2026-08-27)](https://turso.tech/blog/concurrent-writes-in-practice), [Concurrent writes on Turso Cloud (2026-08-03)](https://turso.tech/blog/concurrent-writes-on-turso-cloud)
- **Comparison with SQLite (2026-08-03 table):**
  - Stock SQLite locks at transaction start, at whole-database granularity.
  - SQLite's unmerged `BEGIN CONCURRENT` branch locks at commit, with page-level conflicts.
  - Turso locks at commit, with row-level conflicts. — [Concurrent writes on Turso Cloud](https://turso.tech/blog/concurrent-writes-on-turso-cloud)
- **v0.7 MVCC work:**
  - The dual cursor merges the B-tree with in-memory versions in linear time.
  - Watermark-driven GC is decoupled from checkpoint.
  - Experimental passive checkpoint without the global lock.
  - Smaller per-version memory.
  - The sync engine can sync MVCC databases via the logical log. — [Turso v0.7.0](https://turso.tech/blog/turso-0.7.0)
- **v0.8:** FTS indexes became transactional with `BEGIN CONCURRENT` (#8425). — [Turso 0.8](https://turso.tech/blog/turso-0.8.0)
- **Early-preview limitations (Oct 2025, partly superseded by 0.7/0.8):**
  - No CREATE INDEX.
  - Full-row copies per version.
  - The version list was behind an RwLock and not wait-free.
  - No async I/O for concurrent transactions. — [Beyond the single-writer limitation](https://turso.tech/blog/beyond-the-single-writer-limitation-with-tursos-concurrent-writes)

### Inferences
- For migrations on Turso with MVCC, run DDL in a normal (non-concurrent) transaction. Per the manual, `BEGIN IMMEDIATE`/`DEFERRED` prevents concurrent commits, so a migration transaction effectively takes a global write lock. That is a usable migration mutex on a single Turso database.
- The "logical log of changed rows + periodic checkpoint into base file" design fits an S3 store well: append small row-level log objects, and periodically compact into base pages or segments.

### Gaps
- I found no statement that libSQL has MVCC or `BEGIN CONCURRENT`. Turso's posts discuss only SQLite's experimental branch. Treat libSQL as single-writer.
- I found no published conflict/abort-rate numbers under contention.

---

## 4. Published latency and throughput numbers for Turso Database and its sync engine

### Takeaway
All numbers are vendor-published microbenchmarks. With `BEGIN CONCURRENT`, Turso 0.8 claims 9,500 TPS at 64 connections versus SQLite's ~1,370 TPS, and a p99.9 commit latency of 2.4 ms at 32 connections versus 1.2 s for SQLite. For sync, it claims up to 312x faster and 18x less traffic than libSQL Embedded Replicas in a read-your-writes loop. On a single connection SQLite is still faster.

### Cited Findings
- **Turso 0.8 (2026-09-29):**
  - Hardware: Ryzen 9 3900XT, NVMe, XFS, `PRAGMA synchronous=FULL` for both engines, against SQLite 3.50.2.
  - Latency: "99.9th percentile latency of 2.4 ms at 32 connections (vs. 1.2 s for SQLite, 500x lower)".
  - Throughput: "9,500 TPS at 64 connections (vs. 1,370 TPS for SQLite, 7x higher)", with 100 rows per transaction on disjoint keys.
  - Pooled latency table at 1,000 tx/s: Turso p50 0.87 ms / p99 1.67 ms / p99.9 5.85 ms; SQLite p50 1.35 ms / p99 129 ms / p99.9 730 ms.
  - "SQLite … is faster than Turso with a single connection. Turso overtakes it at 2 connections." — [Turso 0.8](https://turso.tech/blog/turso-0.8.0)
- **Oct 2025 preview:**
  - SQLite reaches about 150k rows/s (100-row batches, FULL sync) and does not scale with threads; with 1 ms of compute per transaction it drops to about 80k rows/s.
  - Turso MVCC was "16% faster than SQLite when more threads are used" with no compute, and "4x faster" at 8 threads with 1 ms compute. — [Beyond the single-writer limitation](https://turso.tech/blog/beyond-the-single-writer-limitation-with-tursos-concurrent-writes)
- **Fsync cost:** the group-commit post shows fsync at about 700 µs on the benchmark machine. That is why group commit matters. — [Group commit post](https://turso.tech/blog/turso-group-commit)
- **Sync vs Embedded Replicas (2026-04-24).** Setup: VM near us-east-1, `@libsql/client@0.17.2` vs `@tursodatabase/sync@0.5.3`.

  | Scenario | Embedded Replicas | Turso Sync | Ratio |
  |---|---|---|---|
  | Sequential inserts (3,000 rows) | 152 s, 34.6 MB | 17 s, 2.1 MB | 8.9x faster |
  | Read-your-writes (200 cycles) | 48.6 s, 243 ms/cycle, 2.7 MB | 156 ms, <1 ms/cycle, 149 KB | "312x faster. 18.4x less data" |
  | Push after every write | 218 ms/cycle | 30 ms/cycle | 7.3x |
  | Pull 5,000 rows to a fresh DB | 718 ms | 338 ms | about the same traffic |

  — [Turso Sync benchmark](https://turso.tech/blog/sync-benchmark)
- **Turso Cloud storage numbers:**
  - S3 Express 4 KB PUT averages 6.4 ms, P99 7 ms. — [AWS Storage Blog](https://aws.amazon.com/blogs/storage/how-turso-built-a-transactional-database-using-amazon-s3-express-one-zone/)
  - Added commit-latency ceilings run from 10 ms (Pro) to 100 ms (Free). — [Durability docs](https://docs.turso.tech/cloud/durability)

### Inferences
- The SQLite single-connection advantage, plus the 700 µs fsync, suggests the sweet spot is many concurrent writers with compute inside transactions. Migration workloads, which are serialized DDL, gain nothing from MVCC.

### Gaps
- I found no independent (non-vendor) benchmarks of Turso Database or Turso Sync.
- I found no published numbers for partial-sync cold-start latency.

---

## 5. Drizzle migrations in depth: generate artifacts, `__drizzle_migrations`, how `migrate()` decides, transactions, failure handling

### Takeaway
There are two Drizzle lines in October 2026:
- **Stable v0** (`drizzle-orm@0.45.4`, `drizzle-kit@0.31.11`) uses `NNNN_tag.sql` files, `meta/_journal.json` and `meta/NNNN_snapshot.json`. Its `__drizzle_migrations(id, hash, created_at)` table is consulted only for the latest `created_at`. This "high-water mark" silently skips older-timestamped pending migrations, and the stored hash is never verified.
- **v1 RC** (`1.0.0-rc.4` on the `rc` tag; the docs site already shows v1) drops the journal. It uses `drizzle/<YYYYMMDDHHMMSS>_<name>/{migration.sql,snapshot.json}`, adds `name` and `applied_at` columns, and applies every local migration whose name is not in the table.

In both lines, all pending migrations run in one transaction (all-or-nothing), the read of history happens before that transaction, and core SQLite and libSQL take no lock against concurrent `migrate()` runs.

### Cited Findings
- **Versions (npm dist-tags, fetched 2026-10-09):** `drizzle-orm` latest `0.45.4` (published 2026-10-08), `rc` `1.0.0-rc.4`, `beta` `1.0.0-beta.22`. `drizzle-kit` latest `0.31.11`. — [npm drizzle-orm](https://www.npmjs.com/package/drizzle-orm), [npm drizzle-kit](https://www.npmjs.com/package/drizzle-kit)
- **v0 reading migrations:** `readMigrationFiles()` reads `${migrationsFolder}/meta/_journal.json`, typed as `{ entries: { idx, when, tag, breakpoints }[] }`. For each entry it reads `${tag}.sql` and splits on `--> statement-breakpoint`. It computes `hash = sha256(file contents)` and uses `folderMillis = journalEntry.when`. — [migrator.ts@main](https://github.com/drizzle-team/drizzle-orm/blob/main/drizzle-orm/src/migrator.ts)
  - The journal also has top-level `version` and `dialect` fields (`dryJournal = { version: snapshotVersion, dialect, entries: [] }`). — [drizzle-kit utils.ts@main](https://github.com/drizzle-team/drizzle-orm/blob/main/drizzle-kit/src/utils.ts)
- **v0 table and decision rule (SQLite/libSQL):**
  ```sql
  CREATE TABLE IF NOT EXISTS "__drizzle_migrations" (
    id SERIAL PRIMARY KEY,   -- literally "SERIAL" even on SQLite
    hash text NOT NULL,
    created_at numeric
  );
  SELECT id, hash, created_at FROM "__drizzle_migrations" ORDER BY created_at DESC LIMIT 1;
  -- apply each journal entry where: !lastDbMigration || Number(last.created_at) < migration.folderMillis
  INSERT INTO "__drizzle_migrations" ("hash","created_at") VALUES (?, ?);  -- per applied migration
  ```
  — [sqlite-core/dialect.ts@main](https://github.com/drizzle-team/drizzle-orm/blob/main/drizzle-orm/src/sqlite-core/dialect.ts), [libsql/migrator.ts@main](https://github.com/drizzle-team/drizzle-orm/blob/main/drizzle-orm/src/libsql/migrator.ts)
- **v0 transactions:**
  - `SQLiteSyncDialect.migrate` (used by expo-sqlite, better-sqlite3 and similar) does `CREATE TABLE IF NOT EXISTS` and the SELECT *outside* the transaction. It then runs `BEGIN`, all statements of all pending migrations plus their INSERTs, then `COMMIT`, or `ROLLBACK` and rethrow.
  - `SQLiteAsyncDialect.migrate` wraps the same loop in a single `session.transaction(...)`. — [sqlite-core/dialect.ts@main](https://github.com/drizzle-team/drizzle-orm/blob/main/drizzle-orm/src/sqlite-core/dialect.ts)
  - libSQL builds the full statement list and calls `db.session.migrate(statementToBatch)`, which calls `@libsql/client`'s `client.migrate()`. — [libsql/migrator.ts@main](https://github.com/drizzle-team/drizzle-orm/blob/main/drizzle-orm/src/libsql/migrator.ts), [libsql/session.ts@main](https://github.com/drizzle-team/drizzle-orm/blob/main/drizzle-orm/src/libsql/session.ts)
- **`@libsql/client` `migrate()`:**
  - Local: runs `PRAGMA foreign_keys=off`, then `BEGIN DEFERRED`, then all statements, aborting with `LibsqlBatchError` on the first failure. — [sqlite3.ts](https://github.com/tursodatabase/libsql-client-ts/blob/main/packages/libsql-client/src/sqlite3.ts)
  - Remote (HTTP/Hrana): sends a single pipelined batch, `executeHranaBatch("deferred", …, disableForeignKeys=true)`. That emits `PRAGMA foreign_keys=off` and BEGIN, with each step conditioned on the previous step succeeding, all in one HTTP request. — [http.ts](https://github.com/tursodatabase/libsql-client-ts/blob/main/packages/libsql-client/src/http.ts), [hrana.ts](https://github.com/tursodatabase/libsql-client-ts/blob/main/packages/libsql-client/src/hrana.ts)
- **v0 high-water-mark bug, open:** issue #5769 (2026-05-16): "`drizzle-kit migrate` silently skips pending migrations when MAX(created_at) in __drizzle_migrations exceeds the journal `when` of pending entries … The CLI still prints `[✓] migrations applied successfully!` and exits 0." — [drizzle-orm#5769](https://github.com/drizzle-team/drizzle-orm/issues/5769)
  - The same report against the v1 beta (#5316) was closed on 2026-03-05, consistent with the v1 name-based rule below. — [drizzle-orm#5316](https://github.com/drizzle-team/drizzle-orm/issues/5316)
- **Hash is write-only:** feature request #6179 notes the table "cannot answer 'which migrations are applied on this database' on its own". `hash` "stops matching if a migration file was edited after being applied". — [drizzle-orm#6179](https://github.com/drizzle-team/drizzle-orm/issues/6179)
  - Confirmed in code: no migrator compares `hash`. The v0 Expo migrator even stores `hash: ''`. — [expo-sqlite/migrator.ts@main](https://github.com/drizzle-team/drizzle-orm/blob/main/drizzle-orm/src/expo-sqlite/migrator.ts)
- **v1 RC folder layout:**
  - `readMigrationFiles` throws if `meta/_journal.json` exists ("You must upgrade drizzle-kit and run 'drizzle-kit up'").
  - It lists subfolders containing `migration.sql` and sorts them by name with `localeCompare`.
  - `folderMillis` comes from the first 14 chars (`YYYYMMDDHHMMSS`). `name` is the folder name. — [migrator.ts@rc5](https://github.com/drizzle-team/drizzle-orm/blob/rc5/drizzle-orm/src/migrator.ts), [migrator.utils.ts@rc5](https://github.com/drizzle-team/drizzle-orm/blob/rc5/drizzle-orm/src/migrator.utils.ts)
- **v1 RC table and decision rule:**
  ```sql
  CREATE TABLE IF NOT EXISTS "__drizzle_migrations" (
    id INTEGER PRIMARY KEY, hash text NOT NULL, created_at numeric, name text, applied_at TEXT);
  ```
  ```ts
  // getMigrationsToRun: apply every local migration whose name is not already recorded
  const dbNamesSet = new Set(dbMigrations.map(m => m.name).filter(n => n !== null));
  return localMigrations.filter(lm => !lm.name || !dbNamesSet.has(lm.name));
  ```
  — [migrator.utils.ts@rc5](https://github.com/drizzle-team/drizzle-orm/blob/rc5/drizzle-orm/src/migrator.utils.ts), [libsql/migrator.ts@rc5](https://github.com/drizzle-team/drizzle-orm/blob/rc5/drizzle-orm/src/libsql/migrator.ts)
- **Table auto-upgrade (v1):** `upgradeSyncIfNeeded`/`upgradeAsyncIfNeeded` detect "Version 0: (id, hash, created_at)" vs "Version 1: (…, name, applied_at)" via `pragma_table_info`. They backfill `name` by matching DB rows (ordered by id) to local migrations sorted by millis, then name, "If multiple migrations share the same second, use hash matching as a tiebreaker". — [up-migrations/sqlite.ts@rc5](https://github.com/drizzle-team/drizzle-orm/blob/rc5/drizzle-orm/src/up-migrations/sqlite.ts)
- **v1 transactions:** `migrateSync` (Expo and other sync drivers) runs BEGIN, all pending migrations plus their INSERTs, then COMMIT, or ROLLBACK ("original error takes priority"). `migrateAsync` (`tursodatabase` driver) uses one `db.session.transaction`. libSQL still uses one `client.migrate` batch. — [sqlite-core/async/session.ts@rc5](https://github.com/drizzle-team/drizzle-orm/blob/rc5/drizzle-orm/src/sqlite-core/async/session.ts)
- **v1 `init` mode:** used by `drizzle-kit pull --init`. It inserts the single baseline migration as already-applied and returns `{exitCode:'databaseMigrations'|'localMigrations'}` on misuse. — [libsql/migrator.ts@rc5](https://github.com/drizzle-team/drizzle-orm/blob/rc5/drizzle-orm/src/libsql/migrator.ts)
- **v1 docs rationale:** "removing journal.json; grouping SQL files and snapshots into separate migration folders; removing the drizzle-kit drop command. These changes eliminate potential Git conflicts with the journal file." Also: "Migrated from database snapshots to DDL snapshots" and "Commutativity checks were added: Detecting non-commutative migrations across branches". — [Upgrading to Drizzle v1](https://orm.drizzle.team/docs/upgrade-v1), [discussion #2832](https://github.com/drizzle-team/drizzle-orm/discussions/2832)
- **`drizzle-kit migrate` flow (docs):** "Reads through migration folder … fetches entries from drizzle migrations log table … decide which new migrations to run … Runs SQL migrations and logs applied migrations". The table and schema can be renamed via `migrations: { table, schema }`. — [drizzle-kit migrate docs](https://orm.drizzle.team/docs/drizzle-kit-migrate)
- **Runtime migration (Drizzle's Option 4):** "widely used for monolithic applications when you apply database migrations during zero downtime deployment and rollback DDL changes if something fails. This is also used in serverless deployments with migrations running in custom resource once during deployment process." — [Drizzle migrations overview](https://orm.drizzle.team/docs/migrations)
  - The "Migrations for teams" and "Web and mobile" pages are still placeholders ("will be updated in the next release"). — [teams](https://orm.drizzle.team/docs/kit-migrations-for-teams), [web/mobile](https://orm.drizzle.team/docs/kit-web-mobile)
- **No concurrency protection:** issue #874 (2023, still open): "`migrate` isn't protected against simultaneous execution … `Promise.all([migrate(), migrate(), migrate()])` will lead to the migrations running three times". — [drizzle-orm#874](https://github.com/drizzle-team/drizzle-orm/issues/874)
  - Open PR #6451 (2026-10-06) fixes this for Postgres only. Its diagnosis: "`migrate()` creates the migrations schema and table, and reads the history, outside the transaction that applies migrations. Two runs … both apply the same pending migration and both succeed". The fix wraps everything in one `read committed` transaction starting with `select 1 from pg_advisory_xact_lock(hashtext('<schema>.<table>'))`. — [drizzle-orm PR #6451](https://github.com/drizzle-team/drizzle-orm/pull/6451)

### Inferences
- **Failure handling:** because every pending migration runs in one transaction, a failure in migration N also rolls back migrations 1..N-1 of the same run. This holds on SQLite, where DDL is transactional. There is no partial-progress record and no "failed" state. The next start simply retries everything pending, unlike Prisma's sticky failed state.
  - Exception: migration SQL that commits implicitly or cannot run in a transaction (`PRAGMA foreign_keys` inside a transaction is a no-op in SQLite; `VACUUM`).
  - libSQL disables foreign keys around the batch, which is how table-rebuild migrations avoid FK failures.
- **Concurrent starts on SQLite/libSQL (v0 and v1):** the history SELECT runs before BEGIN (deferred). Two instances can both compute the same pending set. SQLite's single writer then serializes the transactions, so the second runs after the first commits.
  - It will usually fail on non-idempotent DDL ("table already exists") and roll back, a noisy but safe failure.
  - With idempotent DDL (`IF NOT EXISTS`), its data statements and history INSERT run twice, producing duplicate rows in `__drizzle_migrations`.
  - A correct design re-reads history inside a write-locked transaction (`BEGIN IMMEDIATE`), or takes an external lease.
- **Lessons for bumbledb's migration ledger:**
  - Key it by a stable migration ID/name, not a timestamp high-water mark.
  - Store a checksum *and verify it*.
  - Record `applied_at`.
  - Decide what to run inside the same atomic/locked unit that applies it.

### Gaps
- I did not inspect drizzle-kit's `migrate` CLI path in v1 to confirm it calls the same `migrateAsync`/libsql migrators; the docs' description implies it does.
- I did not verify how v1 handles migration folders generated within the same second on different branches beyond the hash tiebreak comment.

---

## 6. Drizzle with Expo SQLite: `drizzle-orm/expo-sqlite/migrator`, `useMigrations`, bundling via babel-plugin-inline-import and Metro, applying on device

### Takeaway
On React Native, migrations ship inside the JS bundle: `.sql` files are inlined as strings by `babel-plugin-inline-import`, and Metro must accept the `sql` extension. drizzle-kit (with `driver: 'expo'`) generates a `migrations.js` index. At startup, `useMigrations(db, migrations)` (a React hook) or `migrate(db, migrations)` runs them synchronously, in one transaction, against the on-device SQLite file. Expo's own docs show the simpler alternative: a hand-rolled `PRAGMA user_version` ladder.

### Cited Findings
- **Setup:**
  - `babel.config.js`: `plugins: [["inline-import", { "extensions": [".sql"] }]]`.
  - `metro.config.js`: `config.resolver.sourceExts.push('sql')`.
  - `drizzle.config.ts`: `dialect: 'sqlite', driver: 'expo'` ("very important").
  - The docs say "Expo / React Native requires you to have SQL migrations bundled into the app". — [Drizzle Expo SQLite docs](https://orm.drizzle.team/docs/connect-expo-sqlite)
- **Usage on device:**
  ```tsx
  import { useMigrations } from 'drizzle-orm/expo-sqlite/migrator';
  import migrations from './drizzle/migrations';
  const { success, error } = useMigrations(db, migrations); // render error / "in progress" / app
  ```
  — [Drizzle Expo SQLite docs](https://orm.drizzle.team/docs/connect-expo-sqlite)
  - The current docs install `drizzle-orm@rc expo-sqlite@next`, i.e. the v1 RC line. — [Drizzle Expo SQLite docs](https://orm.drizzle.team/docs/connect-expo-sqlite)
- **Generated `migrations.js`, v0:**
  ```js
  import journal from './meta/_journal.json';
  import m0000 from './0000_xxx.sql';
  export default { journal, migrations: { m0000 } }
  ```
  — [drizzle-kit migrate.ts@main `embeddedMigrations`](https://github.com/drizzle-team/drizzle-orm/blob/main/drizzle-kit/src/cli/commands/migrate.ts)
- **Generated `migrations.js`, v1:** `export default { migrations: { "<folder_name>": mNNNN, … } }`, built from each folder's `migration.sql`, with no journal. — [drizzle-kit generate-common.ts@rc5](https://github.com/drizzle-team/drizzle-orm/blob/rc5/drizzle-kit/src/cli/commands/generate-common.ts)
- **Expo migrator, v0:** iterates `journal.entries` and looks up `migrations['m'+idx.padStart(4,'0')]`. It throws `Missing migration: <tag>` if one is absent. It stores `hash: ''`, then calls `db.dialect.migrate(...)`: the sync dialect, a single BEGIN/COMMIT.
  - `useMigrations` runs `migrate` once in `useEffect([])` and exposes `{success, error}` through `useReducer`. — [expo-sqlite/migrator.ts@main](https://github.com/drizzle-team/drizzle-orm/blob/main/drizzle-orm/src/expo-sqlite/migrator.ts)
- **Expo migrator, v1 RC:** sorts `Object.keys(migrations)`, derives millis from the first 14 chars of the key, and uses `name: key`. It calls `migrateSync(...)` (name-set diff, single transaction).
  - The `useMigrations` type signature still declares a `journal` field, but only `migrations` is used. — [expo-sqlite/migrator.ts@rc5](https://github.com/drizzle-team/drizzle-orm/blob/rc5/drizzle-orm/src/expo-sqlite/migrator.ts)
- **Expo's native pattern:** `<SQLiteProvider databaseName="test.db" onInit={migrateDbIfNeeded}>`, where:
  ```ts
  const DATABASE_VERSION = 1;
  let { user_version } = await db.getFirstAsync('PRAGMA user_version');
  if (user_version >= DATABASE_VERSION) return;
  if (user_version === 0) { await db.execAsync(`PRAGMA journal_mode='wal'; CREATE TABLE todos (...);`); user_version = 1; }
  await db.execAsync(`PRAGMA user_version = ${DATABASE_VERSION}`);
  ```
  — [Expo SQLite docs](https://docs.expo.dev/versions/latest/sdk/sqlite/)

### Inferences
- On-device migration is the "migrations ship with app code, apply on startup" model in its purest form. There is exactly one process per DB file, so concurrency is a non-issue. The risks are a failed migration bricking app start (the UI must handle `error`) and the impossibility of rolling back app binaries already in users' hands. So migrations must be forward-only, and old data must remain readable.
- Expo's `user_version` ladder is atomic only if each step and its version bump run in one transaction. The example does not wrap them, so a crash mid-ladder can re-run a step.

### Gaps
- I did not find Drizzle docs on combining Expo SQLite migrations with a synced remote, i.e. which side owns the schema. Drizzle documents no Turso-Sync-plus-Expo migration story; its connect page for "Turso Database Sync" returned 404 at the slug I tried.

---

## 7. Drizzle with Turso/libSQL: migrate at deploy vs startup, `drizzle-kit push`, multi-DB / per-tenant migration and the fate of Turso "multi-DB schemas"

### Takeaway
Turso's Drizzle guide uses `dialect: "turso"` with `@libsql/client`, and runs `drizzle-kit generate` then `drizzle-kit migrate` from a dev or CI machine against the remote URL (deploy time). Runtime `migrate(db, {migrationsFolder})` from `drizzle-orm/libsql/migrator` is the startup alternative. Drizzle v1 also targets the new engine via `drizzle-orm/tursodatabase/*`. Turso's server-side multi-DB schemas (a parent schema DB that auto-propagates DDL to children) are deprecated for new users since January 2025. Turso now tells per-tenant users "You own the migration workflow".

### Cited Findings
- **Turso's Drizzle guide:**
  - Install `drizzle-orm @libsql/client` and `drizzle-kit`.
  - Scripts: `"db:generate": "drizzle-kit generate"`, `"db:migrate": "drizzle-kit migrate"`.
  - Config: `dialect: "turso"`, `dbCredentials: { url: TURSO_DATABASE_URL, authToken }`.
  - Connect with `drizzle-orm/libsql` + `@libsql/client` (Node/serverless) or `@libsql/client/web` (edge). — [Drizzle + Turso docs](https://docs.turso.tech/sdk/ts/orm/drizzle)
  - "There is also beta support for `@tursodatabase/database`" for local or embedded use. — [Drizzle + Turso docs](https://docs.turso.tech/sdk/ts/orm/drizzle)
- **Drizzle v1 Turso Database driver:** `npm i drizzle-orm@rc @tursodatabase/database`; `import { drizzle } from 'drizzle-orm/tursodatabase/database'`. — [Drizzle Turso Database docs](https://orm.drizzle.team/docs/connect-turso-database)
  - The v1 source tree has `tursodatabase/`, `tursodatabase-serverless/` and `tursodatabase-sync/` driver folders. `tursodatabase/migrator.ts` calls `migrateAsync`, a single transaction. — [drizzle-orm src@rc5](https://github.com/drizzle-team/drizzle-orm/tree/rc5/drizzle-orm/src), [tursodatabase/migrator.ts@rc5](https://github.com/drizzle-team/drizzle-orm/blob/rc5/drizzle-orm/src/tursodatabase/migrator.ts)
- **Multi-DB Schemas (Deprecated):**
  - "This feature is now deprecated for all new users. Existing paid users can continue to use Multi-DB Schemas."
  - Mechanism: `turso db create parent-db --type schema`, then `turso db create child-db --schema parent-db`. "You apply schema changes to the parent database, child databases are automatically updated with the new schema." The Platform API uses an `is_schema: true` flag. — [Multi-DB Schemas docs](https://docs.turso.tech/features/multi-db-schemas)
  - Rationale (Jan 2025): "We'll be removing Multi-DB schemas and database ATTACH capabilities for new users … we believe we can make them even better with a fresh architecture." — [Upcoming changes (2025-01-21)](https://turso.tech/blog/upcoming-changes-to-the-turso-platform-and-roadmap)
- **Current per-tenant guidance (2026-08-17):**
  - Seed each tenant DB from a template (`name: "template-db", // copies schema and data from a template database`), keep a central registry DB mapping tenant → database, and use group tokens.
  - "Schema migrations need coordination. You own the migration workflow, including version tracking, retries, and handling databases that are temporarily behind." — [Multi-tenancy at Scale](https://turso.tech/blog/multi-tenancy-at-scale)
- **`drizzle-kit push`:** this is Drizzle's Option 2, "I don't wanna deal with SQL migration files … push my schema directly to the database". Drizzle positions it as a codebase-first approach without a migration history. — [Drizzle migrations overview](https://orm.drizzle.team/docs/migrations)

### Inferences
- With Turso Cloud the typical split is:
  - **Single DB:** `drizzle-kit migrate` in CI before deploying app code, which avoids startup races.
  - **DB per tenant:** a fleet migrator loops over the registry calling `migrate()` per DB, records per-tenant version in each DB's `__drizzle_migrations`, retries laggards, and requires app code to tolerate tenants on N-1 schema (expand–contract).
  - **Synced/embedded clients:** whoever writes DDL pushes it as logical DDL (section 2). Running Drizzle's migrator on each client against a synced DB would replay `CREATE … IF NOT EXISTS`, and the duplicate `__drizzle_migrations` INSERTs would also sync. That combination looks fragile, so the safer course is to migrate only the authoritative remote and let clients pull.
- Turso's deprecation of auto-propagating schema DBs is a signal. Server-side fan-out of DDL across many DBs was hard to make reliable, and an explicit, versioned, idempotent per-DB migrator with a registry is the pattern they now recommend.

### Gaps
- I found no official Turso or Drizzle guidance on running Drizzle migrations against a Turso *Sync* database, local versus remote.
- I found no replacement feature announced for multi-DB schemas as of October 2026.

---

## 8. Schema changes across many databases and replicas, and concurrency when several app instances start and migrate at once

### Takeaway
Neither Drizzle (SQLite/libSQL) nor Turso provides a migration lock. Correctness under simultaneous startup comes only from SQLite's single-writer serialization plus luck: the second runner usually fails on non-idempotent DDL. For replicas and synced copies, schema propagates physically from the primary on pull, or logically as replayed DDL on push. Across many databases, Turso explicitly leaves orchestration to the user.

### Cited Findings
- **Drizzle:** `migrate()` reads history outside the applying transaction and has no lock (issue #874 open since 2023). The only fix in flight is Postgres-only (PR #6451, using `pg_advisory_xact_lock`). — [drizzle-orm#874](https://github.com/drizzle-team/drizzle-orm/issues/874), [PR #6451](https://github.com/drizzle-team/drizzle-orm/pull/6451)
- **Turso engine:** "Each connection can have at most one active top-level transaction". Non-concurrent `BEGIN IMMEDIATE`/`DEFERRED` "prevents any concurrent commits" in MVCC mode. DDL such as CREATE INDEX is not allowed in `BEGIN CONCURRENT`. — [Transactions docs](https://docs.turso.tech/sql-reference/statements/transactions), [Turso manual](https://github.com/tursodatabase/turso/blob/main/docs/manual.md), [Concurrent Writes in Practice](https://turso.tech/blog/concurrent-writes-in-practice)
- **Embedded Replicas (libSQL):** replicas pull WAL frames from the primary; "One frame equals 4kB of data". Schema changes therefore reach replicas as physical frames on `sync()` or periodic `syncInterval`. — [Embedded Replicas docs](https://docs.turso.tech/features/embedded-replicas/introduction)
- **Turso Sync:** DDL pushed from a client is replayed with CREATE rewritten to IF NOT EXISTS, to tolerate "another client pushed its own version of the DDL first". — [database_sync_operations.rs](https://github.com/tursodatabase/turso/blob/main/sync/engine/src/database_sync_operations.rs)
- **Token scopes:** tokens can omit `schema_add`/`schema_update`/`schema_delete`, so client devices can be prevented from performing DDL. — [Turso Sync launch](https://turso.tech/blog/introducing-databases-anywhere-with-turso-sync)
- **Fleet migrations:** "You own the migration workflow, including version tracking, retries, and handling databases that are temporarily behind." — [Multi-tenancy at Scale](https://turso.tech/blog/multi-tenancy-at-scale)
- **Same pattern elsewhere:**
  - The advisory-lock library's README cites "running a database migration at server startup, where multiple processes will simultaneously try to run it" as the classic case. — [advisory-lock README (search result)](https://github.com/olalonde/advisory-lock/blob/master/README.md)
  - Knex's table-based lock broke when three nodes ran `migrate.latest` on an uninitialized DB, inserting several lock rows. — [knex#3538 (search summary)](https://github.com/knex/knex/issues/3538)

### Inferences
- For bumbledb on S3, where there is no server and no advisory locks, the natural equivalents are:
  - (a) A lease/lock object written with S3 conditional writes (`If-None-Match: *` to create, `If-Match: <etag>` to renew or release), with an expiry so a crashed migrator does not wedge deploys (the Ecto/Prisma timeout analogue).
  - (b) Commit the migration as a compare-and-swap on the database manifest/root pointer, so that "read history → apply → record" is one atomic CAS. A loser re-reads, finds the migration recorded, and becomes a no-op. This is strictly better than Drizzle's read-outside-transaction design.
  - (c) Record migrations in the ledger by stable ID plus checksum, inside the same atomic commit as the schema change.
- Apply-on-startup across many instances is safe only if the apply step is idempotent under CAS. Even then, prefer a single deploy-time migrator (CI job or init container) and have app instances merely *check* the schema version and refuse to start, or run in compatibility mode, if the DB is ahead or behind beyond their supported window.

### Gaps
- I found no documentation of Turso Cloud server-side behaviour when two clients concurrently push conflicting DDL (for example, both ALTER the same table), beyond the IF NOT EXISTS rewrite for CREATE.

---

## 9. Comparable patterns: Prisma migrate deploy, Rails/Ecto migration locks, expand–contract for rolling deploys

### Takeaway
Mature migrators add three things Drizzle lacks:
- A database-level mutex: Prisma's 10 s advisory lock, Rails' advisory lock that raises `ConcurrentMigrationError`, and Ecto's table lock or `pg_advisory_lock` with retries.
- A durable record of failed or partial migrations that blocks further deploys until resolved (Prisma).
- A deploy-time CI step as the recommended runner.

Expand–contract (parallel change) is the standard way to keep old and new code working during rolling deploys.

### Cited Findings
- **Prisma (v6/v7 docs):** "Prisma Migrate makes use of advisory locking when you run production commands such as" `migrate deploy`/`dev`/`resolve`. "Advisory locking has a 10 second timeout (not configurable) … purely to avoid catastrophic errors - if your command times out, you will need to run it again." It can be disabled with `PRISMA_SCHEMA_DISABLE_ADVISORY_LOCK` since 5.3.0. "migrate deploy should generally be part of an automated CI/CD pipeline". — [Prisma v7 docs: Development and production](https://www.prisma.io/docs/orm/v7/prisma-migrate/workflows/development-and-production)
- **Prisma failed migrations:** `_prisma_migrations` rows have a `logs` column storing the error. "Until you recover from the failed state, further migrations using prisma migrate deploy are impossible." Recovery uses `prisma migrate resolve --rolled-back` or `--applied`. — [Prisma patching & hotfixing](https://www.prisma.io/docs/orm/prisma-migrate/workflows/patching-and-hotfixing)
- **Prisma ORM 8 (newer; supersedes `migrate deploy` in the current docs):** "`npx prisma db migrate` … replaces Prisma ORM 7's `prisma migrate deploy` … PostgreSQL and MongoDB, with SQLite still experimental."
  - The DB stores "a marker, which is the record of which contract state that database matches". Migrations are edges between hashed contract states.
  - It fails with `MIGRATION.MARKER_MISMATCH` when the marker is outside known history, and `MIGRATION.PATH_UNREACHABLE` when no path exists.
  - On PostgreSQL "the whole run is one transaction", so a failed run leaves no changes. MongoDB runs are not transactional. — [Prisma docs (current): Development and production](https://www.prisma.io/docs/orm/prisma-migrate/workflows/development-and-production)
- **Rails:** `Migrator#run`/`#migrate` wrap work in `with_advisory_lock` when `connection.advisory_locks_enabled?`.
  - The lock ID is `MIGRATOR_SALT (2053462845) * db_name_hash`.
  - Failure to acquire raises `ConcurrentMigrationError`: "Cannot run migrations because another migration process is currently running." — [Rails migration.rb](https://github.com/rails/rails/blob/main/activerecord/lib/active_record/migration.rb)
  - The original commit's rationale: advisory locks are session-scoped, so a crashed migrator releases its lock automatically. — [Rails advisory-lock commit (mirror, search summary)](https://demo.gitea.com/TEST_CODEOWNERS/rails/commit/2c2a8755460ec3d32ece91c9766dbd0304ece028)
- **Ecto:** "By default, Ecto will lock the migration source to throttle multiple nodes to run migrations one at a time."
  - Postgres `:table_lock` wraps each migration, including the version insert, in a transaction.
  - `:pg_advisory_lock` allows concurrent operations such as `CREATE INDEX CONCURRENTLY`. It retries every 5 s, "infinity times" by default (`:migration_advisory_lock_retry_interval_ms`, `:migration_advisory_lock_max_tries`). — [Ecto.Adapters.Postgres](https://ecto-sql.hexdocs.pm/Ecto.Adapters.Postgres.html), [Ecto.Migration](https://hexdocs.pm/ecto_sql/Ecto.Migration.html)
- **Expand–contract / parallel change:** "Parallel change, also known as expand and contract … three distinct phases: expand, migrate, and contract … allows your code to be released in any of these three phases." — [Martin Fowler, ParallelChange](https://martinfowler.com/bliki/ParallelChange.html)
  - Prisma's Data Guide steps: (1) deploy the new schema alongside the old; (2) expand the client interface to dual-write; (3) migrate or backfill existing data; (4) test; (5) cut reads over (still dual-writing); then contract. — [Prisma Data Guide: expand and contract](https://www.prisma.io/dataguide/types/relational/expand-and-contract-pattern)

### Inferences
- A good bumbledb design combines:
  - Prisma's sticky failed-migration state and explicit `resolve`.
  - Ecto/Prisma lock-with-timeout semantics, implemented as an S3 lease object with conditional writes.
  - Drizzle v1's name-keyed ledger plus a verified checksum.
  - Prisma 8's idea of a DB-stored marker of the schema state, so that app code can check compatibility at startup.
  - Expand–contract discipline, so N and N+1 app versions both run against the post-expand schema during rolling deploys and on mobile clients that update slowly.
- On SQLite-family engines (SQLite, libSQL, Turso, bumbledb) DDL is transactional. A whole-run single transaction (Drizzle, Prisma 8 on PG) is feasible and gives clean failure semantics. The cost is that one bad migration blocks the whole batch.

### Gaps
- I did not fetch Ecto's SQLite adapter (`ecto_sqlite3`) docs to confirm how its migration lock behaves on SQLite.
- I did not confirm Prisma 8's locking behaviour in `db migrate`; the current docs page did not mention advisory locks.
