# S3-native transactional storage engines and SQLite replication systems comparable to "a single embedded DB whose authority lives on S3" (as of 2026-10-09)

Scope: Graft (orbitinghail/graft), Litestream v0.5+ (LTX, leases, VFS, writable VFS), SlateDB, mvSQLite, Neon. LiteFS is covered briefly. Code-level claims were read from each project's `main` branch via the GitHub API on 2026-10-09 and are cited to the file URL. Where a page has no publication date, the date is inferred and marked as inferred.

Shared background on object-store primitives (used by every S3-native design below):
- S3 added `If-None-Match` (create-if-absent) on PutObject/CompleteMultipartUpload on 2024-08-20, in all regions at no extra cost — [AWS What's New, Aug 2024](https://aws.amazon.com/about-aws/whats-new/2024/08/amazon-s3-conditional-writes)
- S3 added `If-Match: <etag>` (compare-and-swap overwrite) on 2024-11-25, for general-purpose and directory (Express) buckets — [AWS What's New, Nov 2024](https://aws.amazon.com/about-aws/whats-new/2024/11/amazon-s3-functionality-conditional-writes). Conditional headers were extended to CopyObject on 2025-10-29 — [AWS What's New, Oct 2025](https://aws.amazon.com/about-aws/whats-new/2025/10/amazon-s3-conditional-write-functionality-copy-operations)
- Reference S3 Standard latencies published by turbopuffer: GET p50 63 ms / p99 78 ms / p999 118 ms; PUT p50 100 ms / p99 195 ms / p999 274 ms. turbopuffer allows 1 WAL entry per second per namespace and batches concurrent writes into it, for a write p50 of 165 ms on 500 kB — [turbopuffer architecture](https://turbopuffer.com/docs/architecture)
- S3 Express One Zone: in Turso's test (1,000 ops, same AZ), 4 KB uploads averaged 6.4 ms with p95 and p99 both about 7 ms — [AWS Storage Blog: Turso on S3 Express One Zone](https://aws.amazon.com/blogs/storage/how-turso-built-a-transactional-database-using-amazon-s3-express-one-zone/)

---

## Q1. On-object-storage layout (segments, pages, LTX files, SSTs, manifests, HEAD-like pointer objects)

### Takeaway
None of the five systems uses a mutable "HEAD" pointer object as its commit point. Graft, Litestream and SlateDB all use immutable, monotonically numbered objects and treat "the highest existing ID" as the head:
- Graft: one commit object per LSN.
- Litestream: LTX files named by TXID range.
- SlateDB: `NNNN.manifest` plus WAL SSTs numbered by ID.

SlateDB explicitly considered and rejected a CURRENT-pointer object. The data objects differ:
- Graft: zstd-framed page segments.
- Litestream: per-page-compressed LTX changesets with a trailing page index.
- SlateDB: SSTables.
- Neon: image and delta layer files, with a generation-suffixed `index_part.json`.

### Cited Findings
**Graft (v0.2, "Graft V2", Dec 2025)**
- Remote layout: commits at `logs/{LogId}/commits/{CBE64-hex LSN}` and segments at `segments/{SegmentId}`. There are only these two key types in `RemotePath`. — [graft remote.rs](https://github.com/orbitinghail/graft/blob/main/crates/graft/src/remote.rs); [Graft internals](https://graft.rs/docs/internals)
- A Commit object holds:
  - `log`, `lsn` and `page_count`.
  - A `commit_hash`, always present on remote commits.
  - An optional `segment_idx`: the segment ID, a `pageset` of the PageIdxs it contains, and a `frames` index of compressed frame lengths plus the last PageIdx per frame.
  - A `checkpoints` list. A commit is a checkpoint if it lists its own LSN.

  The commit object is therefore the page→(segment, byte range) index for that LSN. — [graft core/commit.rs](https://github.com/orbitinghail/graft/blob/main/crates/graft/src/core/commit.rs)
- Segments are sequences of ZStd-compressed frames with at most 64 pages per frame (`FRAME_MAX_PAGES = 64`). — [graft remote/segment.rs](https://github.com/orbitinghail/graft/blob/main/crates/graft/src/remote/segment.rs); [Graft glossary](https://graft.rs/docs/internals/glossary/)
- Pages are fixed at 4 KiB. Volumes are sparse, PageIdx starts at 1, and LSNs are gapless and monotonic. — [Graft glossary](https://graft.rs/docs/internals/glossary/)
- Each Volume tracks a local Log (the working copy in a local Fjall LSM) and a remote Log, which is "the source of truth, stored in object storage". — [Graft volumes](https://graft.rs/docs/concepts/volumes/); [Graft glossary](https://graft.rs/docs/internals/glossary/)
- History: v0.1.x (Mar–Jun 2025) needed two servers, MetaStore and PageStore, between clients and S3. The PageStore collated pages from many volumes into shared segments. v0.2.0-rc.5 (2025-11-28) states Graft "no longer has any middle-man services"; clients connect directly to object storage. — [Graft releases](https://github.com/orbitinghail/graft/releases); [Sverre, "Stop syncing everything" (Mar 2025)](https://sqlsync.dev/posts/stop-syncing-everything/); [HN thread with Sverre](https://news.ycombinator.com/item?id=43537272). The v0.1 material is SUPERSEDED.

**Litestream v0.5+**
- LTX files live under an `ltx/` directory on the replica, per level: `LTXFilePath(root, level, minTXID, maxTXID)` = `ltx/<level>/<minTXID>-<maxTXID>.ltx`. — [litestream.go](https://github.com/benbjohnson/litestream/blob/main/litestream.go); [v0.5.0 post](https://fly.io/blog/litestream-v050-is-here/)
- The LTX file structure is a header, then page frames, then a page index (for binary search by page number), then a trailer. The header holds PageSize, Commit (the page count after apply) and PreApplyChecksum. The trailer holds PostApplyChecksum and a CRC-64 file checksum. — [LTX_FORMAT.md](https://github.com/benbjohnson/litestream/blob/main/docs/LTX_FORMAT.md)
- "Now we compress per-page, and keep an index at the end of the LTX file to pluck individual pages out." The index is about 1% of the file size. — [v0.5.0 post](https://fly.io/blog/litestream-v050-is-here/); [Litestream VFS post](https://fly.io/blog/litestream-vfs/)
- Default levels:
  - L0: raw per-sync LTX files.
  - L1: every 30 s.
  - L2: every 5 min.
  - L3: every 1 h.
  - SnapshotLevel = 9: full snapshots, daily per the VFS post.

  — [compaction_level.go](https://github.com/benbjohnson/litestream/blob/main/compaction_level.go); [Litestream VFS post](https://fly.io/blog/litestream-vfs/)
- "Generations" (pre-v0.5) were removed. They were replaced by a monotonically increasing TXID, and a WAL-continuity break triggers a re-snapshot in the next LTX file. — [v0.5.0 post](https://fly.io/blog/litestream-v050-is-here/)
- The lease object is `lock.json` (optionally under a path prefix) holding `{generation, expires_at, owner}`. — [s3/leaser.go](https://github.com/benbjohnson/litestream/blob/main/s3/leaser.go); [ARCHITECTURE.md](https://github.com/benbjohnson/litestream/blob/main/docs/ARCHITECTURE.md)

**SlateDB**
- Bucket layout:
  - `manifest/<20-digit id>.manifest`
  - `wal/<20-digit id>.sst`
  - `compactions/<20-digit id>.compactions`
  - `compacted/<ULID>.sst` (L0 and sorted runs)
  - `gc/manifest.boundary` and `gc/compactions.boundary`

  Each manifest is a complete FlatBuffer snapshot of DB state. — [SlateDB Files](https://slatedb.io/docs/design/files/)
- "The manifest with the highest ID is considered the current manifest"; IDs are "monotonically increasing and contiguous". Fields include `writer_epoch`, `compactor_epoch`, the WAL replay bounds, the SST list and snapshots/checkpoints. The worst-case manifest is estimated at about 5.6 MiB (100k SSTs, 1k snapshots), and there are no incremental manifests. — [RFC 0001 Manifest](https://github.com/slatedb/slatedb/blob/main/rfcs/0001-manifest.md)
- Why there is a WAL separate from L0: the WAL can be flushed frequently, to reduce durable-write latency, without inflating the L0 SST count and in-memory metadata. Reads are not served from the WAL; it is only for recovery. — [SlateDB FAQ](https://slatedb.io/docs/get-started/faq/)
- A draft RFC 0032 explicitly lists "CURRENT Pointer" and "Authoritative CURRENT Pointer" as rejected alternatives. Its non-goal is to "Add a durable pointer object or change the metadata commit point." — [RFC 0032](https://github.com/slatedb/slatedb/blob/main/rfcs/0032-cached-probing-sequenced-metadata.md)

**Neon**
- Pageserver layer files are immutable:
  - Image layers hold all keys in a key range at one LSN.
  - Delta layers hold WAL records or page images over a key range and LSN range.

  Names encode the key range and LSN(s). L0 layers cover the full key range; L1 layers cover partial ranges. — [pageserver-storage.md](https://github.com/neondatabase/neon/blob/main/docs/pageserver-storage.md)
- Every layer and index key has an 8-hex-char generation suffix. The per-timeline index is `index_part.json-<generation>`. — [RFC 025 generation numbers](https://github.com/neondatabase/neon/blob/main/docs/rfcs/025-generation-numbers.md)

**mvSQLite (FoundationDB, not S3)**
- The page index maps `(page_number, page_versionstamp) -> page_hash`. A separate content-addressed store maps `page_hash -> page_content`. Updates may be stored as zstd-compressed XOR deltas against the old page. — [su3.io "Storage and transaction in mvSQLite" (2022-09-21)](https://su3.io/posts/mvsqlite-2)

### Inferences
- The industry convention for an S3-authoritative embedded DB is: an append-only, contiguously numbered sequence of small immutable metadata objects, created with `If-None-Match: *`, plus large immutable data objects named by random or unique IDs. A mutable pointer object adds a second write and a CAS hot spot without adding safety.
- Graft's design puts the page index inside each commit object. That keeps a commit to two PUTs (segment and commit) and makes lazy reads possible with only commit metadata. SlateDB instead rewrites a full manifest snapshot per metadata change.

### Gaps
- Graft's checkpoint object semantics are only partly documented. The `checkpoints` field exists in code, but the GC and compaction that would make use of checkpoints are listed as future work. Graft's earlier RFC "0001 Direct Storage Architecture" URL now returns 404.
- The exact LTX filename hex width was not verified. The TXID-range naming was confirmed.

---

## Q2. Commit path: when is a commit acknowledged, batching/group commit, latency budget, and where durability comes from

### Takeaway
Only SlateDB (with `await_durable`) and Graft's remote push block on an S3 PUT. Graft and Litestream both commit locally first and ship to S3 asynchronously:
- Graft: an explicit push or autosync.
- Litestream: about every 1 s.

Their durability window is therefore the local disk until upload. Neon gets durability from a Paxos quorum of safekeepers, never from S3, on the commit path. mvSQLite gets it from a FoundationDB transaction.

Group commit is universal:
- Graft rolls many local commits into one remote commit.
- Litestream ships one L0 file per sync interval.
- SlateDB batches all writes within `flush_interval` (default 100 ms) into one WAL SST.

### Cited Findings
**Graft**
- Local commit uses optimistic concurrency. It validates that the base snapshot is still latest, takes a global write lock, then appends with the next LSN; on conflict it aborts. "By default, Graft clients commit locally and then asynchronously attempt to commit remotely." — [Graft internals](https://graft.rs/docs/internals); [Graft consistency](https://graft.rs/docs/concepts/consistency/)
- The push protocol has six steps. It plans the LSN range from the SyncPoint watermark. It builds one segment that keeps only the newest version of each page, which "rolls up multiple local commits into a single remote commit". It compresses up to 64 pages per frame. It uploads the segment to `/segments/{SegmentId}`. It then writes the commit to `/logs/{LogId}/commits/{LSN}` "using a conditional write to detect conflicts", and finally updates the SyncPoint. — [Graft internals](https://graft.rs/docs/internals)
- In code, `put_commit` uses OpenDAL `WriteOptions { if_not_exists: true }` ("returning a precondition error if the commit already exists"). Segments are plain multipart uploads with 5-way concurrency. — [graft remote.rs](https://github.com/orbitinghail/graft/blob/main/crates/graft/src/remote.rs)
- If a push uploads every page, it is marked an "implicit checkpoint". — [remote_commit.rs](https://github.com/orbitinghail/graft/blob/main/crates/graft/src/rt/action/remote_commit.rs)
- Background sync is driven by the `autosync` interval in seconds (example: 60). It is unset by default, meaning no automatic sync. — [Graft SQLite config](https://graft.rs/docs/sqlite/config/)
- "Currently Graft provides high-latency writes at low cost." Planned mitigations are S3 Express One Zone, or "buffering writes in a low-latency durable consensus group in front of object storage". — [Graft future work](https://graft.rs/docs/internals/future/); [Stop syncing everything](https://sqlsync.dev/posts/stop-syncing-everything/)
- SQLite integration: WAL journal mode is not supported (`journal_mode=MEMORY` is recommended), and Graft provides its own crash-safe durability. — [Graft SQLite compatibility](https://graft.rs/docs/sqlite/compatibility)

**Litestream (sidecar mode)**
- The app commits to local SQLite. Litestream converts WAL changes into LTX and uploads to L0 "once per second". It performs compaction itself, not through SQLite checkpoints. — [Litestream VFS post](https://fly.io/blog/litestream-vfs/); [v0.5.0 post](https://fly.io/blog/litestream-v050-is-here/)
- Replication is asynchronous, so the commit acknowledgment never waits for S3. `SyncAndWait()` exists for library users who want to block until the WAL→LTX→remote sync completes. — [ARCHITECTURE.md](https://github.com/benbjohnson/litestream/blob/main/docs/ARCHITECTURE.md)

**Litestream writable VFS (c. Feb 2026)**
- Writes go to a local buffer file; a bitmap tracks dirty pages. Every `LITESTREAM_SYNC_INTERVAL` (default 1 s) the dirty pages are packaged into one LTX file and uploaded. "Nothing written through the VFS is truly durable until that sync happens." — [Litestream writable VFS post](https://fly.io/blog/litestream-writable-vfs/); [VFS write mode guide](https://litestream.io/guides/vfs-write-mode/)
- Remote readers see writes only after the next sync. The buffer survives process crashes and is synced on restart, but "Uncommitted changes in the buffer since the last sync may be lost on crash". It is described as a poor fit for "latency-sensitive writes". — [VFS write mode guide](https://litestream.io/guides/vfs-write-mode/)

**SlateDB**
- `put()`, `write()` and `delete()` return a `WriteHandle` once the write reaches the in-memory WAL and MemTable. `handle.await_durable().await` waits for object storage; `flush()` forces a flush. — [SlateDB Writes](https://slatedb.io/docs/design/writes/); [SlateDB FAQ](https://slatedb.io/docs/get-started/faq/)
  - Conflict: older (2024-era) descriptions said `put()` itself returned a future that resolved on durability, with `await_durable=false` to opt out. — [The New Stack 2024](https://thenewstack.io/slatedb-bottomless-databases-built-on-cloud-object-stores/). Current docs show a non-blocking default plus an explicit `await_durable`.
- A single batch-writer task serializes all writes. The WAL buffer is flushed to a new `wal/<id>.sst` when full or every `flush_interval`. The default is `Some(100ms)`; the manifest poll interval defaults to 1 s. — [RFC 0030 background](https://github.com/slatedb/slatedb/blob/main/rfcs/0030-pluggable-wal.md); [config.rs defaults](https://github.com/slatedb/slatedb/blob/main/slatedb/src/config.rs)
- Expected durable-write latency "dominated by object store PUT":

  | Object store | Expected durable-write latency |
  |---|---|
  | S3 Standard | 50–100 ms |
  | S3 Express One Zone | 5–10 ms |
  | GCS | 50–100 ms |
  | Azure Blob | 50–100 ms |
  | MinIO | 5–20 ms |

  — [SlateDB Tuning](https://slatedb.io/docs/operations/tuning/)
- "There's also a floor for this tradeoff as object store PUTs themselves take 10s of ms on average." This motivates the draft RFC 0030 for pluggable WALs (e.g. Kafka or other low-latency logs) in front of object storage. — [RFC 0030](https://github.com/slatedb/slatedb/blob/main/rfcs/0030-pluggable-wal.md)
- `DurabilityLevel` is `Memory` / `Local` (not implemented) / `Remote`. Reads can filter to durable-only data. — [RFC 0008](https://github.com/slatedb/slatedb/blob/main/rfcs/0008-synchronous-commit.md)
- s2-lite, built on SlateDB, uses a 50 ms flush interval against a remote bucket and 5 ms in-memory. This is a secondary source. — [s2 durability docs (mirror)](https://www.mintlify.com/s2-streamstore/s2/concepts/durability)

**Neon**
- Compute streams WAL to safekeepers instead of fsyncing locally. "A transaction is considered committed once a quorum of safekeepers has acknowledged the WAL record." — [Neon architecture overview](https://neon.com/docs/introduction/architecture-overview)
- A record is durable when "the majority of safekeepers have received and stored the WAL to local disk" (three are shown in the diagram), under Paxos-based consensus. Commits do not wait on the pageserver. S3 is the long-term store, written asynchronously by pageservers. — [walservice.md](https://github.com/neondatabase/neon/blob/main/docs/walservice.md)

**mvSQLite**
- Commit sends the read version, read set and mutations to `mvstore`, which runs a read-write conflict check and writes the page index in FoundationDB. The VFS unlock is the visibility fence. — [su3.io mvSQLite-2](https://su3.io/posts/mvsqlite-2); [su3.io mvSQLite](https://su3.io/posts/mvsqlite)
- The README claims a SQLite transaction can be about 39× larger than a native FDB transaction. — [mvsqlite README](https://github.com/losfair/mvsqlite)

### Inferences
- For an embedded, S3-authoritative DB, three durability contracts recur:
  - (a) "S3-durable before ack": SlateDB `await_durable`, Graft push. Costs one or two PUT round-trips, about 50–200 ms on S3 Standard p50–p99 and about 5–10 ms on S3 Express.
  - (b) "Locally durable, S3 within ~1 s": Litestream, Graft local commit, Litestream writable VFS.
  - (c) "Quorum-durable on a separate low-latency tier": Neon safekeepers, SlateDB's proposed pluggable WAL, Graft's proposed consensus group.

  No system found offers single-digit-ms S3-Standard durability.
- A Graft remote commit costs two sequential PUTs (segment, then conditional commit), plus SyncPoint bookkeeping locally. SlateDB's durable write costs one WAL PUT. The manifest is updated only on memtable flush, not per write.

### Gaps
- No published p50/p99 commit latency for Graft v2 pushes or Litestream syncs was found.
- No SlateDB p99 for `await_durable` on S3 Standard or Express was found. Release benchmarks exist at benchmark.slatedb.io ([SlateDB Benchmarks](https://slatedb.io/docs/operations/benchmarks/)) but were not retrieved.
- No Neon-published commit latency figures were found.

---

## Q3. Single-writer enforcement: leases, epochs/fencing tokens, conditional PUTs, CAS loops; takeover and zombie behavior

### Takeaway
SlateDB has the most rigorous S3-only fencing:
- A monotonically increasing `writer_epoch` is stored in the manifest and bumped by CAS.
- A new writer then writes an empty "fencing WAL file" at the next WAL ID with `If-None-Match`. The zombie's next WAL PUT collides and it halts.
- GC boundary files, advanced by `If-Match`, close the "GC deleted the ID so a stale create-if-absent succeeds" hole.

Graft relies only on create-if-absent of `commits/{LSN}`, which is a CAS on the log tail. There are no leases or epochs; a loser gets AlreadyExists and must reconcile.

Litestream (Feb 2026) added a time-based lease on `lock.json` via `If-None-Match`/`If-Match`, TTL 30 s. Its LTX data uploads are not conditional, so the lease is advisory coordination, not a fencing token.

Neon fences pageserver writes to S3 with control-plane-issued generation numbers embedded in object keys. Writers never overwrite each other, and deletions are validated.

### Cited Findings
**SlateDB**
- Writer startup proceeds in steps:
  1. Read the latest manifest.
  2. Increment `writer_epoch`.
  3. Write the next manifest ID (CAS).
  4. "fence all older clients... by writing an empty SST to the next SST ID in the WAL".

  Four outcomes are possible:
  - Success.
  - The slot was taken by an older epoch: move to the next ID and retry.
  - The slot was taken by the same epoch: panic, illegal state.
  - The slot was taken by a newer epoch: "the current writer has been fenced... should halt".

  "A zombie writer is a writer with an epoch that is less than the `writer_epoch` in the current manifest." Zombies detect fencing only on write. The compactor has its own `compactor_epoch` with the same protocol. — [RFC 0001](https://github.com/slatedb/slatedb/blob/main/rfcs/0001-manifest.md)
- Current implementation: "SlateDB fences the WAL by writing a so-called Fencing WAL File (with no rows) to the next WAL ID. WAL File PUTs use `If-None-Match`, so older writers fail with an object store collision on the next WAL write." Both the manifest and the WAL must be fenced, "as SlateDB does not/can not transactionally read-modify-write across the two." — [RFC 0030](https://github.com/slatedb/slatedb/blob/main/rfcs/0030-pluggable-wal.md)
- Epoch invariants (RFC 0030 trait docs):
  - After the first manifest write with epoch E, there are no further manifest/WAL writes with E' < E.
  - The same holds after the first WAL write with E.
  - Rows from older epochs that are not in L0/SRs must be recovered.

  — [RFC 0030](https://github.com/slatedb/slatedb/blob/main/rfcs/0030-pluggable-wal.md)
- The original RFC 0001 needed a DynamoDB-assisted two-phase write on S3. It is SUPERSEDED by native S3 conditional writes, as RFC 0030 describes `If-None-Match` on WAL PUTs. — [RFC 0001](https://github.com/slatedb/slatedb/blob/main/rfcs/0001-manifest.md)
- GC boundary hazard: without boundaries, a writer "can begin updating a manifest... stop making progress (for example, because its process or host is suspended), then resume after `min_age`. GC may have deleted the ID it intended to create, so create-if-absent can reuse that ID and report a stale update as successful."
  - Fix: the GC advances `gc/*.boundary` (ETag-based `If-Match`). Writers check the boundary after creating a metadata object, and treat IDs ≤ boundary as failed.
  - Stores without `If-Match` can disable this, but then `min_age` must exceed the maximum lifetime of a stale process.

  — [SlateDB GC](https://slatedb.io/docs/design/gc/); [SlateDB Files](https://slatedb.io/docs/design/files/); [RFC 0026](https://github.com/slatedb/slatedb/blob/main/rfcs/0026-garbage-collector-boundary.md)
- "since SlateDB fences stale writers, partition management should be fairly straightforward." — [SlateDB FAQ](https://slatedb.io/docs/get-started/faq/)

**Graft**
- The remote commit is the CAS. `put_commit` writes `logs/{log}/commits/{lsn}` with `if_not_exists`. On AlreadyExists the code distinguishes:
  - "Someone (including us) pushed the same commit (idempotency)".
  - "Someone... pushed a DIFFERENT commit (divergence)".

  On other errors it leaves a `pending_commit` for a later `recover_pending_commit` job, which fetches the remote log to see whether the commit landed. — [remote_commit.rs](https://github.com/orbitinghail/graft/blob/main/crates/graft/src/rt/action/remote_commit.rs); [remote.rs](https://github.com/orbitinghail/graft/blob/main/crates/graft/src/remote.rs)
- On rejection the client must:
  - fork the volume,
  - reset and replay (globally strict-serializable; locally "Optimistic Snapshot Isolation", where reads may observe snapshots that never exist globally), or
  - merge (not implemented).

  "divergence requires manual intervention." — [Graft consistency](https://graft.rs/docs/concepts/consistency/); [Graft internals](https://graft.rs/docs/internals)
- Graft "does not currently support accessing the same SQLite database from multiple processes" but supports multiple connections in one process. — [Graft SQLite compatibility](https://graft.rs/docs/sqlite/compatibility)

**Litestream**
- History:
  - LiteFS used Consul for a single leader.
  - The v0.5 redesign proposed a "time-based lease" using S3/Tigris conditional writes ("CASAAS: Compare-and-Swap as a Service").

  — [Litestream Revamped (c. May 2025; undated page)](https://fly.io/blog/litestream-revamped/)
- Shipped: PR #1073 "feat(s3): add distributed leasing with If-Match conditional writes" was merged 2026-02-07, after an earlier attempt (#617, 2024) was closed. — [PR #1073](https://github.com/benbjohnson/litestream/pull/1073); [PR #617](https://github.com/benbjohnson/litestream/pull/617)
- Implementation:
  - `DefaultLeaseTTL = 30s`, `lock.json`.
  - Acquire reads the lease. If it is unexpired, acquire fails with `LeaseExistsError`. Otherwise it writes `{generation+1, expires_at=now+TTL, owner=hostname:pid}` with `If-None-Match: *` (first acquire) or `If-Match: <etag>` (expired takeover). A 412 means another instance won.
  - Renew is an `If-Match` PUT on the held ETag; failure returns `ErrLeaseNotHeld`. Release is a `DeleteObject` with `If-Match`.

  — [s3/leaser.go](https://github.com/benbjohnson/litestream/blob/main/s3/leaser.go); [ARCHITECTURE.md](https://github.com/benbjohnson/litestream/blob/main/docs/ARCHITECTURE.md)
- LTX uploads (`WriteLTXFile`) in the S3 replica client carry no `IfNoneMatch`/`IfMatch` headers; a grep of the file found none on 2026-10-09. — [s3/replica_client.go](https://github.com/benbjohnson/litestream/blob/main/s3/replica_client.go)
- Ben Johnson on HN, on the writable-VFS thread: "Currently we're handling the 'single writer' issue outside of Litestream... the lease PR is the direction we're looking at going." — [HN 46893167](https://news.ycombinator.com/item?id=46893167)
- Open lease follow-ups: PR #1317 ("standalone S3 lease runner", opened 2026-06-18) and #1509 ("support Hetzner lease semantics", 2026-09-04). — [Litestream PRs](https://github.com/benbjohnson/litestream/pulls?q=lease)
- Writable VFS: "Before uploading, the VFS checks if new LTX files appeared from another source. If so, a conflict is raised." It is "Single-writer only, with no conflict prevention"; external coordination is recommended. — [VFS write mode guide](https://litestream.io/guides/vfs-write-mode/); [Litestream VFS docs](https://litestream.io/how-it-works/vfs/)

**Neon**
- The control plane is the sole issuer of generation numbers. A generation increments on reassignment or on pageserver restart and re-attach. Each attachment writes only keys suffixed with its own generation, so competing attachments never collide.
- Deletions follow three steps:
  1. Write an `index_part.json` that unlinks the object.
  2. Ask the control plane whether the generation is still latest.
  3. Delete only if it is.

  A failed validation leaks the object for the scrubber.
- On load, a pageserver picks the highest `index_part.json-<gen>` not newer than its own. "objects written by later generations are never visible to earlier generations." — [RFC 025](https://github.com/neondatabase/neon/blob/main/docs/rfcs/025-generation-numbers.md)
- Compute-side single primary: Paxos ensures "only one primary node can be actively streaming WAL to the quorum of safekeepers". — [walservice.md](https://github.com/neondatabase/neon/blob/main/docs/walservice.md)

**mvSQLite**
- There are no leases. Concurrency is optimistic and fine-grained ("BEGIN CONCURRENT"-like), with no distributed lock on the data plane. FDB detects conflicts against the read set. — [mvsqlite README](https://github.com/losfair/mvsqlite)

### Inferences
- A Litestream zombie could keep uploading LTX files between losing the lease and noticing the failed renew (up to about one TTL), because data PUTs are not conditioned on the lease. The lease prevents overlapping ephemeral instances in the normal case but is not a fencing token in the Kleppmann sense. This is inferred from the absence of conditional headers in the replica client.
- Graft's and SlateDB's patterns are equivalent at their core: the commit point is "create object N+1 iff absent". SlateDB adds epochs so a new writer can deterministically invalidate a zombie before the zombie's next write. Graft has no epoch and simply makes the loser reconcile, which suits its multi-writer/offline model.
- The GC-boundary hazard SlateDB documents applies to any "create-if-absent at next ID" design that also deletes old IDs. A bumbledb design should either never delete the tail region or use an `If-Match`-protected high-water mark as SlateDB does.

### Gaps
- It was not verified whether Litestream's replication loop checks lease validity before each LTX upload. Only the upload call's lack of conditional headers was verified.
- No documented behavior was found for SlateDB on S3-compatible stores lacking `If-None-Match`.

---

## Q4. Read path and replicas: discovering new versions, lazy/partial page fetching, caches, snapshot isolation

### Takeaway
Every S3-native system discovers new versions by polling; none uses push notifications:
- Graft probes the next LSN with GET until 404.
- Litestream VFS polls LTX listings every 1 s by default.
- SlateDB `DbReader` polls the manifest every 1 s and replays newer WAL SSTs. A draft RFC replaces LIST with "GET N+1 until 404" probing to cut idle cost.

Lazy page fetch is via byte-range GETs:
- Graft fetches zstd frames of up to 64 pages.
- The Litestream VFS fetches single pages located through the LTX trailer index.
- SlateDB fetches SST blocks plus a block cache and disk cache.

Snapshot isolation comes from immutable versions: Graft snapshots are LSN ranges, Litestream VFS uses a main plus pending index, and SlateDB uses checkpoints that pin a manifest.

### Cited Findings
**Graft**
- Pull streams only missing commits from `/logs/{LogId}/commits/{LSN}`. `stream_commits_ordered` issues GETs (first one alone, then chunks of 5 concurrent) and "Stops fetching commits as soon as we receive a `NotFound`". There is no LIST and no pointer object. — [graft remote.rs](https://github.com/orbitinghail/graft/blob/main/crates/graft/src/remote.rs); [fetch_log.rs](https://github.com/orbitinghail/graft/blob/main/crates/graft/src/rt/action/fetch_log.rs)
- Pages are loaded lazily. Graft "first searches the snapshot to find which segment contains it... then fetches that segment's frame from remote storage if not already cached locally". Frames are read via `get_segment_range` (a byte-range read). — [Graft internals](https://graft.rs/docs/internals); [remote.rs](https://github.com/orbitinghail/graft/blob/main/crates/graft/src/remote.rs)
- Snapshots are immutable sets of LSN ranges, so readers are lock-free. Writers get read-your-writes over a base snapshot. — [Graft internals](https://graft.rs/docs/internals)
- In v0.1, change discovery returned a "graft": a Splinter (Roaring-like) bitset of the page indexes changed since a snapshot, used to invalidate cache entries. This was SUPERSEDED in v2 by commit objects carrying pagesets. — [Stop syncing everything](https://sqlsync.dev/posts/stop-syncing-everything/); [commit.rs](https://github.com/orbitinghail/graft/blob/main/crates/graft/src/core/commit.rs)
- Planned: hedge S3 requests if there is no first byte within 200 ms. — [Graft future work](https://graft.rs/docs/internals/future/)

**Litestream VFS (read replica, c. Dec 2025)**
- Before opening, the VFS builds a restore plan that needs a contiguous LTX sequence and fails fast on gaps. For each LTX file it reads the page index into an in-memory map from page number to (file, offset, size), then fetches individual pages from storage. — [Litestream VFS docs](https://litestream.io/how-it-works/vfs/); [VFS post](https://fly.io/blog/litestream-vfs/)
- Polling defaults to every 1 s. L0 is checked first, with fallback to higher levels on L0 gaps. "Retention of recent L0 files on the primary (`l0-retention`) is important". — [Litestream VFS docs](https://litestream.io/how-it-works/vfs/)
- The LRU page cache defaults to 10 MB, and cached pages for updated page numbers are invalidated at transaction end. Page 1 is rewritten to report DELETE journal mode, so SQLite treats the replica as a read-only rollback-journal DB. — [Litestream VFS docs](https://litestream.io/how-it-works/vfs/)
- Isolation uses a two-index design. A "main" index serves current readers; a "pending" index accumulates new LTX entries while a read lock is held and merges on release. Examples: about 3,000 entries for 100 writes/s over 30 s, and about 60,000 for 1,000 writes/s over 60 s. It is aimed at "moderate query load, not high-concurrency OLTP". — [Litestream VFS docs](https://litestream.io/how-it-works/vfs/)
- Time travel: `PRAGMA litestream_time = '5 minutes ago'` at about one-second resolution. — [VFS post](https://fly.io/blog/litestream-vfs/)

**SlateDB**
- `DbReader` does three things:
  - Creates a checkpoint in the latest manifest (with `checkpoint_lifetime`), pinning the L0/sorted-run view.
  - Replays newer WAL SSTs into immutable memtables (unless `skip_wal_replay`).
  - Runs a poller every `manifest_poll_interval` that swaps checkpoints when the SST layout changes and refreshes the checkpoint expiry at half-life.

  It only returns committed data already durable in object storage. — [SlateDB Readers](https://slatedb.io/docs/design/readers/)
- Latest-manifest discovery today is LIST + sort + GET + boundary check. A DB idle for five minutes incurs 560 LISTs with default config, "$24.19 per-month in AWS S3 us-east-1".
  - The draft RFC 0032 caches the latest ID and probes N+1 with up to four GETs (a 404 means N is latest), falling back to LIST. The goal is "Idle databases should cost less than $5 per month."

  — [RFC 0032](https://github.com/slatedb/slatedb/blob/main/rfcs/0032-cached-probing-sequenced-metadata.md)
- Read latency:
  - Working sets in memory or disk cache: < 1 ms.
  - Sequential and nearby-key reads: < 1 ms, due to block prefetch and caching.
  - Wide random reads larger than the local cache: approach object-store latency (50–100 ms on S3 Standard).

  — [SlateDB Tuning](https://slatedb.io/docs/operations/tuning/)
- Snapshots pin state so it cannot be garbage-collected. — [SlateDB Consistency](https://slatedb.io/docs/design/consistency/)
- The part-based `CachedObjectStore` (4 MiB parts) "did more harm than good" in recent testing. Draft RFC 0034 proposes a whole-file local mirror for compacted SSTs, so that all compacted-SST reads are local. — [RFC 0034](https://github.com/slatedb/slatedb/blob/main/rfcs/0034-local-object-mirroring.md)
- Object store limits: about 3,500 PUT/s and 5,500 GET/s per prefix. — [SlateDB Tuning](https://slatedb.io/docs/operations/tuning/)

**Neon**
- Compute reads check RAM, then local NVMe cache, then the pageserver. The pageserver returns or materializes the page version at the requested LSN by replaying WAL onto a base image. "queries do not read from object storage." — [Neon architecture overview](https://neon.com/docs/introduction/architecture-overview); [pageserver-storage.md](https://github.com/neondatabase/neon/blob/main/docs/pageserver-storage.md)
- If a needed layer is not local, it "is fetched from Cloud Storage and stored in local disk". — [pageserver-storage.md](https://github.com/neondatabase/neon/blob/main/docs/pageserver-storage.md)

**mvSQLite**
- A page read at a snapshot is a reverse range scan over `(page_number, 0)..=(page_number, read_versionstamp)` with limit 1, at O(log n) for any historical version. On transaction init the client fetches the mutation log since its last known version and evicts those pages from its in-memory cache. — [su3.io mvSQLite-2](https://su3.io/posts/mvsqlite-2)

### Inferences
- "GET N+1 until 404" is converging as the cheapest freshness check: Graft does it today, and SlateDB proposes it. For a strongly consistent store such as S3 (read-after-write since 2020), the first 404 is authoritative given contiguous IDs. A replica polling at 1 s costs about 86,400 GETs/day, about $0.035/day at $0.0004/1k (pricing per [SlateDB FAQ](https://slatedb.io/docs/get-started/faq/)). The same rate with LIST costs about 12.5× more per request.
- Per-page range GETs (Litestream VFS) minimize bytes but maximize request count. Graft's 64-page frames and SlateDB's blocks trade some read amplification for fewer GETs.

### Gaps
- The exact number of LIST calls per Litestream VFS poll was not found.
- Graft's actual cache eviction policy for the local Fjall page store was not found.

---

## Q5. Cold start: what a fresh process (e.g. a serverless function) downloads before the first read or write

### Takeaway
All S3-native systems avoid full hydration on the critical path, but differ in metadata cost:
- Graft fetches every commit object from LSN 1 on a fresh clone (metadata only; pages lazily).
- Litestream VFS reads the page-index trailers of every LTX file in the restore plan (snapshot + L1–L3 + L0), then serves pages lazily, with optional background hydration.
- A SlateDB writer must read the manifest, bump the epoch (one manifest PUT), write a fencing WAL (one PUT), and replay unflushed WAL SSTs before accepting writes.
- Neon compute is stateless and cold-starts in about 500 ms from pre-warmed pools. The heavy state stays on pageservers.

### Cited Findings
**Graft**
- `FetchLog` computes `start = latest_local_lsn.next()` or `LSN::FIRST` for a new volume. It streams commits to the end of the log, then re-fetches any commits upgraded to checkpoints. — [fetch_log.rs](https://github.com/orbitinghail/graft/blob/main/crates/graft/src/rt/action/fetch_log.rs)
- "Decoupled metadata and data allow replicas to spin up immediately." Pull "update[s] commit metadata but defer[s] page downloads until accessed". — [Graft README](https://github.com/orbitinghail/graft); [DeepWiki (AI-generated, secondary)](https://deepwiki.com/orbitinghail/graft)
- `HydrateSnapshot` "Downloads all missing pages for a Snapshot". It coalesces adjacent frames into fewer range requests, with 5 in flight. — [hydrate_snapshot.rs](https://github.com/orbitinghail/graft/blob/main/crates/graft/src/rt/action/hydrate_snapshot.rs)
- The `pragma graft_clone` creates a local Volume tracking a remote Log ("Like git clone"). — [Graft pragmas](https://graft.rs/docs/sqlite/pragmas/)

**Litestream**
- VFS: "starts up really fast"; there is no full download before querying. — [VFS post](https://fly.io/blog/litestream-vfs/)
- Startup computes a restore plan, ingests each LTX file's page index, then closes the streams. — [Litestream VFS docs](https://litestream.io/how-it-works/vfs/)
- Point-in-time restore typically touches "a dozen or so files on average". — [v0.5.0 post](https://fly.io/blog/litestream-v050-is-here/)
- Hydration (`LITESTREAM_HYDRATION_ENABLED` or `_PATH`) streams a full copy via compaction in the background. Reads come from object storage plus cache, then cache-or-remote, then local disk once hydration finishes.
  - Since v0.5.9 a hydrated file with a `.meta` TXID can persist across restarts; it is discarded if the remote regressed.
  - "A 10GB database can be hydrated with the same memory footprint as serving it read-only."

  — [Litestream VFS docs](https://litestream.io/how-it-works/vfs/)
- Motivating use case: Fly Sprites boot in under a second. A block map of "low tens of megabytes worst case" may need reconstituting from object storage after a restart, "while answering an incoming web request". Object-storage queries suit cold start but are "not fast enough for steady state", hence hydration. — [Writable VFS post](https://fly.io/blog/litestream-writable-vfs/)
- With the client co-located with the bucket, VFS reads add roughly 5–50 ms versus local disk. — [Litestream VFS docs](https://litestream.io/how-it-works/vfs/)

**SlateDB**
- Writer open fences the manifest and WAL. It then replays WAL files from `replay_after_wal_id` up to the fence, filtering rows ≤ `last_l0_seq`, into memtables. — [RFC 0030 background](https://github.com/slatedb/slatedb/blob/main/rfcs/0030-pluggable-wal.md)
- Reader open creates a checkpoint (a manifest write) and replays newer WAL. — [SlateDB Readers](https://slatedb.io/docs/design/readers/)
- The disk cache has `preload_disk_cache_on_startup`. The part-based cache's "startup scans that rebuild the in-memory index" are a cited pain point. — [SlateDB Tuning](https://slatedb.io/docs/operations/tuning/); [RFC 0034](https://github.com/slatedb/slatedb/blob/main/rfcs/0034-local-object-mirroring.md)

**Neon**
- Compute cold start went from "3 to 6 seconds in the ideal case" to about 500 ms. This came from pools of pre-started compute instances, cached internal IPs, and fewer config edits at startup. — [Neon "Cold starts just got hot"](https://neon.tech/blog/cold-starts-just-got-hot); [Neon changelog 2023-07-25](https://neon.com/docs/changelog/2023-07-25)
- The docs give 500 ms to "a few seconds" to wake an idle compute, and the default scale-to-zero after 5 minutes idle. — [Neon connection latency](https://neon.tech/docs/connect/connection-latency); [Neon scale to zero](https://neon.tech/docs/introduction/scale-to-zero)
- A new compute "attaches immediately and continues from the same history". — [Neon architecture overview](https://neon.com/docs/introduction/architecture-overview)

**mvSQLite**
- The client holds no durable local state. A transaction start fetches the read version plus the mutation log since the client's last known version. — [su3.io mvSQLite-2](https://su3.io/posts/mvsqlite-2)

### Inferences
- Graft's cold-start metadata cost grows linearly with log length: one GET per LSN, five concurrent, until a 404. Without compaction or GC, which are listed as future work, a long-lived volume's first clone could need thousands of sequential-ish GETs. A design that wants O(1) cold start needs a periodically written checkpoint or snapshot index whose location is discoverable without scanning (SlateDB's manifest gives this at the cost of a LIST or probe).
- SlateDB's writer cold start needs at least two round-trips of conditional writes (manifest epoch bump, fencing WAL) plus a LIST and GETs. That is likely about 200–500 ms on S3 Standard, an estimate from the per-PUT figures above, not a measurement.
- For a serverless function, the Litestream VFS model (open read-only immediately; hydrate in background; per-page range GETs) is the most directly applicable published pattern. Its write mode, however, offers only ~1 s eventual durability and no fencing.

### Gaps
- No measured cold-start times were found for Graft v2, Litestream VFS, or SlateDB open.
- Neon's 500 ms figures are vendor-published, dated 2023, and cover compute only. They do not include pageserver layer download from S3.

---

## Q6. Compaction, checkpointing and GC of old objects, and how they stay safe with concurrent readers

### Takeaway
- Litestream: compaction is time-tiered (L0 → 30 s → 5 min → 1 h → daily snapshot), keeps only the newest page per window, and doubles as PITR. Readers rely on L0 retention.
- SlateDB: a separate compactor with its own epoch, and a GC that deletes only objects older than `min_age` that no active manifest or checkpoint references, guarded by boundary files.
- Neon: generation-validated deletions plus an LSN horizon and branches.
- Graft v2: no GC or compaction yet; checkpoints exist only as a commit flag.

### Cited Findings
- **Litestream:**
  - "Litestream performs this compaction itself. It doesn't rely on SQLite to process the WAL file."
  - Compaction reads backward and skips pages already seen, keeping the newest version of each page.
  - L0 files are retained only until compacted into L1.

  — [v0.5.0 post](https://fly.io/blog/litestream-v050-is-here/); [VFS post](https://fly.io/blog/litestream-vfs/); [compaction_level.go](https://github.com/benbjohnson/litestream/blob/main/compaction_level.go)
- **Litestream (VFS):** L1 compactions replace the page index when commits shrink, e.g. after VACUUM. — [Litestream VFS docs](https://litestream.io/how-it-works/vfs/)
- **SlateDB GC:**
  - Runs in-process (or standalone), per file type, with an `interval` and `min_age`.
  - Deletes files older than `min_age` "that are not referenced by any active manifest or checkpoint".
  - Excludes the latest manifest and fencing WALs.
  - Example settings in the config docs: manifests, compacted SSTs and compactions at interval 300 s / min_age 86,400 s; WAL at 60 s / 60 s.

  — [SlateDB GC](https://slatedb.io/docs/design/gc/); [config.rs](https://github.com/slatedb/slatedb/blob/main/slatedb/src/config.rs); [RFC 0030](https://github.com/slatedb/slatedb/blob/main/rfcs/0030-pluggable-wal.md)
- **SlateDB RFC 0001 rules:**
  - The compactor must not delete the WAL SST at `wal_id_last_compacted`, because readers need it to recover `writer_epoch`.
  - Orphaned temp objects are removed after a day.
  - Small clock skew is assumed.

  — [RFC 0001](https://github.com/slatedb/slatedb/blob/main/rfcs/0001-manifest.md)
- **SlateDB:** compaction state is persisted in `compactions/` (RFC 0013), and a draft RFC 0025 covers distributed compaction. — [SlateDB Files](https://slatedb.io/docs/design/files/); [SlateDB RFC list](https://github.com/slatedb/slatedb/tree/main/rfcs)
- **Neon:**
  - Compaction merges L0 deltas into L1 layers with narrower key ranges.
  - GC keeps history within an LSN horizon (64 MB by default in this older doc) and keeps files needed by child branches.
  - PITR is a branch at a historic LSN.

  — [pageserver-storage.md](https://github.com/neondatabase/neon/blob/main/docs/pageserver-storage.md). Deletes are queued per pageserver and validated against the control plane's latest generation. — [RFC 025](https://github.com/neondatabase/neon/blob/main/docs/rfcs/025-generation-numbers.md)
- **Graft:**
  - "Garbage collection, checkpointing, and compaction... are needed to maximize query performance, minimize wasted space, and enable deleting data permanently"; these are listed under future work.
  - Rule: "a checkpoint can't be deleted until no snapshots exist between the checkpoint and the subsequent one."
  - Planned XOR page deltas against the last checkpoint.

  — [Graft future work](https://graft.rs/docs/internals/future/)
- **mvSQLite:** GC scans the entire page index to discover garbage. — [su3.io mvSQLite-2](https://su3.io/posts/mvsqlite-2)

### Inferences
- The common safety rule is: never delete an object a live reader may reference. It is enforced either by reference tracking in a manifest with reader-registered checkpoints that have lifetimes (SlateDB), or by time retention long enough for readers to finish (Litestream `l0-retention`, SlateDB `min_age`). SlateDB uses both, plus boundary files for metadata.

### Gaps
- Litestream's default retention and snapshot intervals in v0.5.17 were not verified.
- How Litestream VFS readers behave if L0 is deleted mid-read beyond "fallback to higher levels" was not found.

---

## Q7. Published performance: commit latency p50/p99, cold-start time, request count and cost per commit

### Takeaway
Hard numbers are scarce. SlateDB publishes expected durable-write latency by store (S3 Standard 50–100 ms; S3 Express 5–10 ms), < 1 ms cached reads, and an idle-cost analysis. Neon publishes about 500 ms compute cold start. Litestream publishes a 1 s ship interval and about 5–50 ms added VFS read latency. Graft publishes no numbers.

### Cited Findings
- **SlateDB:**
  - Write latency: S3 Standard 50–100 ms, S3 Express One Zone 5–10 ms, GCS and Azure 50–100 ms, MinIO 5–20 ms.
  - Reads < 1 ms for cached working sets.
  - About 3,500 PUT/s and 5,500 GET/s per prefix.

  — [SlateDB Tuning](https://slatedb.io/docs/operations/tuning/)
- **SlateDB:** an idle DB with default polling makes 560 LISTs per 5 minutes, about $24.19/month; RFC 0032 targets < $5/month. — [RFC 0032](https://github.com/slatedb/slatedb/blob/main/rfcs/0032-cached-probing-sequenced-metadata.md)
- **SlateDB:** S3 Standard is $0.005 per 1k writes and $0.0004 per 1k reads. — [SlateDB FAQ](https://slatedb.io/docs/get-started/faq/)
- **SlateDB:** release benchmarks run on a 300M-record (about 120 GiB) DB with YCSB and db_bench workloads, 64 closed-loop clients, and 15-minute runs, published at benchmark.slatedb.io. — [SlateDB Benchmarks](https://slatedb.io/docs/operations/benchmarks/)
- **Litestream:**
  - L0 upload every 1 s.
  - Writable-VFS sync interval 1 s by default (100 ms and 10 s are given as example settings).
  - VFS read overhead about 5–50 ms versus local disk when co-located.

  — [VFS post](https://fly.io/blog/litestream-vfs/); [VFS write mode guide](https://litestream.io/guides/vfs-write-mode/); [Litestream VFS docs](https://litestream.io/how-it-works/vfs/)
- **Neon:** cold start about 500 ms (down from 3–6 s), and "500 ms to a few seconds" to wake. — [Neon blog](https://neon.tech/blog/cold-starts-just-got-hot); [Neon connection latency](https://neon.tech/docs/connect/connection-latency)
- **Neon:** safekeepers now pre-process WAL into the pageserver format with Protobuf and Zstd, cutting WAL transmission bandwidth by about 70%. This is an ingest improvement, not commit latency. — [Neon storage perf improvements](https://neon.com/blog/recent-storage-performance-improvements-at-neon)
- **Graft:** qualitative only: "high-latency writes at low cost". — [Graft future work](https://graft.rs/docs/internals/future/)
- **mvSQLite:** transactions about 39× larger than native FDB (README claim; no latency figures). — [mvsqlite README](https://github.com/losfair/mvsqlite)
- **Raw S3** (see the background section): PUT p50 100 ms / p99 195 ms; GET p50 63 ms / p99 78 ms ([turbopuffer](https://turbopuffer.com/docs/architecture)). S3 Express 4 KB PUT averages 6.4 ms with p99 about 7 ms ([AWS/Turso](https://aws.amazon.com/blogs/storage/how-turso-built-a-transactional-database-using-amazon-s3-express-one-zone/)).

### Inferences
These request costs are derived at $0.005 per 1k PUTs, not measured:

| System and setting | Write rate if continuously busy | PUT cost |
|---|---|---|
| Graft push | 2 PUTs per remote commit (segment + conditional commit) | — |
| Litestream at 1 s L0 cadence | up to 86,400 L0 PUTs/day | about $0.43/day, about $13/month, before compaction PUTs |
| SlateDB with `flush_interval=100ms` | up to 10 WAL PUTs/s, about 864k/day | about $4.30/day, about $130/month, before memtable-flush and compaction PUTs |

Group commit amortizes these costs across all writes in a window.

### Gaps
- There are no p50/p99 commit latencies for Graft, Litestream or SlateDB under load in the retrieved sources.
- No numbers from benchmark.slatedb.io were retrieved.
- No published Litestream VFS cold-open time for a large database was found.

---

## Q8. Maturity and status in 2026 (versions, production users); LiteFS and why Fly moved away

### Takeaway
SlateDB is the most production-adopted S3-native engine. It released v0.17.0 on 2026-09-29, releases monthly, has 20+ listed adopters including Dropbox, Prisma and Gadget, and is in the Commonhaus Foundation.

Litestream v0.5.x is actively maintained (v0.5.17, 2026-08-31; commits on 2026-10-09). Its VFS, writable VFS, hydration and S3 leasing all shipped between late 2025 and 2026.

Graft is "Alpha". v0.2.1 (2025-12-04) went serverless-direct-to-S3, and the repo remained active through mid-2026, including Antithesis testing.

mvSQLite is lightly maintained (v0.3.22, 2026-04-30). Neon was acquired by Databricks (2025) and underpins Lakebase. LiteFS is effectively in maintenance: its last release was April 2025, LiteFS Cloud shut down 2024-10-15, and Fly says it cannot provide support.

### Cited Findings
- **Graft:**
  - Releases: v0.2.0 on 2025-12-03 ("Graft V2 is here!") and v0.2.1 on 2025-12-04. The v0.1.x line ran Mar–Jun 2025.
  - The README says "Graft should be considered Alpha quality software"; contact the author before production use.
  - Last commits were 2026-06-17 (an Antithesis API-key and badge commit, 2026-06-12). About 1.6k stars; dual MIT/Apache-2.0.

  — [Graft releases](https://github.com/orbitinghail/graft/releases); [Graft README](https://github.com/orbitinghail/graft); [Graft commits](https://github.com/orbitinghail/graft/commits/main)
- **Graft:** Dec 2025 issues include zerocopy local storage, a checkout-at-LSN pragma, SQLite-aware diff/merge, and per-volume page sizes. — [Graft issues](https://github.com/orbitinghail/graft/issues)
- **SlateDB:**
  - Releases in 2026: v0.12.1 (Apr 16), v0.13.0 (May 15), v0.14.0 (Jun 25), v0.15.0 (Jul 29), v0.16.0 (Aug 31), v0.17.0 (Sep 29). The last commit was 2026-10-07.
  - The README says releases come "approximately every 2 months". Actual 2026 cadence is closer to monthly, which conflicts with the README.
  - Storage format is compatible between adjacent versions only; there is no API stability guarantee.

  — [SlateDB releases](https://github.com/slatedb/slatedb/releases); [SlateDB README](https://github.com/slatedb/slatedb)
- **SlateDB adopters listed:** 4og.io, Dropbox, Embucket, Gadget, Goldsky, HelixDB, Malstrom, Massive, Merklemap, OpenData, Prisma, Responsive, s2-lite, Storrito, Taquba, Tasklet, Tensorlake, Triplox, Volga, WombatKV, ZeroFS, LixRay. It is a Commonhaus Foundation member, with about 3.5k stars. — [SlateDB README](https://github.com/slatedb/slatedb)
- **Litestream:**
  - Releases v0.5.7 (2026-02-02) through v0.5.17 (2026-08-31). A v0.3.14 maintenance release came out 2026-03-26.
  - The last commit, 2026-10-09, is "fix(vfs): fix litestream_time() to update with new LTX files".
  - v0.5.0 announced c. Oct 2025: one replica destination per DB, CGO removed (modernc.org/sqlite), NATS JetStream replica, and no restore from v0.3 WAL segments.

  — [Litestream releases](https://github.com/benbjohnson/litestream/releases); [v0.5.0 post](https://fly.io/blog/litestream-v050-is-here/); [Simon Willison 2025-10-03](https://simonwillison.net/2025/Oct/3/litestream/)
- **Litestream:** hydration persistence requires ≥ v0.5.9. VFS packages are published for npm, Python and Ruby. — [Litestream VFS docs](https://litestream.io/how-it-works/vfs/); [Litestream repo tree](https://github.com/benbjohnson/litestream)
- **mvSQLite:** v0.3.22 released 2026-04-30; the repo was last pushed 2026-10-02 and is not archived; about 1.6k stars. Commit groups remain experimental. — [mvsqlite releases](https://github.com/losfair/mvsqlite/releases); [mvsqlite README](https://github.com/losfair/mvsqlite)
- **Neon:**
  - Databricks announced the acquisition on 2025-05-14, reportedly about $1B, and launched Lakebase on Neon technology in June 2025. Lakebase GA on AWS is reported as Feb 3 (2026 per article timeline). — [SiliconANGLE](https://siliconangle.com/2025/05/14/databricks-buys-serverless-database-startup-neon-reported-1b/); [The New Stack](https://thenewstack.io/lakebase-is-databricks-fully-managed-postgres-database-for-the-ai-era/); [TechTarget](https://www.techtarget.com/searchDataManagement/news/366638723/Databricks-launches-PostgreSQL-Lakebase-to-aid-AI-developers)
  - Neon docs say Lakebase Postgres on Neon and on Databricks both run on the safekeeper/pageserver/object-storage architecture. — [Neon architecture overview](https://neon.com/docs/introduction/architecture-overview)
- **LiteFS:**
  - LiteFS Cloud was wound down, available until 2024-10-15, because "most LiteFS users don't use LiteFS Cloud". Fly recommended Litestream to Tigris/S3, or Turso for managed SQLite. — [Fly community: Sunsetting LiteFS Cloud](https://community.fly.io/t/sunsetting-litefs-cloud/20829)
  - The LiteFS docs say it is "not able to provide support or guidance for this product", pre-1.0, and warn against combining it with autostop/autostart. — [LiteFS docs](https://fly.io/docs/litefs/); [Fly community: status of LiteFS](https://community.fly.io/t/what-is-the-status-of-litefs/23883)
  - The last release was v0.5.14 (2025-04-22); the repo was last pushed 2026-05-11 and is not archived. — [LiteFS releases](https://github.com/superfly/litefs/releases)
- **LiteFS architecture context:**
  - A FUSE filesystem intercepts SQLite journal writes and forwards them to replicas. LiteVFS was the alternative where FUSE is unavailable.
  - Leadership comes from Consul. A third-party report attributes a March 2026 LiteFS primary-election issue to a degraded Fly Consul cluster (secondary source).
  - "Litestream is the more popular project. It's easier to deploy and easier to reason about."

  — [Litestream Revamped](https://fly.io/blog/litestream-revamped/); [Kuberns incident roundup (secondary)](https://kuberns.com/blogs/is-fly-io-good-for-production/)

### Inferences
- Fly's trajectory matters for this design space. It moved from LiteFS (FUSE, Consul leader, node-to-node streaming, stateful replicas) to Litestream v0.5 (object store as the only shared component, conditional-write leases, VFS read and write replicas). That is a vote for "object storage as the only coordination substrate" over a consensus service, accepting about 1 s durability lag for that product.
- Among the compared systems, SlateDB is the only one with a production-proven, documented fencing protocol on pure S3. Graft v2 and the Litestream VFS write mode are the closest SQLite-shaped analogues, but both are young (≤ 1 year in their current form).

### Gaps
- No public production-user list for Graft v2 was found.
- No confirmation was found of which SlateDB adopters use it as a primary store versus a cache or other component.
- No 2026 primary source was found on whether Neon's pageserver/safekeeper design changed under Databricks.
