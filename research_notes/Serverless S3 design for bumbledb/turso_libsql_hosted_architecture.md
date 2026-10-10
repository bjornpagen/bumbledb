# How Turso/libSQL makes SQLite a fast hosted/serverless database (libSQL server era to Turso Cloud, as of 2026-10-09)

Era map. Read this first, because "Turso" means different things at different dates:
- **2022 to 2024, the libSQL-server ("sqld") era.** An open-source Rust server wraps the libSQL C fork of SQLite. It has primary/replica roles, gRPC WAL-frame replication, "bottomless" async S3 backup, and embedded replicas. Hosting ran on Fly.io, with a small VM per user that scaled to zero. Code: github.com/tursodatabase/libsql, which is still maintained (commits as late as 2026-08-23 per the GitHub API).
- **Oct 2024 to Mar 2025, the new closed-source server.** It is massively multi-tenant, built on Deterministic Simulation Testing (DST), and runs on AWS. The design is "diskless": the WAL is written to S3 Express One Zone before the commit is acknowledged, and checkpoints go to S3 Standard. It is wire-compatible with libSQL ([2025-01-21 roadmap](https://turso.tech/blog/upcoming-changes-to-the-turso-platform-and-roadmap)).
- **Dec 2024 onward, the "Turso Database" engine.** Originally named Limbo, it is a full Rust rewrite of SQLite. It brings async I/O, CDC-based "Turso Sync" with push()/pull() (Oct 2025), and MVCC `BEGIN CONCURRENT` (on Turso Cloud since 2026-08-03).
- **2026-10-02:** Supabase announced it is acquiring Turso ([Turso blog](https://turso.tech/blog/turso-is-joining-supabase)).

Source caveats:
- `docs/DESIGN.md` in the libsql repo is the original 2022 design. It describes a PostgreSQL-wire-protocol client and an optional mvSQLite/FoundationDB backend. **Superseded.** Treat only its primary/replica/write-delegation description as still valid.
- Many 2023–2024 Turso blog posts (embedded replicas, Lambda) now render only a banner: "This post references an older version of Turso". Their original bodies were not retrievable. Example: [Lambda post, 2023-11-21](https://blog.turso.tech/get-microsecond-read-latency-on-aws-lambda-with-local-databases-479db00a).

---

## Q1. sqld / libsql-server architecture: primary and replica roles, write delegation, single-writer model, namespaces / multi-tenancy

### Takeaway
sqld is a single-writer design built on stock SQLite semantics:
- One **primary** per database owns all writes and keeps a replication log of WAL frames.
- **Replicas** execute reads locally. They proxy any write (or any transaction that turns into a write) to the primary over a gRPC "proxy" stream. They pull WAL frames from the primary to stay fresh.
- Multi-tenancy uses **namespaces**: many independent SQLite databases inside one sqld process. The database is selected by an `x-namespace` header or by the Host subdomain.

Turso's hosted service later replaced sqld with a new closed-source, massively multi-tenant server. That server hosts "millions" of SQLite files per node and keeps the same client protocols.

### Cited Findings
**Primary and replica roles**
- The primary "is responsible for accepting writes and servicing replicas for write-ahead log (WAL) updates." When a client sends an INSERT to a replica, the replica delegates the write to the primary. SELECTs run on the replica directly. Replicas "poll the primary instance for WAL updates periodically over a gRPC connection" — [libsql USER_GUIDE.md](https://github.com/tursodatabase/libsql/blob/main/docs/USER_GUIDE.md)
- Cluster setup in the guide: the primary serves SQL over HTTP on :8081 and gRPC+TLS on :5001. The replica is started with `--primary-grpc-url`. In TLS terms the primary is the server and the replicas are clients — [USER_GUIDE.md](https://github.com/tursodatabase/libsql/blob/main/docs/USER_GUIDE.md)
- Original 2022 design (superseded): replicas "only serve reads locally, and delegate writes to the primary". The client originally spoke PostgreSQL wire protocol, with an optional mvSQLite/FoundationDB backend for "improved write concurrency". That backend never became the production path — [DESIGN.md](https://github.com/tursodatabase/libsql/blob/main/docs/DESIGN.md)

**Write delegation mechanics**
- `proxy.proto` defines the delegation messages:
  - `Program`: a list of `Step`s, each with an optional `Cond` (ok/err/not/and/or/is_autocommit). This lets a whole batch or transaction run as one remote program.
  - `ExecuteResults` returns the transaction `State` (INIT/INVALID/TXN) and `current_frame_no`, described as "Primary frame_no after executing the request".
  - Error codes include `TX_BUSY` and `TX_TIMEOUT`.
  - Source: [proxy.proto](https://github.com/tursodatabase/libsql/blob/main/libsql-replication/proto/proxy.proto)
- In the replica's `write_proxy.rs`:
  - `execute_remote` increments a `write_requests_delegated` stat. It holds a persistent remote connection, i.e. a stream to the primary, for the lifetime of the client connection.
  - It records `last_write_frame_no` from the primary's response.
  - `wait_replication_sync()` blocks later reads until the local replicator has applied that frame_no, or an explicit client-supplied `replication_index`.
  - Source: [write_proxy.rs](https://github.com/tursodatabase/libsql/blob/main/libsql-server/src/connection/write_proxy.rs)
- `should_proxy()`: when a replica namespace is loaded and has applied nothing yet, or is behind the primary's replication index from handshake time, the request is **proxied unconditionally to the primary** until the replica catches up — [write_proxy.rs](https://github.com/tursodatabase/libsql/blob/main/libsql-server/src/connection/write_proxy.rs)

**Single-writer model and consistency**
- sqld inherits SQLite semantics: "All operations occurring on the primary are linearizable." A connection is guaranteed to see its own writes. Reads on a replica are monotonic. There is no global ordering, and two processes on the same replica may read different points in time — [CONSISTENCY_MODEL.md](https://github.com/tursodatabase/libsql/blob/main/docs/CONSISTENCY_MODEL.md)
- libSQL keeps SQLite's one-writer-many-readers WAL model. Turso says SQLite reaches ~150k rows/s with batched inserts and fsync on every commit, but "adding more threads does nothing" — [Turso, 2025-10-06](https://turso.tech/blog/beyond-the-single-writer-limitation-with-tursos-concurrent-writes)
  - The same post elsewhere says "500k rows per second with proper batching". The two figures are internally inconsistent and probably come from different settings.

**Namespaces / multi-tenancy**
- `namespace_from_headers()` resolves the namespace in this order: a configured default, then the `x-namespace` header, then the `Host` header subdomain via `split_namespace` — [db_factory.rs](https://github.com/tursodatabase/libsql/blob/main/libsql-server/src/http/user/db_factory.rs)
- Per-namespace config is stored in a meta store and shipped over the wire as `DatabaseConfig`. Fields: `block_reads`/`block_writes` with a reason, `max_db_pages`, `bottomless_db_id`, `jwt_key`, `txn_timeout_s`, `allow_attach`, `max_row_size`, `shared_schema`, and `durability_mode` (enum RELAXED/STRONG/EXTRA/OFF) — [metadata.proto](https://github.com/tursodatabase/libsql/blob/main/libsql-replication/proto/metadata.proto)
- The hosted model through 2024: "our control plane provisions a small VM for you. Inside that VM you can create a lot of SQLite files … if you don't access any of your databases after a while, your VM scales to zero" — [Glauber Costa, 2024-10-16](https://turso.tech/blog/a-deep-look-into-our-new-massive-multitenant-architecture)
  - Isolation between databases "is good but it is not perfect", hence the two-layer VM + namespaces approach.

**Why they rewrote the server (Oct 2024)**
- A customer created ~5,000 databases in a few hours and triggered a deadlock. The cause was a sync mutex held across a hidden `block_on` inside Tokio. It "took us weeks" to find — [2024-10-16 post](https://turso.tech/blog/a-deep-look-into-our-new-massive-multitenant-architecture)
- They rewrote the server from scratch:
  - DST, plus the Antithesis hypervisor.
  - A hand-written callback event loop in sync Rust, instead of Tokio. They cite allocation/lifetime costs and nondeterminism of async Rust.
  - They state the goal is to "codify that excessive resource consumption case are bugs".
  - Source: [2024-10-16 post](https://turso.tech/blog/a-deep-look-into-our-new-massive-multitenant-architecture)
- The new server is closed source. Turso says "this is an entirely new implementation, not a relicensing of libSQL's server components". The new server "uses the same protocols", so self-hosters can still use libSQL — [roadmap, 2025-01-21](https://turso.tech/blog/upcoming-changes-to-the-turso-platform-and-roadmap)
- On scale: "a single Turso compute node can host millions of [databases] with no cold start" — [AWS Storage Blog, 2026-07-23](https://aws.amazon.com/blogs/storage/how-turso-built-a-transactional-database-using-amazon-s3-express-one-zone/)
  - The same post describes "hundreds of active SQLite databases" per node.
  - The diskless post says "millions of SQLite files per node … most databases would have to be inactive" — [Turso, 2025-04-07](https://turso.tech/blog/turso-cloud-goes-diskless)
- MVCC `BEGIN CONCURRENT` (Turso engine, Hekaton-inspired, row-level conflict detection at commit) became available in early preview on Turso Cloud on 2026-08-03 — [Turso, 2026-08-03](https://turso.tech/blog/concurrent-writes-on-turso-cloud)
  - Claimed 4x SQLite throughput at 8 threads with 1 ms of compute per transaction, and +16% with no compute — [Turso, 2026-08-27](https://turso.tech/blog/concurrent-writes-in-practice)
  - Writes go to the in-memory MVCC index, then to a log file, and are eventually checkpointed into the B-tree via the WAL — [2026-08-27](https://turso.tech/blog/concurrent-writes-in-practice)

### Inferences
- The essential pattern is: **single writer per database, plus delegation of writes, plus frame_no-based read-your-writes**. For an LMDB-backed engine, which is also single-writer, this maps directly:
  - The primary returns its post-commit txn id or log sequence number (LSN).
  - A replica, or embedded client, blocks reads on that connection until its applied LSN is at least that value.
- `should_proxy()` is a cheap trick that matters for serverless. A freshly started, cold replica serves reads by proxying to the primary while it catches up, rather than serving stale data or blocking.
- Database-per-tenant with header- or subdomain-based routing inside one process is what made "unlimited databases" pricing possible. Per-database overhead is a file plus some metadata, not a process or a VM.

### Gaps
- No public internals of the new closed-source multi-tenant server: scheduling, per-database memory limits, how idle databases are unloaded. Only blog-level descriptions exist.
- The semantics of `DurabilityMode` RELAXED/STRONG/EXTRA/OFF in libsql-server were not verified. The mapping to fsync or bottomless waits is unknown.

---

## Q2. The replication protocol: WAL frames, frame numbers, generations, snapshots, sync endpoints, catch-up, consistency tokens

### Takeaway
libSQL replication is **physical, page-level log shipping**.
- The primary wraps SQLite's WAL and appends every committed page to its own replication log (`wallog`). Each frame is a 24-byte header plus one 4 KiB page, carrying a monotonically increasing `frame_no` and a rolling CRC-64.
- Replicas handshake to learn a **generation** (a UUID that changes on each primary restart) and the current replication index. They then stream frames from their last applied `frame_no`.
- If the log has been compacted past that point, the server answers NEED_SNAPSHOT. The replica then pulls a **snapshot**: a deduplicated set holding the latest version of each page, in descending frame order. After that it resumes streaming.
- A newer HTTP "sync" protocol (`/info`, `/export/{gen}`, `/sync/{gen}/{start}/{end}`) supports embedded replicas with offline writes, using push and pull of frames.
- Consistency is read-your-writes via frame_no / `replication_index` tokens.

### Cited Findings
**gRPC service**
- `ReplicationLog` has four RPCs: `Hello`, `LogEntries` (server stream), `BatchLogEntries`, and `Snapshot` (stream) — [replication_log.proto](https://github.com/tursodatabase/libsql/blob/main/libsql-replication/proto/replication_log.proto)
- `HelloResponse` carries:
  - `generation_id`: the UUID of the current generation.
  - `generation_start_index`: the first frame_no in the generation.
  - `log_id`.
  - `session_token`: changes on each restart and must be sent in later headers, otherwise the server returns `NO_HELLO`.
  - `current_replication_index`.
  - the namespace `DatabaseConfig`.
  - Source: [replication_log.proto](https://github.com/tursodatabase/libsql/blob/main/libsql-replication/proto/replication_log.proto)
- `LogOffset{next_offset, wal_flavor}` is the request cursor. `Frame{data, timestamp (commit time on commit frames), durable_frame_no}` is the reply unit. `durable_frame_no` appears to come from the newer libsql-wal/storage work — [replication_log.proto](https://github.com/tursodatabase/libsql/blob/main/libsql-replication/proto/replication_log.proto)
- Server side: `MAX_FRAMES_PER_BATCH = 1024` for `BatchLogEntries`. Errors map to `NEED_SNAPSHOT_ERROR_MSG` and `NO_HELLO_ERROR_MSG` — [rpc/replication/replication_log.rs](https://github.com/tursodatabase/libsql/blob/main/libsql-server/src/rpc/replication/replication_log.rs)

**Frame and log formats**
- `FrameHeader` (little-endian) has four fields:
  - `frame_no: u64`, which is incremental.
  - `checksum: u64`, a "rolling checksum of all the previous frames, including this one".
  - `page_no: u32`.
  - `size_after: u32`, the database size in pages after commit. It "serves as commit transaction boundary".
  - Source: [libsql-replication/src/frame.rs](https://github.com/tursodatabase/libsql/blob/main/libsql-replication/src/frame.rs)
- Log file header: magic `b"SQLDWAL\0"`, `start_checksum` (CRC_64_GO_ISO), `log_id` (UUID), `start_frame_no`, `frame_count`, version 2, page size 4096 — [logger.rs](https://github.com/tursodatabase/libsql/blob/main/libsql-server/src/replication/primary/logger.rs)
- `ReplicationLogger` holds a `Generation{id: Uuid::new_v4(), start_index}`. The generation is created when the logger starts (i.e. on process start/restart), starting from the log's last frame_no — [logger.rs](https://github.com/tursodatabase/libsql/blob/main/libsql-server/src/replication/primary/logger.rs)
- The logger also has a `new_frame_notifier` watch channel. Subscribers use it for long-poll / streaming on new frames — [logger.rs](https://github.com/tursodatabase/libsql/blob/main/libsql-server/src/replication/primary/logger.rs)

**Log compaction into snapshots**
- Compaction triggers when `frame_count > max_log_frame_count` or `max_log_duration` has elapsed, and only when there are no uncommitted frames. The old log is moved to `to_compact/` and the compactor turns it into a snapshot — [logger.rs](https://github.com/tursodatabase/libsql/blob/main/libsql-server/src/replication/primary/logger.rs)
- Defaults: `max_log_size` is 200 MB; `max_log_duration` defaults to None — [config.rs](https://github.com/tursodatabase/libsql/blob/main/libsql-server/src/config.rs)
- A snapshot "contains the most recent version of each page, in descending frame_no order". Duplicate pages are dropped using a `seen_pages` HashSet. Snapshot files are named by `(log_id, start_frame_no, end_frame_no)` — [snapshot.rs](https://github.com/tursodatabase/libsql/blob/main/libsql-server/src/replication/snapshot.rs)
- Snapshots are merged again when `SNAPHOT_SPACE_AMPLIFICATION_FACTOR = 2` (snapshot bytes greater than 2x the DB size) or when `MAX_SNAPSHOT_NUMBER = 32` — [snapshot.rs](https://github.com/tursodatabase/libsql/blob/main/libsql-server/src/replication/snapshot.rs)

**Replica catch-up**
- The replica's state machine runs NeedHandshake, then NeedFrames, then NeedSnapshot when it gets `Error::NeedSnapshot`. It also handles `SnapshotPending` (retry later) and `NamespaceDoesntExist` — [libsql-replication/src/replicator.rs](https://github.com/tursodatabase/libsql/blob/main/libsql-replication/src/replicator.rs)
- Frames are applied through an "injector": a custom WAL that writes received frames straight into the replica's SQLite WAL and commits at frame boundaries — [replicator.rs](https://github.com/tursodatabase/libsql/blob/main/libsql-replication/src/replicator.rs)
- Offline incremental snapshots: sqld can emit incremental snapshot files and run a `--snapshot-exec` hook on each one. You can ship those files to another machine and apply them with `Database::sync_frames()`. `--max-log-duration` controls snapshot cadence — [USER_GUIDE.md](https://github.com/tursodatabase/libsql/blob/main/docs/USER_GUIDE.md)

**HTTP "sync" protocol (embedded replicas, offline writes)**
- `GET {sync_url}/info` returns `current_generation`. `GET /export/{generation}` downloads the whole DB file to bootstrap (`bootstrap_db`) — [libsql/src/sync.rs](https://github.com/tursodatabase/libsql/blob/main/libsql/src/sync.rs)
- Pull uses `/sync/{generation}/{frame_no}/{frame_no+pull_batch}`. A pull can return `EndOfGeneration{max_generation}`. Push uses `/sync/{generation}/{start}/{end}[/{baton}]`. Default push/pull batches are 128 frames; default max retries is 5 — [sync.rs](https://github.com/tursodatabase/libsql/blob/main/libsql/src/sync.rs)
- The server replies with `durable_frame_num`, which the client tracks as `durable_generation` / `durable_frame_num`:
  - A reply lower than what was pushed is `InvalidPushFrameNoLow`; the client re-pushes from that point.
  - A higher reply is an error.
  - A conflict is `InvalidPushFrameConflict`.
  - Source: [sync.rs](https://github.com/tursodatabase/libsql/blob/main/libsql/src/sync.rs)
- Hrana 3 (the client SQL-over-HTTP protocol) uses a `baton` "similar to a session cookie" to pin a stream or transaction across stateless HTTP requests. The server returns a new, unpredictable baton each time — [HRANA_3_SPEC.md](https://github.com/tursodatabase/libsql/blob/main/docs/HRANA_3_SPEC.md)

**Consistency tokens**
- The HTTP query type has an optional `replication_index: Option<u64>`. It is passed into `execute_batch_or_rollback` and the replica waits until it has applied that index — [types.rs](https://github.com/tursodatabase/libsql/blob/main/libsql-server/src/http/user/types.rs), [http/user/mod.rs](https://github.com/tursodatabase/libsql/blob/main/libsql-server/src/http/user/mod.rs), [write_proxy.rs](https://github.com/tursodatabase/libsql/blob/main/libsql-server/src/connection/write_proxy.rs)
- The embedded-replica client updates `max_write_replication_index` from each remote write's `current_frame_no` — [libsql/src/replication/connection.rs](https://github.com/tursodatabase/libsql/blob/main/libsql/src/replication/connection.rs)
  - It also records `last_handshake_replication_index` from `Hello` — [remote_client.rs](https://github.com/tursodatabase/libsql/blob/main/libsql/src/replication/remote_client.rs)

### Inferences
**What bumbledb can copy**
- A self-describing frame: (LSN, rolling checksum, page id, db-size-after as the commit marker). The cursor `(generation_uuid, next_lsn)` is enough to resume, and a generation change means "your cursor is meaningless, re-snapshot".
- LMDB has no WAL. Its COW B+tree writes new pages and flips the meta page. A bumbledb analogue would capture the set of dirty pages per write txn, plus the new meta page, as one "frame batch" keyed by LMDB txnid.

**What to avoid**
- Page-granularity shipping amplifies writes: a 1-byte row costs a 4 KB frame, and B-tree splits emit many frames (see Q3/Q7).
- Generation churn on every primary restart forces replicas to re-snapshot.
- Turso eventually abandoned page-level push for logical CDC (Q7).

### Gaps
- The exact on-disk format and storage layer of the libsql-wal crate (segments, `durable_frame_no`) and whether Turso Cloud ever ran it in production. docs.rs only says it is "a linked list of segments… head segment is sealed and becomes immutable", depending on `aws-sdk-s3` and `fst` — [docs.rs/libsql-wal](https://docs.rs/libsql-wal)
- No published replica lag or catch-up throughput numbers for sqld gRPC replication.

---

## Q3. Embedded replicas: local SQLite file plus sync(), periodic sync, offline reads, local read vs remote write latency, serverless/edge usage and cold start

### Takeaway
An embedded replica is a local SQLite file that the client library keeps in sync with the cloud primary by pulling frames.
- **Reads** run locally ("microseconds").
- **Writes** go over the network to the primary by default. The local file is then updated automatically to give read-your-writes.
- Sync is manual (`sync()`) or periodic (`syncInterval`). An `offline: true` mode lets writes go to the local file and be pushed later.
- Cold start means downloading the whole database first. Partial or lazy bootstrap only arrived with the Turso-engine "Turso Sync" (experimental).
- Turso itself now calls embedded replicas "the earlier approach" and recommends Turso Sync, which pushes logical row changes and pulls physical pages.

### Cited Findings
**How embedded replicas behave**
- "Reads run locally from the file in microseconds, and writes are sent to the cloud primary and then reflected back to the replica." Writes "are NOT written to the local file first" unless `offline: true` is set. Write transactions that contain reads are also sent to the primary — [docs.turso.tech embedded replicas](https://docs.turso.tech/features/embedded-replicas/introduction)
- Periodic sync uses `syncInterval` (the docs example uses 60 s; the PHP/Laravel examples use 300 s). Manual `sync()` can run "every 5 minutes or every time the application starts" — [docs](https://docs.turso.tech/features/embedded-replicas/introduction)
- "After a write returns successfully, the replica that initiated the write will always be able to see the new data right away, even if it never calls sync()." Other replicas see it on their next sync — [docs](https://docs.turso.tech/features/embedded-replicas/introduction)

**Documented caveats**
- Do not open the local DB while it is syncing (risk of corruption).
- Embedded replicas cannot be used "in serverless environments without a filesystem".
- A B-tree split "would cause many new frames"; a dirty server restart "would regenerate the replication log and sync additional frames"; and "One frame equals 4kB … if you write a 1 byte row, it will always show up as a 4kB write".
- Source: [docs](https://docs.turso.tech/features/embedded-replicas/introduction)

**Client internals**
- The client runs a background periodic sync task (`perodic_sync: Option<Duration>`). `sync_oneshot()` returns a `Replicated{frame_no, frames_synced}`. `sync_frames(Frames)` lets the application inject frames it obtained elsewhere — [libsql/src/replication/mod.rs](https://github.com/tursodatabase/libsql/blob/main/libsql/src/replication/mod.rs)

**Cold start and bootstrap**
- Bootstrap pulls the entire DB via `/export/{generation}` when no local file exists — [sync.rs](https://github.com/tursodatabase/libsql/blob/main/libsql/src/sync.rs)
- "for libSQL Embedded Replicas you always have to sync the entire database first. Turso allows you to lazy load (because everything in Turso is async)" — [Glauber Costa, 2026-04-24](https://turso.tech/blog/sync-benchmark)
- Turso Sync (Turso engine):
  - Bootstraps from the remote on first run. Partial sync is "experimental" and can bootstrap "from a byte prefix of the database or from the pages touched by a query".
  - Pull supports long-polling (`longPollTimeoutMs`).
  - Local WAL is retained to "rebase" local changes after a pull. A sync-aware `checkpoint()` exists.
  - Source: [Turso local-first guide, 2026-07-27](https://turso.tech/blog/building-local-first-apps-the-complete-guide-to-offline-first-database-sync)
- Turso Sync protocol (announced 2025-10-08): "Local changes are pushed to the remote as logical mutations"; "Remote changes are pulled as physical pages, ensuring the local replica eventually becomes byte-for-byte identical". Conflict resolution is row-level Last-Push-Wins by default, with a `transform` hook to rewrite mutations — [Introducing Databases Anywhere with Turso Sync](https://turso.tech/blog/introducing-databases-anywhere-with-turso-sync)

**Serverless and Lambda usage**
- Turso published a 2023 post on microsecond reads in AWS Lambda using local databases. Its body is now replaced by an "older version of Turso" banner and could not be read — [blog, 2023-11-21](https://blog.turso.tech/get-microsecond-read-latency-on-aws-lambda-with-local-databases-479db00a)
- Turso's 2026 framing: "Backend services can use local databases for microsecond read latency … reads from local disk instead of making a network call on every request" — [local-first guide](https://turso.tech/blog/building-local-first-apps-the-complete-guide-to-offline-first-database-sync)

**Edge replicas (the server-side counterpart) are deprecated**
- "70% of Turso users never create geographical replicas … syncing the SQLite file itself to your API or device is often a better approach than edge replication". Edge replicas were discontinued for new users on 2025-01-21 — [roadmap](https://turso.tech/blog/upcoming-changes-to-the-turso-platform-and-roadmap), [Data Edge docs](https://docs.turso.tech/features/data-edge)

### Inferences
- For a serverless function, an embedded replica only helps if the function has a writable filesystem and either a warm, reused instance or a small database. Full-file bootstrap makes cold-start cost scale with DB size.
- Lazy page fetch (partial sync, or "pages touched by a query") is the feature that makes "replica in a function" viable for large DBs. bumbledb on S3 would want page- or segment-addressable objects so a cold function can fetch only its working set.
- Read-your-writes via a write token is cheap and avoids forcing a full sync after every write. Turso's own benchmark shows that **auto-pull after each write** (readYourWrites on) cost ~218–243 ms per cycle over a public network (Q6). That is a cost to design around.

### Gaps
- No official numbers for embedded-replica read latency beyond "microseconds", and no p50/p99 published.
- No official cold-start (bootstrap) timings versus DB size for embedded replicas. The only data point is 5,000 rows, about 1 MB: 718 ms with embedded replicas vs 338 ms with Turso Sync ([sync benchmark](https://turso.tech/blog/sync-benchmark)).
- A third-party guide's advice to "plan for 15–50 ms per write" with embedded replicas appeared in search snippets but was not verified.

---

## Q4. Bottomless: how libSQL ships WAL and snapshots to S3; batching, compression, bucket layout, restore and PITR, durability semantics

### Takeaway
Bottomless is a libSQL **virtual WAL** that copies committed WAL frames to S3 **asynchronously**.
- Frames are batched by count (default 10,000) or time (default 15 s), compressed with zstd by default, and uploaded as objects named by frame range.
- Each "generation" (a time-reversed UUIDv7 prefix) starts with a compressed snapshot of the main DB file, taken at checkpoint. A `.dep` object links each generation to its parent.
- Restore finds the generation, walks parents to the nearest snapshot, then replays WAL batches up to a frame or timestamp. That gives point-in-time recovery.
- **Commits are acknowledged before S3.** Turso later called this out as the reason the old architecture "was not diskless". The one synchronous guard: a checkpoint must wait until frames are confirmed in S3, so the WAL is never truncated before backup.

### Cited Findings
**Basics and bucket layout**
- "This project implements a virtual write-ahead log (WAL) which continuously backs up the data to S3-compatible storage." Enabled in sqld with `--enable-bottomless-replication`. "All page writes committed to the database end up being asynchronously replicated to S3-compatible storage" — [bottomless README](https://github.com/tursodatabase/libsql/blob/main/bottomless/README.md), [libsql-server README](https://github.com/tursodatabase/libsql/blob/main/libsql-server/README.md)
- Bucket layout per the code comments:
  - Each generation lives under a `{db-name}-{uuid-v7}/` prefix.
  - `.meta` holds the page size and initial WAL checksum.
  - WAL batches are `{first-frame-no}-{last-frame-no}-{timestamp}.{raw|gz|zstd}`.
  - The DB snapshot is `db.{raw|gz|zstd}`.
  - Other objects: `.changecounter` and `.dep` (the parent generation).
  - Source: [bottomless/src/replicator.rs](https://github.com/tursodatabase/libsql/blob/main/bottomless/src/replicator.rs)
- Generation IDs are UUIDv7 built from a **reversed timestamp** (`253370761200 - seconds`). The comment says newest generations list "first in the S3-compatible bucket, under the assumption that fetching newest generations is the most common operation" — [replicator.rs](https://github.com/tursodatabase/libsql/blob/main/bottomless/src/replicator.rs)

**Batching and compression**
- Environment knobs, with defaults:
  - `LIBSQL_BOTTOMLESS_BATCH_INTERVAL_SECS` = 15
  - `LIBSQL_BOTTOMLESS_BATCH_MAX_FRAMES` = 10000
  - `LIBSQL_BOTTOMLESS_COMPRESSION` = zstd (gzip and raw also supported)
  - Further options: `s3_max_parallelism`, `s3_max_retries`, `verify_crc`, `skip_snapshot`, and encryption config.
  - Source: [replicator.rs](https://github.com/tursodatabase/libsql/blob/main/bottomless/src/replicator.rs)
- Upload loop: a `WalCopier` task wakes on a flush trigger or after the batch interval. It copies frames `(last_sent+1)..next_frame` to S3 and publishes `last_committed_frame_no` on a watch channel — [replicator.rs](https://github.com/tursodatabase/libsql/blob/main/bottomless/src/replicator.rs)
- A flush is also requested when `most_recent - last_sent >= max_frames_per_batch` — [replicator.rs](https://github.com/tursodatabase/libsql/blob/main/bottomless/src/replicator.rs)

**Durability semantics and the checkpoint guard**
- `insert_frames` writes to the local WAL first, then calls `replicator.submit_frames(n)` without waiting. This is asynchronous — [bottomless_wal.rs](https://github.com/tursodatabase/libsql/blob/main/bottomless/src/bottomless_wal.rs)
- In `checkpoint`:
  - Only TRUNCATE checkpoints are allowed; weaker ones return SQLITE_BUSY, to avoid partial checkpoints and autocheckpoint on close.
  - Before checkpointing, it calls `replicator.wait_until_committed(last_known_frame)`. If S3 has not confirmed the frames it times out with BUSY, and it skips the checkpoint if the previous generation is not yet snapshotted.
  - After checkpointing it snapshots the main DB file and starts a new generation.
  - Source: [bottomless_wal.rs](https://github.com/tursodatabase/libsql/blob/main/bottomless/src/bottomless_wal.rs)
- Turso, describing its pre-2025 hosted stack: "We've been storing backups on S3 for a long time. But writes to the database were returned to the client right away, with backups done asynchronously … there was a period when writes were not really durable anywhere except the local disk" — [Turso Cloud Goes Diskless, 2025-04-07](https://turso.tech/blog/turso-cloud-goes-diskless)

**Restore and PITR**
- `restore_from`:
  1. Uploads any leftover local files from a previous run.
  2. Finds the last consistent remote frame.
  3. Compares with the local DB, and does a full restore only if needed.
  4. Restores into a temp path, then atomically renames.
  - Source: [replicator.rs](https://github.com/tursodatabase/libsql/blob/main/bottomless/src/replicator.rs)
- `full_restore` walks the `.dep` parent chain back to a generation that has a snapshot (`MAX_RESTORE_STACK_DEPTH = 100`). It replays WAL batches generation by generation, optionally capped by a timestamp, which is PITR. The last generation is capped at the last consistent frame checked at the start, because it may still be being written — [replicator.rs](https://github.com/tursodatabase/libsql/blob/main/bottomless/src/replicator.rs)
- `latest_generation_before(timestamp)` lists keys by prefix to pick a generation for PITR. Tombstoned databases refuse restore — [replicator.rs](https://github.com/tursodatabase/libsql/blob/main/bottomless/src/replicator.rs)
- Boot behavior: if the DB file is empty, it is restored from remote. If the local file is newer, it is uploaded as a new generation. A newer local WAL is uploaded too. `bottomless-cli` supports `ls`, `restore`, and `rm --older-than` — [bottomless README](https://github.com/tursodatabase/libsql/blob/main/bottomless/README.md)

### Inferences
**Patterns worth copying for bumbledb on S3**
- Frame-range-named immutable objects.
- Time-reversed generation IDs, so "latest" is a 1-key LIST.
- A parent-pointer chain between generations.
- Restore into a temp file plus atomic rename.
- **Never truncate or reclaim log space until S3 has acknowledged it.** For LMDB, the analogue is not letting freed pages be reused before the corresponding txn batch is durable in S3. That requires holding a reader or snapshot, or keeping the txn's dirty pages separately.

**Anti-pattern**
- An async-only S3 copy leaves a window, up to the 15 s batch interval, where an acked commit exists only on local disk. Turso deemed this unacceptable for a diskless design (Q5).

### Gaps
- No published numbers on bottomless upload lag, restore time, or object counts in production.
- Bottomless's README still says "Work in heavy progress". The commit history shows activity into 2024–2025, but it is unclear whether Turso Cloud on Fly used exactly this code path through 2025.

---

## Q5. Turso Cloud's diskless / S3-backed architecture (2025–2026): what lives where, write-path latency budget, low p99 on object storage, cold start / scale-to-zero, pricing that reveals the design

### Takeaway
Since GA on AWS (2025-03-17), Turso Cloud is "diskless".
- **Write path:** every commit's WAL is written to **S3 Express One Zone before acknowledgement**.
- **Cross-tenant batching:** commits from many tenant databases on a node are batched into a **single PUT**, with a plan-dependent time window (up to 10/25/50/100 ms added).
- **Checkpoints:** frequent (about every 1 MB, or "every few minutes"). Results go to **S3 Standard** as 128 KB segments grouped into "generations" (versioned, so PITR and branching are metadata-only).
- **Local NVMe:** only a write-through and read cache with per-tenant quotas, with lazy segment fetch on a miss.
- **Pods are disposable.** A replacement serves immediately from S3 while it warms up.
- **No cold starts:** an always-up multi-tenant process holds millions of mostly idle database files. The result is unlimited databases per plan, with no per-database idle cost.

### Cited Findings
**Where data lives**
- Turso's durability doc says:
  - "Commits are only acknowledged once data is safely stored in either S3 or S3-express. Compute nodes can come and go at any time, and local disks act as a local cache."
  - Applies to "all Turso AWS regions", for users registered or upgraded after 2025-03-17.
  - Pro and Enterprise customers can use their own buckets.
  - Source: [Durability Guarantees docs](https://docs.turso.tech/cloud/durability)
- "the database file is split into 128kB segments. The collection of all the segments that comprise a database file is called a generation." Three stated reasons:
  - A new version reuses unchanged segments.
  - Moving or replicating needs only "the segments that are needed to serve the next query … The rest can be lazily fetched, or not at all".
  - A per-tenant local cache cap, e.g. "1GB will be available locally, while the rest is evicted, and brought back from S3 on-demand".
  - Source: [How does Turso Cloud keep your data durable and safe?, 2025-05-20](https://turso.tech/blog/how-does-the-turso-cloud-keep-your-data-durable-and-safe)
- "All writes that have happened since the last generation … are sent to S3 Express … Only after the data is safely stored in S3 Express, is the transaction acknowledged." — [2025-05-20](https://turso.tech/blog/how-does-the-turso-cloud-keep-your-data-durable-and-safe)
  - "The entire WAL has to be present on the local storage of the server in order for any query to be serviced. Because of that, the Turso Cloud servers will checkpoint often, usually after each 1MB written."
  - On failover: "the latest WAL is brought back from S3 Express, and the database segments are lazily fetched from S3."
- Checkpoint cadence varies across Turso's own descriptions:
  - "every couple of minutes" — [2025-04-07](https://turso.tech/blog/turso-cloud-goes-diskless)
  - "usually after each 1MB written" — [2025-05-20](https://turso.tech/blog/how-does-the-turso-cloud-keep-your-data-durable-and-safe)
  - "Every few minutes … stored in S3 Standard … using S3 Versioning" — [AWS blog, 2026-07-23](https://aws.amazon.com/blogs/storage/how-turso-built-a-transactional-database-using-amazon-s3-express-one-zone/)
  - These are probably a size-or-time trigger. The exact rule is not published.
- Each region has its own S3 bucket, for data residency — [2025-05-20](https://turso.tech/blog/how-does-the-turso-cloud-keep-your-data-durable-and-safe)

**Write path, step by step**
- Per the AWS blog:
  1. The transaction executes in memory.
  2. The node "waits a few milliseconds to batch WAL entries from multiple databases on the same node".
  3. A "single S3 PUT request" goes to S3 Express One Zone.
  4. On success, the transactions are acked.
  5. The data is also written to local NVMe.
  - Read path: NVMe cache, then S3 Express on a miss (single-digit ms), then cache the result.
  - Source: [AWS Storage Blog, 2026-07-23](https://aws.amazon.com/blogs/storage/how-turso-built-a-transactional-database-using-amazon-s3-express-one-zone/)
- "We may wait a couple of extra milliseconds even when we could start a new upload to S3 Express right away … accumulate transactions for a large number of databases." "The local disk becomes a write-through cache. Once data is on S3 Express, we also write it locally." — [2025-04-07](https://turso.tech/blog/turso-cloud-goes-diskless)

**Per-plan added commit latency (batching window ceiling)**

| Plan | Added commit latency ceiling |
|---|---|
| Free | up to 100 ms |
| Developer | up to 50 ms |
| Scaler | up to 25 ms |
| Pro and above | up to 10 ms |

- "The commit latency is not the expected latency in every commit, but the ceiling of added latency in each commit." The doc's worked example: a commit at the start of a 10 ms window waits the full 10 ms; one arriving 4 ms in waits 6 ms; one at the end waits 0 — [Durability docs](https://docs.turso.tech/cloud/durability)

**Cost reasoning (why batching is mandatory)**
- At the time of the post, a $4.99 plan allowed 25M rows written per month. With 1 row per transaction, that is $125/month in standard S3 PUTs ($0.005 per 1k) or ~$57/month on S3 Express ($0.0025 per 1k). Amortized over 100 active databases per node, that becomes "$0.57 per database per month" — [2025-04-07](https://turso.tech/blog/turso-cloud-goes-diskless)
- "Writing to two zones" costs the same as standard S3. Multi-AZ writes are a possible future step; S3 Express is single-AZ with a 99.95% uptime SLA — [2025-04-07](https://turso.tech/blog/turso-cloud-goes-diskless)

**Failure, recovery and BYOC**
- "If a pod suddenly dies, another one can be brought in immediately and start reading directly from S3. This isn't fast … at the expense of higher latencies while a background recovery process executes." Planned moves stream data into the new pod and switch when ready — [2025-04-07](https://turso.tech/blog/turso-cloud-goes-diskless)
- BYOC needs only "an S3 Express One Zone directory bucket, launching compute pods, and configuring … IAM policies", with no PersistentVolumes or StatefulSets — [AWS blog](https://aws.amazon.com/blogs/storage/how-turso-built-a-transactional-database-using-amazon-s3-express-one-zone/)

**PITR and branching**
- PITR = find "the latest generation before the specific timestamp … the WAL fragments written after that generation up to the specific timestamp". Branching is "a metadata-only operation" that shares generations and WAL fragments. PITR is offered up to 90 days — [2025-05-20](https://turso.tech/blog/how-does-the-turso-cloud-keep-your-data-durable-and-safe)

**No cold starts**
- "On our AWS regions, databases never have cold starts … every database is just a SQLite file, and the server itself is always up" — [AWS GA post, 2025-03-17](https://turso.tech/blog/turso-aws-out-of-beta)
- Free-tier users were migrated from Fly to AWS ("which has no cold starts!") — [roadmap, 2025-01-21](https://turso.tech/blog/upcoming-changes-to-the-turso-platform-and-roadmap)
- Current banner on old posts: "Because Turso databases are files — not processes — they never sleep, never cold-start, and don't need scale-to-zero" — [archived embedded-replica post](https://turso.tech/blog/local-first-cloud-connected-sqlite-with-turso-embedded-replicas)

**Pricing that reveals the design (turso.tech/pricing, fetched 2026-10-09)**

| Plan | Databases | Storage | Rows read/mo | Rows written/mo | Syncs/mo | PITR |
|---|---|---|---|---|---|---|
| Free ($0) | 100 | 5 GB | 500M | 10M | 3 GB | 1 day |
| Developer ($4.99/mo shown) | Unlimited | 9 GB, +$0.75/GB | 2.5B, +$1/B | 25M, +$1/M | 10 GB, +$0.35/GB | 10 days |
| Scaler ($24.92/mo shown) | Unlimited | 24 GB, +$0.50/GB | 100B, +$0.80/B | 100M, +$0.80/M | 24 GB, +$0.25/GB | 30 days |
| Pro ($416.58/mo shown) | Unlimited | 50 GB, +$0.45/GB | 250B, +$0.75/B | 250M, +$0.75/M | 100 GB, +$0.15/GB | 90 days |

- BYOC is Enterprise only. The prices shown carry "Save $X/month" labels, so they may be annual-billing rates — [Turso pricing](https://turso.tech/pricing)

### Inferences
**Where the low p99 comes from**
- S3 Express same-AZ PUT p99 is about 7 ms, versus 102 ms for a 4 KB standard-S3 PUT (Q6). Turso explicitly rejected standard S3 for the commit path.
- Batching across tenants turns per-commit PUT cost into per-batch cost.
- Plan tiers effectively sell **shorter batch windows**: paying more buys lower worst-case commit latency because your commits are flushed on a tighter timer. This is a very transferable pricing and design idea.

**Two-tier log / base structure**
- A hot WAL lives in a low-latency, single-AZ object store, and immutable 128 KB base segments live in cheap multi-AZ S3 with versioning.
- Frequent checkpoints keep the WAL small enough to replay on failover, and keep local disk a bounded, evictable cache.
- For an LMDB-backed engine, the analogues are:
  - Ship each commit's dirty-page set, or a logical log, to S3 Express in cross-tenant batches.
  - Periodically materialize page-range segments to S3 Standard.
  - Treat the local LMDB file as a rebuildable cache. LMDB's mmap model makes "lazy fetch of segments on page fault" harder: you would need a custom page-fetch layer or a sparse-file pre-populator.

**Cold start and scale-to-zero**
- "Scale-to-zero" moved from a VM per user (Fly era) to an **always-on shared process where an idle database costs roughly its S3 bytes**. This only works if per-database memory and file-descriptor overhead is near zero when idle and opening a database is cheap.
- Since the WAL must be fully local before a query is served, a database's first access after failover costs (WAL fetch from S3 Express) plus lazy segment fetches. This is a likely source of first-query tail latency (my inference; Turso publishes no number).

### Gaps
- No published production commit p50/p99 for Turso Cloud. Published figures are S3 Express microbenchmarks and the plan ceilings above.
- Not published:
  - the batch format (how multiple databases' WAL fragments are packed into one object and indexed for recovery);
  - how the WAL fragments in S3 Express are garbage-collected after a checkpoint;
  - how ordering and fencing between two pods writing the same database is enforced (no leases or conditional writes are described);
  - per-database first-access latency after eviction.
- Concurrent writes (MVCC) on Turso Cloud: how the MVCC log interacts with the S3 Express WAL path is not described.
- Third-party claims that scale-to-zero ended for new users in January 2025 come from techsy.io comparison articles and are not confirmed by Turso, beyond the AWS-migration statements above.

---

## Q6. Measured latencies published by Turso or third parties

### Takeaway
Published numbers are mostly storage microbenchmarks and sync benchmarks, not service SLOs.
- S3 Express One Zone, 4 KB, same AZ: PUT avg 6.4 ms, p99 7 ms. Standard S3 4 KB PUT: avg 31 ms, p99 102 ms. Local fsync is about 2 ms.
- Plan-level added commit latency ceiling: 10–100 ms.
- New server vs old server, local HTTP query: mean 3.3 ms vs 20.4 ms.
- Embedded replicas with read-your-writes over a public network: about 218–243 ms per write+read cycle. Turso Sync: under 1 ms locally.

### Cited Findings
**S3 microbenchmarks from EC2 us-east-1, avg / p95 / p99 in ms** — [Turso Cloud Goes Diskless, 2025-04-07](https://turso.tech/blog/turso-cloud-goes-diskless)

Standard S3:

| Operation | avg | p95 | p99 |
|---|---|---|---|
| Download 4 KB | 19 | 23 | 42 |
| Download 128 KB | 24 | 33 | 46 |
| Download 512 KB | 25 | 34 | 48 |
| Upload 4 KB | 31 | 60 | 102 |
| Upload 128 KB | 53 | 124 | 199 |
| Upload 512 KB | 83 | 176 | 233 |

S3 Express, same AZ:

| Operation | avg | p95 | p99 |
|---|---|---|---|
| Download 4 KB | 3.8 | 4 | 4 |
| Download 128 KB | 4.3 | 5 | 6 |
| Download 512 KB | 5.5 | 6 | 7 |
| Upload 4 KB | 6.4 | 7 | 7 |
| Upload 128 KB | 5.5 | 7 | 7 |
| Upload 512 KB | 7.5 | 8 | 10 |

S3 Express, cross-AZ, same region:

| Operation | avg | p95 | p99 |
|---|---|---|---|
| Download 4 KB | 4.4 | 5 | 5 |
| Download 128 KB | 4.9 | 6 | 6 |
| Download 512 KB | 6.1 | 7 | 8 |
| Upload 4 KB | 7 | 8 | 8 |
| Upload 128 KB | 6.2 | 7 | 8 |
| Upload 512 KB | 7.7 | 9 | 10 |

- AWS restates the 4 KB same-AZ numbers (PUT 6.4 / 7 / 7, GET 3.8 / 4 / 4) from 1,000 operations. It frames the trade as replacing "local disk fsync (approximately 2 milliseconds) with batched PUTs … (approximately 6.4 milliseconds on average)" — [AWS Storage Blog, 2026-07-23](https://aws.amazon.com/blogs/storage/how-turso-built-a-transactional-database-using-amazon-s3-express-one-zone/)
- Added commit latency ceiling by plan: Free ≤100 ms, Developer ≤50 ms, Scaler ≤25 ms, Pro and above ≤10 ms — [Durability docs](https://docs.turso.tech/cloud/durability)

**Server rewrite benchmark**
- One SQLite query over local HTTP with 30 concurrent connections:
  - Existing server: mean 20.377 ms, std-dev 22.408, max 135.
  - New DST server: mean 3.317 ms, std-dev 4.392, max 25.
- Async executor overhead: "almost 5x slower … A 70us difference".
- Source: [2024-10-16](https://turso.tech/blog/a-deep-look-into-our-new-massive-multitenant-architecture)

**Sync benchmark: libSQL embedded replicas vs Turso Sync** — Node 22, Turso Cloud us-east-1, client on a non-AWS VM over the public internet, `@libsql/client@0.17.2` vs `@tursodatabase/sync@0.5.3`. Source: [Turso Sync benchmark, 2026-04-24](https://turso.tech/blog/sync-benchmark)

| Scenario | Embedded replicas | Turso Sync | Ratio |
|---|---|---|---|
| 3,000 sequential inserts, readYourWrites off, one sync at end | 152 s, 34.6 MB | 17 s, 2.1 MB | 8.9x faster, 16.3x less data |
| Read-your-writes, 200 cycles | 48.6 s (243 ms/cycle), 2.7 MB | 156 ms (<1 ms/cycle), 149 KB | 312x |
| Same, pushing after every write | 43.7 s (218 ms/cycle) | 6.0 s (30 ms/cycle), 1.1 MB vs 2.7 MB | 7.3x |
| Pull 5,000 rows to a fresh DB | 718 ms, 1.08 MB | 338 ms, 1.05 MB | 2.1x |

**Engine throughput**
- SQLite: about 150k rows/s (100-row batches, full sync). It drops to about 80k rows/s with 1 ms of compute per transaction, regardless of threads.
- Turso MVCC: up to 4x the write throughput.
- Source: [2025-10-06](https://turso.tech/blog/beyond-the-single-writer-limitation-with-tursos-concurrent-writes)

### Inferences
- About 30 ms per "local write + push" cycle over the public internet is a reasonable reference for a write round trip from a client that is not co-located. Each push is one network round trip plus the server commit, including the S3 Express batch.
- About 220–240 ms per cycle for embedded replicas with readYourWrites implies multiple round trips per write: proxying the write, then pulling the resulting frames.
- For bumbledb: the S3 Express PUT floor (about 6–7 ms) plus a batch window of 0–10 ms bounds achievable durable-commit latency to about 7–17 ms p99 in-region, if batching across tenants. Standard S3 alone implies about 100–230 ms p99 per commit PUT.

### Gaps
- No Turso-published end-to-end p50/p99 for hosted writes or reads over HTTP from the same region.
- No published cold-start or first-query-after-eviction latency for the new server.
- Embedded-replica local read latency is only given as "microseconds".
- No independent third-party benchmark of Turso Cloud (2025–2026) found.

---

## Q7. Problems Turso publicly said they hit (why they rewrote, page/WAL-frame protocol issues, fork maintenance burden)

### Takeaway
Turso has been candid about four problems:
1. **Physical page-level replication** was wasteful (4 KB per tiny write), opaque (no logical change stream, so no local writes or conflict resolution), and fragile. Checkpoint ordering made replicas diverge and require full re-bootstrap. This drove the CDC-based Turso Sync.
2. **The libSQL fork of SQLite** could not safely make deep changes, given SQLite's proprietary test suite, C, and a synchronous API. This drove the Rust rewrite (Limbo, now the Turso engine).
3. **The Tokio-based multi-tenant server** had hard-to-reproduce bugs and per-query overhead. This drove a closed-source DST server.
4. **Async S3 backups** left acked writes durable only on local disk. This drove the diskless design.

Turso also cut features to focus: edge replicas, multi-DB schemas, ATTACH, and the Fly.io footprint.

### Cited Findings
**Page-level replication problems** — all from [Glauber Costa, 2026-04-24](https://turso.tech/blog/sync-benchmark)
- "There is no good way to have a logical stream of changes in SQLite. So we built our replication protocol on physical pages."
- Problems listed:
  - "It can be wasteful."
  - "There is no visibility on what changed … Embedded Replicas by default still sent writes to the remote database."
  - "Keeping the pages always applied in the same order for both the Cloud and the Local database was challenging, especially on the face of database checkpoints … the local database often diverged, and had to be re-bootstrapped from the cloud."
- With physical changes, "Any write, even for unrelated tables, would create a conflict because the physical pages would be different."

**Fork maintenance burden**
- They chose a fork in 2022 because a rewrite had a "huge lead time" and the fork allowed back-merging from SQLite — [Introducing Limbo, 2024-12-10](https://turso.tech/blog/introducing-limbo-a-complete-rewrite-of-sqlite-in-rust)
- Downsides: "SQLite's test suite is proprietary, meaning that it is hard to achieve the confidence to make very large changes. It is also written in C." Vector search required bytecode-generation changes, which was "an eye opener". Goals of the rewrite include full async I/O (io_uring) and DST built in — [Introducing Limbo, 2024-12-10](https://turso.tech/blog/introducing-limbo-a-complete-rewrite-of-sqlite-in-rust)
- "We had many reasons to rewrite SQLite. But doing sync right was at the very top of the list." Async enables partial sync, i.e. lazy page fetch — [2026-04-24](https://turso.tech/blog/sync-benchmark)

**Server-side bugs and overhead**
- The ~5,000-database creation burst caused a deadlock that took weeks to find. Async Rust and Tokio add overhead (about 70 µs per query), static-lifetime heap allocations, and nondeterminism. The response was a rewrite with DST plus Antithesis — [2024-10-16](https://turso.tech/blog/a-deep-look-into-our-new-massive-multitenant-architecture)
- The durability docs say the server "is written from the ground up to use Deterministic Simulation Testing" and Antithesis is used to test interaction with S3 and S3 Express — [Durability docs](https://docs.turso.tech/cloud/durability)

**Statefulness and asynchronous backups**
- StatefulSets in BYOC are hard: "the data needs to move with it". The old architecture acked writes before S3 — [2025-04-07](https://turso.tech/blog/turso-cloud-goes-diskless)
- Pre-diskless pod recovery "required coordinating between local WAL files and asynchronous backups" — [AWS blog](https://aws.amazon.com/blogs/storage/how-turso-built-a-transactional-database-using-amazon-s3-express-one-zone/)

**Feature cuts and refocus (2025-01-21)** — [roadmap](https://turso.tech/blog/upcoming-changes-to-the-turso-platform-and-roadmap)
- Edge replicas discontinued for new users ("70% … never create geographical replicas").
- Multi-DB schemas and ATTACH removed for new users.
- Moved fully to AWS.
- New server kept closed source.
- Layoffs, because the team composition did not fit "writing a database from scratch".

**Open issues Turso still names (2026)** — [Pekka Enberg, 2026-01-11](https://penberg.org/blog/disaggregated-agentfs.html)
- SQLite's single writer.
- Checkpointing latency ("an incremental checkpoint approach might be needed").
- Page-granular write amplification ("Physiological logging … could reduce the amplification").
- Large blobs in SQLite pages.

**libSQL status**
- Embedded replicas remain "fully supported in production", but new projects are steered to Turso Sync — [docs](https://docs.turso.tech/features/embedded-replicas/introduction)
- The Supabase acquisition announcement (2026-10-02) describes the cloud as "a diskless, WAL-on-S3 architecture" — [Turso blog](https://turso.tech/blog/turso-is-joining-supabase)

### Inferences
**Cross-cutting lessons for bumbledb**
1. **Design the replication and sync stream as logical, or physiological, from day one.** Physical page shipping is easy to bolt onto SQLite or LMDB, but it blocks local writes, conflict resolution, and bandwidth efficiency. It also couples replica correctness to checkpoint and compaction ordering. LMDB's COW pages make page shipping even more amplifying, since a write touches root-to-leaf paths.
2. **Make the commit durable in object storage before ack, and amortize it.** Use S3 Express, or an equivalent low-latency tier, for the log tail, with a cross-tenant group-commit window. Expose the window as a tunable, or even as a pricing tier.
3. **Make the base image segment-addressable and immutable** (Turso uses 128 KB). Generations should be lists of segment references, so that PITR, branching, and migration are metadata operations and cold starts can fetch lazily.
4. **Keep the WAL small** through frequent checkpoints (Turso uses about 1 MB), because failover must fetch the whole WAL before serving.
5. **Use cheap per-database identity in a shared always-on process**, instead of per-tenant processes or VMs. This is what removes cold starts.
6. **Invest in DST early.** Turso's multi-tenant failures came from rare interleavings and resource exhaustion, which ordinary tests did not catch.

### Gaps
- No public post-mortems with incident specifics, beyond the deadlock example.
- No public data on how often embedded replicas diverged or needed re-bootstrap.
- It is unclear whether Turso Cloud's server now runs the Turso (Rust) engine, libSQL, or both, per database. The MVCC launch implies the Turso engine for at least some databases, but this is not stated explicitly.
