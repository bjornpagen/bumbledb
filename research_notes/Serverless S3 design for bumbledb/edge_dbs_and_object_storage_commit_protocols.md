# Commit and Durability Mechanisms of Edge/Serverless Databases and Lakehouse Formats on Object Storage (as of 2026-10-09)

Scope note: Primary sources were fetched directly unless marked "(search excerpt)". Material from before 2024 is marked "(older)". "Inferences" are my reasoning, not source claims. Recurring sources:
- CF-DO-SQLite = https://blog.cloudflare.com/sqlite-in-durable-objects/ (2024)
- CF-Gates = https://blog.cloudflare.com/durable-objects-easy-fast-correct-choose-three/ (published Aug 3, 2021; older)
- CF-D1-RR = https://blog.cloudflare.com/d1-read-replication-beta/ (2025)
- AWS-CW = https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html

---

## 1. Cloudflare Durable Objects (SQLite-backed) and D1: actor model, output gates, local synchronous writes, WAL quorum replication, snapshots, PITR, and D1 replication/bookmarks

### Takeaway
Durable Objects hide durable-storage latency with three mechanisms: (1) one single-threaded instance per object, with SQLite executing synchronously against local disk in microseconds; (2) an output gate that lets code continue immediately but holds every outgoing message until the writes it depends on are confirmed; (3) a commit is "confirmed" once 3 of 5 cross-datacenter followers hold the WAL frames, not after an object-storage PUT. Object storage (R2) is only the lazy, batched (10 s / 16 MB) backing store plus snapshots, which also gives 30-day PITR. D1 is a thin routing layer over these SQLite DOs. Its read replicas receive the same WAL stream and give sequential consistency through Lamport-timestamp "bookmarks" that a replica waits on.

### Cited Findings

**Execution model / gates**
- A Durable Object is "a small server that can be addressed by a unique name". All messages for a name reach the same instance, and each request runs to completion before the next starts. A synchronous call blocks everything else in the object. — [CF-DO-SQLite](https://blog.cloudflare.com/sqlite-in-durable-objects/)
- Input gate (older, 2021): "While a storage operation is executing, no events shall be delivered to the object except for storage completion events". This prevents interleaving across `await` points, but two storage calls started by the same event can still race. — [CF-Gates](https://blog.cloudflare.com/durable-objects-easy-fast-correct-choose-three/)
- Output gate (older, 2021): while a write is in progress, "any new outgoing network messages will be held back until the write has completed". This covers both responses to clients and new outbound `fetch()` calls. — [CF-Gates](https://blog.cloudflare.com/durable-objects-easy-fast-correct-choose-three/)
- On write failure, held messages "will be discarded and replaced with errors" and the object "will be shut down and restarted from scratch". Clients therefore never see success for an unpersisted write. — [CF-Gates](https://blog.cloudflare.com/durable-objects-easy-fast-correct-choose-three/); restated for SQLite DOs in [CF-DO-SQLite](https://blog.cloudflare.com/sqlite-in-durable-objects/)
- Automatic write coalescing (older, 2021): writes issued without an intervening `await` "are automatically grouped together and stored atomically", which makes them all-or-nothing on power failure. "Writes will be coalesced (even if you await them)", so the output gate waits about one round trip rather than one per write. — [CF-Gates](https://blog.cloudflare.com/durable-objects-easy-fast-correct-choose-three/)
- The 2021 in-memory cache holds "up to several megabytes" in-process, and a `put()` completes almost instantly into cache. Escape hatches exist: `allowConcurrency`, `allowUnconfirmed`, and `noCache`. — [CF-Gates](https://blog.cloudflare.com/durable-objects-easy-fast-correct-choose-three/)
- In SQLite DOs, the runtime "holds the response until all storage writes relevant to the response have been confirmed". The response can be computed in parallel with confirmation, which the post credits with reducing latency. — [CF-DO-SQLite](https://blog.cloudflare.com/sqlite-in-durable-objects/)

**Synchronous local SQLite**
- `sql.exec()` returns a cursor synchronously, with no `await`. Local SSD is treated as an "L5 cache". Local queries "can complete in microseconds" versus networked DB calls "measured in milliseconds". The N+1 example works out to 101 queries × 5 ms = 505 ms over a network, versus negligible cost when SQLite runs as a library. — [CF-DO-SQLite](https://blog.cloudflare.com/sqlite-in-durable-objects/)

**Storage Relay Service (SRS): WAL capture, quorum, batching, snapshots**
- SQLite always runs in WAL mode. SRS hooks SQLite's VFS to intercept WAL writes. SRS replaced the older key/value persistence layer, which sat on a regional off-the-shelf database, and SRS had "powered D1 for over a year" at the time of the post. — [CF-DO-SQLite](https://blog.cloudflare.com/sqlite-in-durable-objects/)
- Each commit is forwarded immediately to **5 follower machines in different physical data centers**. It is **confirmed once ≥3 acknowledge**, at which point the data sits in follower buffers and not yet in object storage. — [CF-DO-SQLite](https://blog.cloudflare.com/sqlite-in-durable-objects/)
- Followers buffer changes on local disk and delete them once SRS reports them persisted to object storage. If that notice never arrives, a follower uploads the change itself after a timeout. Losing data would take "at least four different machines in at least three different physical buildings" failing at once. — [CF-DO-SQLite](https://blog.cloudflare.com/sqlite-in-durable-objects/)
- Uploads to object storage (R2, "tens or hundreds of milliseconds" latency) are batched into one object when either **10 seconds or 16 MB** is reached, "whichever happens first", to avoid floods of tiny objects. — [CF-DO-SQLite](https://blog.cloudflare.com/sqlite-in-durable-objects/)
- A full snapshot is uploaded when the logs since the last snapshot grow larger than the database. Recovery therefore downloads at most about 2× the DB size, and stored data is capped at about 2× because older data is deleted after each snapshot. The design was inspired by Litestream, though the implementation differs. — [CF-DO-SQLite](https://blog.cloudflare.com/sqlite-in-durable-objects/)
- Failover and fencing: if the host is unreachable, followers are asked to stop confirming writes for the old instance. A new instance starts only after **3 of 5 followers agree**, which prevents two instances from confirming conflicting writes. — [CF-DO-SQLite](https://blog.cloudflare.com/sqlite-in-durable-objects/)
- PITR: "any object can be reverted to the state it had at any point in time in the last 30 days". It is on by default. Old logs and snapshots are marked for deletion 30 days later, and restore replays the log from the last snapshot. — [CF-DO-SQLite](https://blog.cloudflare.com/sqlite-in-durable-objects/)
- Beta limits in the 2024 post: 1 GB per object, rising to 10 GB at GA. Planned pricing: rows written $1.00/M after 50 M included, rows read $0.001/M after 25 B, storage $0.20/GB-month after 5 GB. — [CF-DO-SQLite](https://blog.cloudflare.com/sqlite-in-durable-objects/)

**D1 on Durable Objects; read replication**
- D1 has three layers: the Worker binding, a stateless Worker that routes by database ID, and the SQLite-backed DO that executes SQL on SRS. A non-replicated DB is exactly one DO. — [CF-D1-RR](https://blog.cloudflare.com/d1-read-replication-beta/)
- The leader streams each WAL entry to replicas at the same time as it sends it to the durability followers. Replicas receive frames before confirmation, write them immediately, and keep them hidden from SQLite until the primary confirms. New replicas boot from the latest snapshot and replay the log to the primary's last committed bookmark. Replication is asynchronous. — [CF-D1-RR](https://blog.cloudflare.com/d1-read-replication-beta/)
- Bookmarks are Lamport timestamps. Each write yields "a new bookmark with a value greater than any other bookmark for that database". — [CF-D1-RR](https://blog.cloudflare.com/d1-read-replication-beta/)
- Sessions API: `env.DB.withSession(bookmark | "first-unconstrained" | "first-primary")` and `session.getBookmark()`. The example returns the bookmark in an `x-d1-bookmark` header so the client can carry it on later requests. Writes sent to a replica are forwarded to the primary. A replica that receives a bookmark ahead of its own state waits (`waitForBookmark`) until it catches up. — [CF-D1-RR](https://blog.cloudflare.com/d1-read-replication-beta/)
- Semantics: sessions provide sequential consistency, meaning operations "executed in the order in which you write them in your code". This includes read-your-writes, writes-follow-reads, and a total order of writes. Queries outside a session are only guaranteed "read committed". — [CF-D1-RR](https://blog.cloudflare.com/d1-read-replication-beta/)
- Replica confirm lag (primary confirm to replica confirm) is 30–75 ms. Examples: primary ENAM → WNAM 45 ms, WEUR 55 ms, EEUR 67 ms; primary WNAM → ENAM 30 ms, WEUR/EEUR 75 ms. Lag correlates with inter-DC RTT. The post gives no end-user latency numbers. There is a static set of replicas per supported region, at no extra cost. The responses carry `meta.served_by_region` and `meta.served_by_primary`. — [CF-D1-RR](https://blog.cloudflare.com/d1-read-replication-beta/)
- D1 limits:
  - 10 GB max DB on Workers Paid ("cannot be further increased"); 500 MB on Free.
  - 50,000 DBs and 1 TB per account (Paid).
  - Time Travel: 30 days Paid, 7 days Free; restores limited to 10 per 10 minutes.
  - 30 s max query duration (this also applies to a whole `db.batch()`); 100 KB statements; 100 bound params; 2 MB rows.
  - Each DB is **single-threaded, processing one query at a time**: about 1,000 qps at 1 ms per query, or 10 qps at 100 ms. Excess requests queue and then get an "overloaded" error. Each read replica is a separate DO with the same limits.
  - [D1 limits](https://developers.cloudflare.com/d1/platform/limits/)

### Inferences
- The Cloudflare design shows that the commit-ack latency of an "S3-authoritative" system does not have to include an S3 PUT. It can instead depend on a replicated log quorum (3/5 cross-DC), with S3 as a lazily batched secondary. That works only because Cloudflare runs a follower fleet. A design without a fleet must either pay S3 (or S3 Express) latency per group commit, or add its own small quorum/sequencer service.
- The output gate amounts to "speculative execution with externalized-effect fencing". The app sees its own writes instantly (local SQLite), and only network egress waits for durability. For bumbledb, the analogue is: apply the transaction to local state immediately, pipeline the next transactions, and block only the client acknowledgement (and any outbound side effects) on the durable group-commit future. If the durable write fails, the local state must be discarded (Cloudflare restarts the object), so local speculative state cannot be trusted past a failed commit.
- The fencing step (a majority of followers must stop accepting the old leader before a new one starts) is the same safety property that a CAS-based epoch/lease on S3 would provide (see SlateDB below).

### Gaps
- No measured end-to-end write-ack latency (p50/p99) for SQLite DOs or D1 was found in these sources. The posts give only qualitative claims plus D1 replica lag numbers.
- I did not confirm the date of SQLite DO GA or whether the 10 GB per-object limit and the planned pricing took effect as stated. The D1 limits page confirms 10 GB for D1 itself.
- I found no 2025–2026 primary source that updates the input-gate semantics for SQLite DOs (the 2021 post shows a 2026 modified date, but its content reads as the original).
- Whether D1 read replication left beta by Oct 2026 was not verified.

---

## 2. Delta Lake and Apache Iceberg commit protocols on S3: put-if-absent log vs. pointer CAS, pre-2024 DynamoDB workarounds, the S3 conditional-write era, and contention/retry

### Takeaway
Delta commits by atomically creating the next-numbered `_delta_log/<v>.json`, which needs only put-if-absent. Iceberg commits by atomically swapping a "current metadata" pointer from version V to V+1, a check-and-put done by a catalog/metastore. Before S3 had conditional writes, Delta needed the S3DynamoDBLogStore (DynamoDB conditional PutItem as the mutual-exclusion point) for multi-writer safety. S3 added `If-None-Match: *` (Aug 2024) and `If-Match` ETag CAS (Nov 25, 2024; extended to CopyObject Oct 2025), which provide both primitives natively. Both formats use optimistic concurrency: losers re-read, validate against what changed, and either rebase/retry (blind appends always can) or fail with a conflict.

### Cited Findings

**S3 conditional-write primitive (AWS)**
- `If-None-Match: *` prevents overwrites by validating that no object with the same key exists. It works on PutObject, CompleteMultipartUpload, and CopyObject, and requires SigV4. — [AWS-CW](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html)
- "If multiple conditional writes or copies occur for the same object name, the first write operation to finish succeeds. Amazon S3 then fails subsequent writes with a 412 Precondition Failed response." — [AWS-CW](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html)
- A `409 Conflict` can occur if a concurrent delete succeeds first. PutObject may be retried after a 409. CompleteMultipartUpload must restart from CreateMultipartUpload. — [AWS-CW](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html)
- `If-Match: <etag>` succeeds only if the current object's ETag matches (otherwise 412). It needs `s3:PutObject` plus `s3:GetObject`. Concurrent requests can also receive `409 Conflict`, and a concurrent delete yields `404`. — [AWS-CW](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html)
- In-progress multipart uploads are not considered. A conditional PUT racing an MPU wins, and the later CompleteMultipartUpload fails with 412. — [AWS-CW](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html)
- Bucket policies can mandate conditional headers via the `s3:if-none-match` / `s3:if-match` condition keys. — [AWS-CW](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html); [Simon Willison (search excerpt)](https://simonwillison.net/b/8329)
- If-Match was announced Nov 25, 2024, for PutObject and CompleteMultipartUpload in general purpose and directory buckets, at no additional charge, pitched as "offloading compare and swap operations to S3". — [AWS What's New (China mirror)](https://www.amazonaws.cn/en/new/2024/amazon-s3-adds-new-functionality-for-conditional-writes/); [AWS What's New](https://aws.amazon.com/about-aws/whats-new/2024/11/amazon-s3-functionality-conditional-writes/)
- If-None-Match on S3 dates to August 2024. — [InfoQ, Aug 2024 (search result)](https://www.infoq.com/news/2024/08/amazon-s3-conditional-writes)
- Conditional writes were extended to CopyObject in October 2025. — [AWS What's New, Oct 2025](https://aws.amazon.com/about-aws/whats-new/2025/10/amazon-s3-conditional-write-functionality-copy-operations)
- Pitfall (secondary source): a successful conditional PUT whose response is lost, followed by an SDK retry, returns 412 against your *own* object. The suggested fix is to embed a writer/txn UUID in the payload and read the object back on 412. — [mohakchugh blog (secondary)](https://mohakchugh.is-a.dev/blog/s3-conditional-writes-distributed-coordination)

**Delta Lake**
- Storage requirements are atomic visibility, **mutual exclusion** ("Only one writer must be able to create (or rename) a file at the final destination"), and consistent listing. The current docs still say S3 single-cluster mode is the default and warn that "Concurrent writes to the same Delta table on S3 storage from multiple Spark drivers can lead to data loss". — [Delta storage docs](https://docs.delta.io/latest/delta-storage.html)
- Each commit is a JSON file in `_delta_log` named by the next version. "Delta files are the unit of atomicity for a table". — [Delta PROTOCOL.md](https://github.com/delta-io/delta/blob/master/PROTOCOL.md)
- Checkpoints may be created only after the corresponding delta file is written. Multi-part checkpoints are not atomic, so readers must ignore incomplete ones. Optional log-compaction files `<x>.<y>.compacted.json` cover a version range. `<v>.crc` checksum files are written only after the delta file and must not be overwritten. — [Delta PROTOCOL.md](https://github.com/delta-io/delta/blob/master/PROTOCOL.md)
- Catalog-managed commits (the successor to "coordinated commits"):
  - Commits are staged in `_delta_log/_staged_commits`, and a catalog is "the source of truth" for which staged files are ratified versions. "The mere existence of a staged commit does not mean that the file has been ratified".
  - The catalog ratifies versions in order and at most once.
  - Ratified commits are "published" (copied) to `_delta_log/<v>.json` in order.
  - The spec also mentions "relying on PUT-if-absent primitives to facilitate the ratification and publication all in one step".
  - [Delta PROTOCOL.md](https://github.com/delta-io/delta/blob/master/PROTOCOL.md)
- Pre-conditional-writes multi-cluster S3 support (older, 2022), the S3DynamoDBLogStore introduced in Delta 1.2:
  - A DynamoDB table `delta_log` keyed by (`tablePath` HASH, `fileName` RANGE) records which commits were attempted or completed.
  - The writer first writes about 200 bytes of metadata to DynamoDB with a conditional PutItem (only if absent), then writes the log file to S3.
  - Readers and writers recover the latest incomplete entry (at most one exists) before proceeding.
  - Mixing this LogStore with plain writers risks "data loss".
  - [delta.io 2022 blog](https://delta.io/blog/2022-05-18-multi-cluster-writes-to-delta-lake-storage-in-s3/)
- The current docs add that a temp file under `_delta_log/.tmp/` holds a copy of the commit, TTL is on `expireTime` (default one day after completion), and "only the latest temp file will ever be used during recovery of a failed commit". — [Delta storage docs](https://docs.delta.io/latest/delta-storage.html)
- Optimistic concurrency control in three steps: read the latest version, write data files, then validate and commit. On conflict the write fails with a concurrent modification exception. INSERT never conflicts with INSERT, and INSERT never conflicts with compaction. UPDATE, DELETE, MERGE, and compaction "can conflict" depending on whether they touch the same set of files. The named exceptions are ConcurrentAppendException, ConcurrentDeleteReadException, ConcurrentDeleteDeleteException, MetadataChangedException, ConcurrentTransactionException, and ProtocolChangedException. — [Delta concurrency control](https://docs.delta.io/latest/concurrency-control.html)
- Rust ecosystem: arrow-rs `object_store` PR #6682 "Support native S3 conditional writes" (merged Nov 8, 2024) added `PutMode::Create` / copy-if-not-exists on native AWS S3, opt-in through the `s3_conditional_put` / `s3_copy_if_not_exists` configs. — [arrow-rs PR #6682](https://github.com/apache/arrow-rs/pull/6682)
- Before that, delta-rs used a DynamoDB lock to get put-if-absent behavior on S3. — [brokenco.de 2023 (older, search excerpt)](https://brokenco.de/2023/11/29/locking-with-deltalake.html)

**Apache Iceberg**
- "An atomic swap of one table metadata file for another provides the basis for serializable isolation." Writers create metadata optimistically and commit "by swapping the table's metadata file pointer from the base version to the new version". If the base is stale, "the writer must retry the update based on the new current version". — [Iceberg spec](https://iceberg.apache.org/spec/) (fetched from [format/spec.md](https://github.com/apache/iceberg/blob/main/format/spec.md))
- Metastore tables: the pointer is stored in a metastore or database "updated with a check-and-put operation" that "validates that the version of the table that a write is based on is still current". If the swap fails, "another writer has already created V+1" and the writer returns to step 1. — [Iceberg spec](https://iceberg.apache.org/spec/)
- File-system tables use atomic rename to `v<V+1>.metadata.json`, which needs a filesystem like HDFS. "This file system based scheme to commit a metadata file is deprecated and will be removed in version 4 of this spec." — [Iceberg spec](https://iceberg.apache.org/spec/)
- Commit conflict resolution:
  - "only one commit will succeed", and "in most cases, the failed commit can be applied to the new current version of table metadata and retried".
  - "Append operations have no requirements and can always be applied". Replace and delete operations must verify that the files they delete "are still in the table". Expression deletes can always be applied. Schema and partition-spec changes must validate that the schema did not change.
  - [Iceberg spec](https://iceberg.apache.org/spec/)
- Retry cost: a snapshot is optimistically assigned the next sequence number, which is reassigned on retry. Manifests inherit the sequence number, so "only the manifest list must be rewritten" on retry and new manifests are reused. — [Iceberg spec](https://iceberg.apache.org/spec/)
- Default retry knobs:
  - `commit.retry.num-retries`=4, `min-wait-ms`=100, `max-wait-ms`=60000, `total-timeout-ms`=1800000.
  - Status check after a lost connection: `commit.status-check.num-retries`=3 with 1 s to 60 s waits, after which the commit fails with an *unknown commit state*.
  - `commit.manifest-merge.enabled`=true.
  - [Iceberg configuration](https://iceberg.apache.org/docs/latest/configuration/)
- Community commentary (secondary): a Substack post floats replacing the catalog with a single S3 conditional PUT on the metadata pointer, and cautions about isolation subtleties. — [juhache Substack (search excerpt)](https://juhache.substack.com/p/rip-iceberg-catalogs)

### Inferences
- Delta-style "create next numbered object with If-None-Match" gives an append-only, linearizable log with no overwrite, so the ambiguity about which ETag to compare against never arises. Finding the head requires a LIST (or a hint plus probing), and the log needs periodic checkpoints and compaction. Iceberg-style "one pointer object + If-Match" makes the head a single GET, but every commit rewrites the pointer and contends on one key. For a small embedded DB, a hybrid is natural: numbered log segments created with If-None-Match (the commit point), plus an occasionally-CAS'd manifest/checkpoint pointer as a hint. This is roughly what SlateDB and Delta catalog-managed commits converge on.
- Both formats treat "unknown commit outcome" (lost response) as a first-class state. Iceberg has explicit status-check retries. Any S3-CAS commit path needs idempotent commit identity (a txn UUID inside the object) to resolve a 412 after a timeout.
- Both formats get cheap rebase by keeping the commit object tiny and reusing large data files and manifests across retries. Only the small log entry or manifest list is rewritten. Contention cost is therefore an extra small PUT plus a GET per retry, plus validation logic.

### Gaps
- I could not find an official Delta Lake (Spark) release note stating that S3 multi-writer mode now uses S3 `If-None-Match` instead of DynamoDB. The current docs.delta.io storage page still documents only the DynamoDB LogStore for multi-cluster S3. A secondary blog asserts that delta-rs dropped its DynamoDB requirement. That is unverified; the delta-rs S3 docs URL I tried returned 404.
- I did not verify whether any Iceberg catalog (REST, Glue, S3 Tables) or S3FileIO uses S3 `If-Match` for the pointer swap. Today the swap is done by the catalog.
- No published numbers were found for Delta or Iceberg commit latency or throughput under contention on S3.

---

## 3. turbopuffer: WAL on object storage, group commit window, write latency, cache hierarchy, cold vs. warm queries, CAS usage

### Takeaway
turbopuffer acknowledges a write only after it is a committed WAL file in object storage. Concurrent writes to a namespace are merged into at most one WAL entry per second (group commit), giving a published p50 write latency of 165 ms for a 500 kB write. Committed-but-unindexed data is query-visible by scanning the WAL tail. Reads are served from an NVMe+memory cache (warm p50 14 ms), and cold queries pay 3–4 S3 round trips (p50 about 500–874 ms). All concurrency control is delegated to object storage (CAS). Their 2026 queue post is a compact case study of CAS + group commit + a stateless broker.

### Cited Findings
- Architecture: stateless Rust binaries (`./tpuf`) read namespaces from object storage, which is the source of truth. After the first query a namespace is cached on NVMe SSD, with memory caching above that. Queries for a namespace are routed to the same node for cache locality, but any node can serve any namespace. — [turbopuffer architecture](https://turbopuffer.com/docs/architecture)
- WAL: each namespace has a prefix with a WAL directory (`s3://tpuf/{namespace_id}/wal`). "Every write adds a new file to the WAL". A successful write means the data is durably in object storage. The architecture diagram labels a "CAS commit point". — [turbopuffer architecture](https://turbopuffer.com/docs/architecture)
- Group commit: each namespace commits **at most 1 WAL entry per second**. Concurrent writes are merged into one entry, and a batch starting within 1 s of the previous commit may wait up to 1 s. — [turbopuffer architecture](https://turbopuffer.com/docs/architecture)
- Write performance: about 10,000+ vectors/s throughput, and **write latency p50 = 165 ms for a 500 kB write**. — [turbopuffer architecture](https://turbopuffer.com/docs/architecture)
- Query latency (1M docs):
  - Cold p50 = 874 ms; warm p50 = 14 ms.
  - Each object-storage round trip takes about 100 ms. A cold vector query needs 3–4 of them (metadata, then filter/centroid/unindexed-WAL, then cluster data), about 400–500 ms minimum.
  - Clients can send a pre-flight query to warm the cache.
  - [turbopuffer architecture](https://turbopuffer.com/docs/architecture)
- Consistency:
  - Strong by default: consistent reads cost about 10 ms to check the WAL, and a strongly consistent cold query adds an object-storage round trip.
  - Eventual consistency offers sub-10 ms warm reads with worst-case staleness of about 1 hour.
  - Committed WAL is indexed asynchronously by `./tpuf indexer` processes from a queue in object storage. Unindexed data is still searchable via an exhaustive scan of recent WAL.
  - [turbopuffer architecture](https://turbopuffer.com/docs/architecture)
- Guarantees:
  - "Writes are committed to object storage upon successful return".
  - "Over 99.8% of queries return consistent data", with about 100 ms staleness possible during scaling or failover.
  - Once a namespace has more than 128 MiB of outstanding unindexed writes, new writes stay invisible until indexed (up to about 1 h).
  - Upserts apply atomically. Conditional writes are evaluated atomically with the write and behave as Serializable. `patch_by_filter` and `delete_by_filter` are Read Committed. There are no general read-write transactions.
  - "All concurrency control is delegated to object storage", and object storage is the only stateful dependency. turbopuffer chooses consistency over availability when object storage is unreachable.
  - [turbopuffer guarantees](https://turbopuffer.com/docs/guarantees)
- Object-storage queue case study (Feb 12, 2026), with design steps:
  1. A single `queue.json` rewritten in full with CAS by pushers and workers. This is "production grade" up to about 1 req/s, a limit attributed to GCS per-object writes, and replacing the file "can take up to 200ms".
  2. Group commit: buffer requests while a CAS write is in flight and flush them as the next CAS write. Throughput is then bounded by bandwidth instead of the ~200 ms latency, but contention remains: "We need fewer writers".
  3. A stateless broker runs one group-commit loop for all clients and does not ack until the group commit lands in object storage. One broker can serve "hundreds or thousands of clients".
  4. HA: the broker's address is stored in `queue.json`. A new broker is started if requests time out, and "CAS ensures correctness even with two brokers" because the stale broker's CAS fails. Workers heartbeat timestamps into the file for job takeover.
  
  The result is FIFO, at-least-once delivery and about 10× lower tail latency than the prior sharded design. — [turbopuffer blog: object-storage queue](https://turbopuffer.com/blog/object-storage-queue)

### Inferences
- turbopuffer is the cleanest "pure S3, no sequencer" reference: ack = durable CAS'd WAL object, about 165 ms p50 including a 1 s/namespace commit-rate ceiling. That fits a vector DB's write profile. An embedded OLTP-ish DB would want a much shorter group-commit window (WarpStream shows 25–50 ms windows work).
- The queue post's progression (CAS → group commit → single broker per object → CAS-fenced broker failover) maps directly onto a "single-writer leader per database, fenced by CAS on S3" design. The CAS on the commit object is the fencing token, so no separate lock service is needed for safety, only for liveness and leader discovery.
- The read-side lesson: strong consistency on a cache hit still costs one small metadata round trip (about 10 ms) to learn the WAL head. A bookmark/version token from the client could avoid even that when the cache is known current.

### Gaps
- turbopuffer does not document the exact CAS mechanics: whether WAL entries are created with If-None-Match on a sequential key, or whether a manifest is updated with If-Match. Only the "CAS commit point" diagram label is public.
- No p99 write latency was published. No per-cloud (S3 vs. GCS) commit latency breakdown was found.

---

## 4. WarpStream: stateless agents, batching to object storage, metadata store as sequencer, latency/cost trade-offs

### Takeaway
WarpStream agents are stateless. Each buffers produce requests (default 250 ms or 4 MB), writes one multi-partition file to object storage, then commits file metadata to a control-plane "virtual cluster" (a replicated state machine) that sequences records and assigns offsets, and only then acks the client. Defaults give about 400–500 ms p99 produce latency. With S3 Express One Zone (writes to a quorum of zonal buckets) and a 25–50 ms batch window, p99 drops below 150 ms. "Lightning Topics" (Feb 2026) take the sequencer off the critical path, acking after the object-storage journal write, for p50 33 ms / p99 under 50 ms, at the cost of no offsets in the ack, no idempotence/transactions, and loss of external consistency.

### Cited Findings
- The Agent is a "single stateless binary" that talks only to object storage and the Cloud Metadata Store. Any agent can lead any topic or act as coordinator. Each file can hold records from many topics and partitions, and agents write "a few files per second". — [WarpStream architecture docs](https://docs.warpstream.com/warpstream/overview/architecture)
- Each Virtual Cluster is a replicated state machine mapping object-storage files to offset ranges. Metadata operations are journaled to log storage before a replica executes them and acks the agent, which then acks the client. Background agents compact small files. — [WarpStream architecture docs](https://docs.warpstream.com/warpstream/overview/architecture)
- Cost rationale: cross-AZ replication costs an effective $0.05/GB at AWS retail prices, which WarpStream avoids by using free EC2↔S3 networking. 100 ms file intervals would cost about $130/month per partition in S3 PUTs alone, which is why files are shared across partitions. — [WarpStream architecture docs](https://docs.warpstream.com/warpstream/overview/architecture)
- Classic produce path:
  1. Buffer until the batch timeout (**250 ms default**) or size threshold (**4 MB default**).
  2. Write the file to object storage.
  3. Commit file metadata to the Control Plane, "which sequences the records and assigns offsets".
  4. Ack with offsets.
  
  — [WarpStream blog, Feb 4, 2026](https://www.warpstream.com/blog/the-art-of-being-lazy-log-lower-latency-and-higher-availability-with-delayed-sequencing)
- Low-latency breakdown (50 ms batch timeout, S3EOZ): about 30 ms buffering, about 20 ms writing to a **quorum of S3 Express One Zone buckets**, and about 50 ms for the control-plane commit, which is the largest share because the control plane itself batches briefly. — [WarpStream blog, Feb 2026](https://www.warpstream.com/blog/the-art-of-being-lazy-log-lower-latency-and-higher-availability-with-delayed-sequencing)
- Lightning Topics:
  - The agent writes the file into a per-agent "sequence" folder of about 1,000 consecutive files and acks immediately. The control-plane commit is asynchronous, and a background "slow path" scans for and replays uncommitted files.
  - The relaxations: the produce response returns offset 0, idempotent and transactional producers are rejected, and a later-acked record can get a lower offset.
  - The docs say Lightning Topics provide "the exact same durability guarantees as regular topics".
  - [WarpStream blog, Feb 2026](https://www.warpstream.com/blog/the-art-of-being-lazy-log-lower-latency-and-higher-availability-with-delayed-sequencing); [Low latency clusters docs](https://docs.warpstream.com/warpstream/kafka/advanced-agent-deployment-options/low-latency-clusters)
- Latency numbers:
  - Default config: p99 produce latency 400 ms.
  - Classic on S3EOZ: median 105 ms, p99 170 ms.
  - Lightning on S3EOZ: median 33 ms, p99 50 ms (batch timeout 25 ms).
  - [WarpStream blog, Feb 2026](https://www.warpstream.com/blog/the-art-of-being-lazy-log-lower-latency-and-higher-availability-with-delayed-sequencing)
  - An earlier S3EOZ benchmark reported p99 169 ms and median 105 ms, about 3× lower than S3 Standard. — [WarpStream S3EOZ benchmark (search excerpt)](https://www.warpstream.com/blog/warpstream-s3-express-one-zone-benchmark-and-total-cost-of-ownership)
- Docs setup table:

  | Setup | Produce p50 / p99 | E2E p50 / p99 |
  |---|---|---|
  | S3 Standard, 250 ms batch | 250 / 500 ms | 500 / 900 ms |
  | S3 Express, 50 ms batch | <80 / <150 ms | <200 / <400 ms |
  | S3 Express, 25 ms batch, Lightning | <35 / <50 ms | <200 / <400 ms |

  The batch timeout can go as low as 25 ms (`-batchTimeout` / `WARPSTREAM_BATCH_TIMEOUT`). Lowering it has "no impact on durability or correctness". Produce is never acked before data is "durably persisted in object storage". — [Low latency clusters docs](https://docs.warpstream.com/warpstream/kafka/advanced-agent-deployment-options/low-latency-clusters)
- Cost:
  - S3 Express PUTs are about 1/5 the price of S3 Standard PUTs.
  - Cutting the batch timeout from 250 to 50 ms raises ingestion PUT cost 2× on S3 Express versus 5× on Standard.
  - S3 Express raises costs about 20% on average.
  - DynamoDB- or Spanner-backed control planes are "more expensive" and not recommended for high volume.
  
  — [Low latency clusters docs](https://docs.warpstream.com/warpstream/kafka/advanced-agent-deployment-options/low-latency-clusters)

### Inferences
- WarpStream separates *durability* (object-storage write, which is parallel, stateless, and cheap to batch) from *ordering* (a small, strongly consistent sequencer). Once the S3 write is fast (S3EOZ quorum, about 20 ms), the sequencer hop (about 50 ms) dominates. Removing it from the ack path cuts latency by about 70%, but only if clients do not need the assigned position in the ack.
- For bumbledb, the trade-off appears directly. If a transaction's commit version must be known at ack time (for read-your-writes tokens or serializable validation), the sequencing step must be on the critical path, whether as an S3 CAS or a sequencer RPC. If ack can mean "durable, will be ordered", one could journal first and sequence later. That breaks external consistency, so it is a poor fit for a database with transactions.

### Gaps
- The docs do not specify the S3EOZ quorum size or ack rule (how many zonal buckets, how many must ack).
- The internal replication of the Cloud Metadata Store (consensus protocol, journal storage) is not described in the fetched pages.

---

## 5. S2 and similar "durable streams / databases on object storage" (S2, Neon, SlateDB)

### Takeaway
S2 offers a serverless stream API where every append is durable in object storage before ack. "Standard" streams sit on S3 Standard (about 400–500 ms), and "Express" streams use a quorum of three S3 Express One Zone buckets in different AZs (about 40–50 ms). It provides optimistic (expected-sequence-number) and pessimistic (fencing-token) single-writer control. Neon shows the classic alternative: ack after a Paxos quorum of safekeepers, with object storage written asynchronously. SlateDB, an embedded LSM on object storage, shows the embedded-DB pattern: CAS'd numbered manifests, a `writer_epoch` fenced by CAS-writing the next WAL SST, and an opt-out `await_durable` flag.

### Cited Findings

**S2 (s2.dev)**
- The Dec 20, 2024 launch post describes S2 as a diskless, multi-tenant, serverless stream store in which "all writes will be safe in S3 with regional durability before being acknowledged".
  - Storage classes: **Standard**, backed by S3 Standard; **Express**, backed by "a quorum of three S3 Express One Zone buckets".
  - End-to-end p99 is under 500 ms for Standard and under 50 ms for Express.
  - S2 assigns sequence numbers ("S2 will durably sequence all records"), and a strongly consistent check-tail exists.
  - Writers can be pessimistic, using a fencing token, or optimistic, supplying the expected sequence number.
  - Limits: 125 MiB/s write per stream; 500 MiB/s for recent reads from memory.
  - [S2 intro blog](https://s2.dev/blog/intro)
- Durability docs (search excerpt): ack latency "within 400ms" (Standard) and "within 40ms" (Express). "Every write to S2 is durable on object storage before it is acknowledged. This applies to both the managed service and s2-lite". Express writes are durable "across a quorum of three zonal buckets". Appends are atomic per batch. An AppendSession pipelines batches and acks them in order, and if any batch fails, later batches will not become durable. — [S2 durability & consistency docs (search excerpt)](https://s2.dev/docs/concepts/durability-consistency)
- Fetched docs content: "Stream operations are linearizable". If an append has been acknowledged, any subsequent read or check-tail reflects it. Time-to-first-byte for records written in the last 20 s on Express is single-digit ms, and up to 200 ms otherwise. Check-tail takes single-digit ms. — [S2 docs](https://s2.dev/docs/concepts/durability-consistency)
- s2-lite stores records in SlateDB with a 50 ms default flush interval for remote buckets. Shorter intervals cut append latency but add API calls. — [S2 durability docs mirror (search excerpt)](https://www.mintlify.com/s2-streamstore/s2/concepts/durability)

**Neon (ack-after-quorum, object storage async)**
- "the compute node streams WAL to the storage layer" over the network to multiple safekeepers. "A transaction is considered committed once a quorum of safekeepers has acknowledged the WAL record" (Paxos). Safekeepers batch WAL flushes, and commit latency depends mainly on quorum network RTTs. — [Neon architecture overview](https://neon.com/docs/introduction/architecture-overview)
- Pageservers materialize pages from committed WAL off the commit path. Pages are "persisted into object storage asynchronously". Object storage is "the last line of defense". Object-storage reads "may take hundreds of milliseconds" but happen only inside pageservers, never on the hot query path. — [Neon architecture overview](https://neon.com/docs/introduction/architecture-overview)

**SlateDB (embedded LSM on object storage; RFC is 2024, older relative to some sources)**
- Manifests are numbered files, and the highest ID is current. "All updates to the manifest will be done using compare-and-swap (CAS)". IDs are "monotonically increasing and contiguous". On CAS failure "the client must retry the entire process". Manifest updates are kept small and infrequent because "Conflict can lead to starvation". — [SlateDB manifest RFC](https://github.com/slatedb/slatedb/blob/main/rfcs/0001-manifest.md)
- Zombie fencing: "using CAS to ensure each SST is written exactly one time". `writer_epoch` is "transactionally incremented by a writer on startup". The new writer writes an empty SST with the new epoch to the next WAL SST ID via CAS. A writer that finds its slot taken by a higher epoch "has been fenced" and halts. The compactor has its own `compactor_epoch`. — [SlateDB manifest RFC](https://github.com/slatedb/slatedb/blob/main/rfcs/0001-manifest.md)
- Pre-conditional-write S3 fallback: a two-phase write (temp object, then intent in DynamoDB, then copy to the final name, then mark committed), costing 2 S3 PUTs, 2 DynamoDB writes, and 1 S3 DELETE per SST. Durability is reached after 1 S3 write plus 1 DynamoDB write. Estimated manifest round trip: "250-500ms". Object versioning was rejected because S3 Express One Zone lacks versioning. — [SlateDB manifest RFC](https://github.com/slatedb/slatedb/blob/main/rfcs/0001-manifest.md)
- A put's future completes once the data is durably persisted. `put_with_options` with `await_durable: false` trades durability for latency. Writes are batched and memtables are flushed periodically as SSTs on a configurable interval. — [slatedb README on docs.rs (search excerpt)](https://docs.rs/crate/slatedb/0.15.0)
- A secondary description of SlateDB fencing says fence objects must not be garbage-collected while they still fence stale writers. — [Vanlightly post mirror (search excerpt; secondary)](https://nitter-canary.kareem.one/vanlightly)

### Inferences
- S2 Express and WarpStream S3EOZ independently converged on "quorum-write to 3 single-AZ S3 Express buckets" for about 20–40 ms durable acks with multi-AZ durability. This is the closest thing to Cloudflare's 3-of-5 follower quorum that needs no self-run storage fleet. S3 Express One Zone directory buckets are zonal, so a quorum is what restores AZ-failure tolerance (inference; the sources state the quorum but not the rationale).
- The quorum write gives durability but not a total order or CAS by itself. Ordering then comes from a single writer (S2's sequencer, a lease-holder) or a sequencer service (WarpStream). A small embedded DB with one writer per database could get order from the writer's lease and durability from the quorum write, keeping S3 Standard as the long-term authority (compaction/snapshots), in the style of Cloudflare and Neon.
- SlateDB's "fence by CAS-claiming the next log slot" is directly reusable. If every WAL object is created with `If-None-Match: *` at `wal/<seq>`, a zombie writer's next append collides and fails without any lease service. Epochs in the manifest make the takeover explicit.

### Gaps
- S2's internal sequencer design is not described in the fetched pages: who orders appends across the three Express buckets, and how many of the three must ack. Exact fencing-token and `match_seq_num` API semantics were not fetched.
- The number of safekeepers and quorum size for Neon were not stated on the fetched page. I also did not cover post-Databricks-acquisition (2025) changes to Neon.
- No primary measured WAL flush latency numbers were found for SlateDB.
- Other 2025–2026 systems that might be relevant were not researched: AutoMQ, Bufstream, Litestream's 2025 rewrite with S3 CAS leases, LiteFS, Turso, Chroma wal3. The coordinator may want a separate pass on Litestream/LiteFS, which are the closest analogues for SQLite-on-S3.

---

## 6. Cross-cutting patterns: group commit, ack point, sequencer vs. pure S3 CAS, single-writer leases, read-your-writes tokens

### Takeaway
All of these systems hide object-storage latency by (a) amortizing it with group commit (Cloudflare 10 s/16 MB to R2; turbopuffer 1 entry/s/namespace; WarpStream 25–250 ms or 4 MB), and (b) moving the ack point to something faster than an S3 Standard PUT (a 3/5 cross-DC follower quorum at Cloudflare; a safekeeper Paxos quorum at Neon; a 3-bucket S3 Express quorum at S2 and WarpStream), or else paying about 100–200 ms per commit (turbopuffer, S2 Standard). Correctness comes from one linearization point per log: a CAS/put-if-absent on S3 (Delta, turbopuffer, SlateDB), a check-and-put in a catalog (Iceberg, Delta catalog-managed commits), or a small consensus-backed sequencer (WarpStream, Cloudflare followers). Fencing of stale writers and bookmark-style version tokens complete the picture.

### Cited Findings

**Group commit / batching windows**
- Cloudflare SRS batches WAL to object storage at 10 s or 16 MB, while per-commit durability comes from the follower quorum. — [CF-DO-SQLite](https://blog.cloudflare.com/sqlite-in-durable-objects/)
- Cloudflare's legacy KV storage coalesces all writes issued before the output gate releases into one atomic batch. — [CF-Gates](https://blog.cloudflare.com/durable-objects-easy-fast-correct-choose-three/)
- turbopuffer: at most 1 WAL entry per second per namespace, merging concurrent writes. — [turbopuffer architecture](https://turbopuffer.com/docs/architecture)
- turbopuffer's queue buffers requests while a CAS write is in flight and flushes them as the next write. — [turbopuffer queue blog](https://turbopuffer.com/blog/object-storage-queue)
- WarpStream: 250 ms / 4 MB by default, tunable down to 25 ms. A shorter window raises PUT cost (2× on S3EOZ, 5× on Standard for 250→50 ms). — [WarpStream blog](https://www.warpstream.com/blog/the-art-of-being-lazy-log-lower-latency-and-higher-availability-with-delayed-sequencing); [WarpStream low-latency docs](https://docs.warpstream.com/warpstream/kafka/advanced-agent-deployment-options/low-latency-clusters)
- Neon safekeepers batch WAL flushes. — [Neon architecture](https://neon.com/docs/introduction/architecture-overview)

**Ack point and resulting latencies**

| System | Ack after | Published latency | Source |
|---|---|---|---|
| Cloudflare DO SQLite | ≥3 of 5 cross-DC followers | No number published; replica lag 30–75 ms gives a scale | [CF-DO-SQLite](https://blog.cloudflare.com/sqlite-in-durable-objects/); [CF-D1-RR](https://blog.cloudflare.com/d1-read-replication-beta/) |
| Neon | Paxos quorum of safekeepers | No number published | [Neon](https://neon.com/docs/introduction/architecture-overview) |
| turbopuffer | CAS-committed WAL object on S3/GCS | p50 165 ms for 500 kB | [turbopuffer](https://turbopuffer.com/docs/architecture) |
| S2 Standard | Durable on S3 Standard | ≤400 ms ack / <500 ms e2e p99 | [S2 intro](https://s2.dev/blog/intro); [S2 docs (excerpt)](https://s2.dev/docs/concepts/durability-consistency) |
| S2 Express | Quorum of 3 S3EOZ buckets | ≤40 ms ack / <50 ms p99 | [S2 intro](https://s2.dev/blog/intro); [S2 docs (excerpt)](https://s2.dev/docs/concepts/durability-consistency) |
| WarpStream Classic | Object-storage write + control-plane commit | 250/500 ms p50/p99 (Standard); 105/170 ms (S3EOZ) | [WarpStream docs](https://docs.warpstream.com/warpstream/kafka/advanced-agent-deployment-options/low-latency-clusters); [WarpStream blog](https://www.warpstream.com/blog/the-art-of-being-lazy-log-lower-latency-and-higher-availability-with-delayed-sequencing) |
| WarpStream Lightning | Object-storage write only | 33/50 ms | [WarpStream blog](https://www.warpstream.com/blog/the-art-of-being-lazy-log-lower-latency-and-higher-availability-with-delayed-sequencing) |

- Other reference points:
  - An object-storage round trip is about 100 ms (turbopuffer) — [turbopuffer architecture](https://turbopuffer.com/docs/architecture)
  - A GCS file replace "can take up to 200ms", with about 1 write/s per object on GCS — [turbopuffer queue blog](https://turbopuffer.com/blog/object-storage-queue)
  - R2 latency is "tens or hundreds of milliseconds" — [CF-DO-SQLite](https://blog.cloudflare.com/sqlite-in-durable-objects/)
  - A SlateDB manifest round trip (two-phase S3+DynamoDB) is estimated at "250-500ms" — [SlateDB RFC](https://github.com/slatedb/slatedb/blob/main/rfcs/0001-manifest.md)

**Linearization point: sequencer service vs. pure S3 CAS**
- Pure object-storage CAS:
  - Delta's next-version file (put-if-absent) — [Delta PROTOCOL.md](https://github.com/delta-io/delta/blob/master/PROTOCOL.md); [Delta storage docs](https://docs.delta.io/latest/delta-storage.html)
  - turbopuffer, where "All concurrency control is delegated to object storage" — [turbopuffer guarantees](https://turbopuffer.com/docs/guarantees)
  - SlateDB's CAS'd manifests and WAL SSTs — [SlateDB RFC](https://github.com/slatedb/slatedb/blob/main/rfcs/0001-manifest.md)
- External strongly consistent store:
  - Iceberg's metastore check-and-put — [Iceberg spec](https://iceberg.apache.org/spec/)
  - Delta's catalog ratification — [Delta PROTOCOL.md](https://github.com/delta-io/delta/blob/master/PROTOCOL.md)
  - WarpStream's control plane assigning offsets — [WarpStream blog](https://www.warpstream.com/blog/the-art-of-being-lazy-log-lower-latency-and-higher-availability-with-delayed-sequencing)
  - Pre-2024 DynamoDB LogStores — [delta.io 2022](https://delta.io/blog/2022-05-18-multi-cluster-writes-to-delta-lake-storage-in-s3/)
- At low latency the sequencer hop dominates: about 50 ms of WarpStream's roughly 100 ms S3EOZ path. — [WarpStream blog](https://www.warpstream.com/blog/the-art-of-being-lazy-log-lower-latency-and-higher-availability-with-delayed-sequencing)
- S3 CAS semantics: the first conditional write to finish wins, and others get 412. 409 can occur under concurrency, and MPU completion races need a full restart. — [AWS-CW](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html)

**Single-writer, leases, fencing**
- Cloudflare: a new DO instance can start only after 3 of 5 followers stop confirming the old one. — [CF-DO-SQLite](https://blog.cloudflare.com/sqlite-in-durable-objects/)
- SlateDB: `writer_epoch` bump, then CAS-write of the next WAL slot fences zombies. — [SlateDB RFC](https://github.com/slatedb/slatedb/blob/main/rfcs/0001-manifest.md)
- turbopuffer queue: the broker address lives in the CAS'd file, and a stale broker discovers it was replaced when its CAS fails. — [turbopuffer queue blog](https://turbopuffer.com/blog/object-storage-queue)
- S2: a fencing token for pessimistic single-writer control, and an expected sequence number for optimistic writers. — [S2 intro](https://s2.dev/blog/intro)
- D1: one primary DO per database is the only writer, and replicas forward writes to it. — [CF-D1-RR](https://blog.cloudflare.com/d1-read-replication-beta/)

**Read-your-writes tokens**
- D1 bookmarks are Lamport timestamps returned per session (for example in an `x-d1-bookmark` header). A replica waits until it reaches the bookmark, which gives sequential consistency. — [CF-D1-RR](https://blog.cloudflare.com/d1-read-replication-beta/)
- turbopuffer's strong reads check the WAL head (about 10 ms warm), and eventual reads skip this. — [turbopuffer architecture](https://turbopuffer.com/docs/architecture)
- S2's check-tail is strongly consistent and takes single-digit ms. — [S2 docs](https://s2.dev/docs/concepts/durability-consistency)
- WarpStream Lightning does not return offsets in the ack, so clients get no position token. — [WarpStream blog](https://www.warpstream.com/blog/the-art-of-being-lazy-log-lower-latency-and-higher-availability-with-delayed-sequencing)

**Rebase/retry under contention**
- Iceberg reapplies the commit to the new base when validation allows (appends always). Manifests are reused and only the manifest list is rewritten. Defaults are 4 retries with 100 ms to 60 s backoff, plus a status check for unknown outcomes. — [Iceberg spec](https://iceberg.apache.org/spec/); [Iceberg config](https://iceberg.apache.org/docs/latest/configuration/)
- Delta fails with typed concurrent-modification exceptions when file sets overlap, and blind INSERTs never conflict. — [Delta concurrency control](https://docs.delta.io/latest/concurrency-control.html)
- SlateDB retries the entire manifest update on CAS failure. — [SlateDB RFC](https://github.com/slatedb/slatedb/blob/main/rfcs/0001-manifest.md)

### Inferences (design guidance for an S3-authoritative embedded DB, "bumbledb")
- **The layered recipe that recurs**:
  1. Single writer per database, holding an epoch or lease that S3 CAS fences.
  2. Transactions execute synchronously against local state, Cloudflare-style.
  3. Client acks and side effects are held behind an "output gate" until the group commit containing them is durable.
  4. Group commit coalesces all transactions that arrive while the previous durable write is in flight. This is turbopuffer-queue style: no fixed timer, just "flush when the previous write returns", optionally with a small max window.
  5. Each group commit is one object `wal/<epoch>/<seq>` created with `If-None-Match: *`. That makes it both the commit point and the zombie fence.
  6. A manifest or checkpoint is published asynchronously and occasionally, either by If-Match CAS or as a numbered put-if-absent.
  7. Snapshots are taken when the log exceeds DB size, which bounds recovery to about 2× DB size as at Cloudflare.
- **Expected latency envelope**:
  - About 100–200 ms per commit on S3 Standard (turbopuffer 165 ms p50; GCS up to 200 ms). Pipelining keeps throughput high.
  - About 20–40 ms with a 3-bucket S3 Express One Zone quorum write (WarpStream about 20 ms; S2 Express 40 ms), with S3 Standard kept as the authority via async compaction.
  - Sub-10 ms needs a self-run replicated follower tier (Cloudflare, Neon).
- **Why a quorum write needs a single writer**: plain quorum writes to 3 zonal buckets give no CAS across buckets. Ordering therefore has to come from the single-writer epoch. Takeover must fence on a majority of buckets (claim the next slot via If-None-Match in at least 2 of 3) before a new writer accepts commits. This mirrors Cloudflare's 3-of-5 rule.
- **Unknown outcomes**: put a txn/batch UUID inside every commit object. On timeout, 412, or 409, re-read the slot to learn whether you won before retrying. Iceberg's "unknown commit state" handling and the 412-after-lost-response pitfall both argue for this.
- **Read-your-writes across stateless readers**: return a commit sequence number (a D1-style bookmark) with each ack. Readers either serve from a cache already at or past that sequence, or fetch the WAL head (one small GET/LIST, about 10 ms warm per turbopuffer). This avoids a mandatory head check on every read.
- **Cost knob**: PUT cost scales inversely with the group-commit window. A window of about 25–50 ms is where vendors operate for low latency, and an idle-time adaptive window (flush immediately when idle, coalesce under load) keeps cost proportional to load.

### Gaps
- No source gave S3 Standard PUT or conditional-PUT latency distributions directly from AWS. The ~100–200 ms figures come from vendors (turbopuffer, WarpStream) and differ by object size and region.
- I found no primary data on S3 conditional-write throughput limits per key (GCS's 1 write/s per object is documented via turbopuffer; an S3 equivalent was not found).
- The Nov 2024 AWS announcement says If-Match works in "general purpose and directory buckets", which covers S3 Express One Zone. I did not check whether S3 Express behaves the same as S3 Standard on 409/412 races, or what its conditional-write latency is. The claim that sending both headers in one request returns 400 comes only from secondary sources.
