# Object-storage primitives, consistency, latency and pricing for an S3-based commit protocol (as of 2026-10-09)

Scope: what operations exist and their exact semantics, how fast they are, and what they cost, for S3 Standard, S3 Express One Zone, and S3-compatible stores (R2, Tigris, MinIO, Garage, Ceph RGW, B2, GCS, Azure Blob, local test servers). Prices are us-east-1 list prices unless noted. "Fetched 2026-10-09" means the page was read live today. Source dates are publication or "Posted on" dates where the page gives one.

---

## 1. Amazon S3 Standard (general purpose buckets): consistency, conditional writes and deletes, and 2025–2026 additions

### Takeaway
S3 general purpose buckets give strong read-after-write consistency for PUT, DELETE, and LIST, and atomic single-key updates. They also offer real compare-and-swap primitives: `If-None-Match: *` (create-if-absent, Aug 2024) and `If-Match: <ETag>` (CAS, Nov 2024) on PutObject, CompleteMultipartUpload, and CopyObject (Oct 2025), plus `If-Match` on DeleteObject/DeleteObjects (Sept 2025). A losing writer gets `412 Precondition Failed`. A writer racing an in-flight conflicting operation gets `409 ConditionalRequestConflict` and must retry; for multipart uploads that means restarting the whole upload. Bucket policies can make the headers mandatory. Nothing in S3 does multi-key atomicity, and as of today rename and append still exist only on S3 Express One Zone directory buckets.

### Cited Findings

**Consistency model (fetched 2026-10-09)**
- "Amazon S3 provides strong read-after-write consistency for PUT and DELETE requests of objects in your Amazon S3 bucket in all AWS Regions." Reads of object metadata (HEAD), ACLs, and tags are also strongly consistent. — [S3 User Guide: data consistency model](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html#ConsistencyModel)
- Any read, GET or LIST, that starts after a successful PUT response returns the written data. AWS's own examples: a new object appears in an immediate LIST, and a deleted object disappears from an immediate LIST. — [same](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html#ConsistencyModel)
- "Updates to a single key are atomic". A concurrent GET sees old or new data, never partial data. — [same](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html#ConsistencyModel)
- Without preconditions, concurrent PUTs resolve as last-writer-wins: "If two PUT requests are simultaneously made to the same key, the request with the latest timestamp wins". Also, "There is no way to make atomic updates across keys." — [same](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html#ConsistencyModel)
- Bucket configuration is eventually consistent. AWS recommends waiting 15 minutes after first enabling versioning before writing. — [same](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html#ConsistencyModel)
- "Amazon S3 never adds partial objects; if you receive a success response, Amazon S3 added the entire object to the bucket." — [PutObject API reference](https://docs.aws.amazon.com/AmazonS3/latest/API/API_PutObject.html) (fetched 2026-10-09)

**Conditional writes: timeline**
- 2024-08-20: `If-None-Match` on PutObject and CompleteMultipartUpload, for general purpose and directory buckets, in all Regions, at no additional charge. — [AWS What's New, Aug 20 2024](https://aws.amazon.com/about-aws/whats-new/2024/08/amazon-s3-conditional-writes)
- 2024-11-25: `If-Match` (ETag) on PutObject and CompleteMultipartUpload, for general purpose and directory buckets, at no additional charge. — [AWS What's New, Nov 25 2024](https://aws.amazon.com/about-aws/whats-new/2024/11/amazon-s3-functionality-conditional-writes)
- 2024-11-25: bucket-policy enforcement of conditional writes for general purpose buckets, via the `s3:if-none-match` and `s3:if-match` condition keys. — [AWS China What's New, Nov 25 2024](https://www.amazonaws.cn/en/new/2024/amazon-s3-now-supports-enforcement-of-conditional-write-operations-for-s3-general-purpose-buckets/); [Enforce conditional writes (User Guide)](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes-enforce.html)
- 2025-09: conditional deletes (`If-Match`) on DeleteObject and DeleteObjects in general purpose buckets, in all Regions, at no extra cost. — [AWS What's New, Sept 2025](https://aws.amazon.com/about-aws/whats-new/2025/09/amazon-s3-conditional-deletes-s3-general-purpose-buckets)
- 2025-10-29: conditional CopyObject on the destination key, `If-None-Match` or `If-Match` with an ETag, in general purpose and directory buckets, at no additional charge. Enforceable with the `s3:if-match` and `s3:if-none-match` policy keys. — [AWS What's New, Oct 29 2025](https://aws.amazon.com/about-aws/whats-new/2025/10/amazon-s3-conditional-write-functionality-copy-operations)

**Conditional writes: exact semantics (User Guide, fetched 2026-10-09)** — [How to prevent object overwrites with conditional writes](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html)
- Requests must be signed with SigV4: "To use conditional writes, you must use AWS Signature Version 4 to sign the request."
- `If-None-Match` accepts only `*`. It applies to PutObject, CompleteMultipartUpload, and CopyObject, and needs only `s3:PutObject`.
- `If-Match` takes an ETag string. It applies to PutObject, CompleteMultipartUpload, and CopyObject, and needs `s3:PutObject` plus `s3:GetObject`.
- If-None-Match outcomes:
  - No object with the key: `200 OK`.
  - Object exists: `412 Precondition Failed`.
  - In versioned buckets, the write succeeds if there is no current version or the current version is a delete marker.
- Racing creators: "If multiple conditional writes or copies occur for the same object name, the first write operation to finish succeeds. Amazon S3 then fails subsequent writes with a 412 Precondition Failed response."
- Concurrent delete during an If-None-Match write: this can produce `409 Conflict`. A PutObject can be retried. A CompleteMultipartUpload cannot: "the entire multipart upload must be re-initiated with CreateMultipartUpload".
- If-Match outcomes:
  - ETag matches: `200`.
  - ETag mismatch: `412`.
  - Concurrent requests: can produce `409 Conflict`.
  - A concurrent delete that wins, or a missing key or delete-marker current version: `404 Not Found`.
- Multipart interaction: "Conditional writes do not consider any in-progress multipart uploads requests since those are not yet fully written objects." If client 2 conditionally PUTs during client 1's MPU, client 1's conditional CompleteMultipartUpload then fails with `412`.
- If a delete lands during an MPU, the CompleteMultipartUpload gets `409` (If-None-Match) or `404` (If-Match), and a new MPU is needed.
- PutObject API reference wording (fetched 2026-10-09): "If a conflicting operation occurs during the upload S3 returns a `409 ConditionalRequestConflict` response."
  - On a 409 with If-Match: "you should fetch the object's ETag and retry the upload".
  - On a 409 with If-None-Match: "retry the upload".
  - Conditional headers are "not supported for S3 on Outposts".
  - [PutObject API reference](https://docs.aws.amazon.com/AmazonS3/latest/API/API_PutObject.html)
- Community report, unverified by AWS: conditional writes fail for presigned URLs not signed with SigV4, and `If-Match: *` was rejected on PutObject while `If-None-Match: *` worked. — [AWS re:Post thread](https://repost.aws/questions/QU0NMXJve9QMS-p7eQQcnBNg/s3-conditional-writes-putobject-presigned-url-with-if-match-etag-does-not-work-for-non-sigv4-requests)

**Conditional deletes (User Guide, fetched 2026-10-09)** — [How to perform conditional deletes](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-deletes.html)
- Available on DeleteObject and DeleteObjects "in S3 general purpose and directory buckets". The header is `If-Match: <ETag>` (unchanged check) or `If-Match: *` (existence check).
- Outcomes and permissions:
  - Success returns `204 No Content`.
  - ETag mismatch returns `412`.
  - With `If-Match: *`, a key whose latest version is a delete marker returns `412`.
  - The ETag form needs `s3:DeleteObject` plus `s3:GetObject`. The `*` form needs only `s3:DeleteObject`.
- Concurrency: "You can also receive a 409 Conflict error response in the case of concurrent requests if a DELETE or PUT request to an object succeeds before a conditional delete operation on that object completes."
- DeleteObjects takes a per-key `<ETag>` in the XML body and reports results per key in `<Deleted>` or `<Error>`.
- Evaluations apply only to the current version. A bucket or IAM policy can enforce the header through `s3:if-match`; requests without it get `403`. — [Enforce conditional deletes](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-delete-enforce.html)

**Other 2026 S3 changes seen (lower relevance; from a third-party API changelog plus AWS pages)**
- Mar 2026: account regional namespaces for general purpose bucket names. — [AWS What's New, Mar 2026](https://aws.amazon.com/about-aws/whats-new/2026/03/amazon-s3-account-regional-namespaces)
- Apr 2026: SSE-C disabled by default for new buckets. Jan 2026: UpdateObjectEncryption API. Jun 2026: object annotations. These come from [awsapichanges.info](https://awsapichanges.info/archive/service/s3/) and are third-party, so verify before relying on them.
- Apr 7, 2026: S3 Files (buckets accessible as file systems). — [StorageNewsletter, 2026-04-09](https://www.storagenewsletter.com/2026/04/09/aws-launches-s3-files-making-s3-buckets-accessible-as-file-systems/)
- RenameObject is still documented as Express-only: "RenameObject is only supported for objects stored in the S3 Express One Zone storage class" ([RenameObject API reference](https://docs.aws.amazon.com/AmazonS3/latest/API/API_RenameObject.html), fetched 2026-10-09).
- Append (`x-amz-write-offset-bytes`) is likewise "only supported for objects in the Amazon S3 Express One Zone storage class in directory buckets" ([PutObject API reference](https://docs.aws.amazon.com/AmazonS3/latest/API/API_PutObject.html), fetched 2026-10-09).

### Inferences
- A Delta-Lake-style commit protocol works on S3 Standard without an external lock service. Each commit writes the log entry `log/<seq>` with `If-None-Match: *`, so the first finisher wins and losers get 412. This is the cleanest primitive because the key either exists or does not; there is no ETag to track.
- A single "head pointer" object updated with `If-Match: <etag>` is a true CAS. On S3 it needs a HEAD/GET first to learn the ETag (+1 round trip) unless the client caches the ETag returned by its last successful PUT.
- Commit-path rules:
  - Treat 409 as retryable contention.
  - Treat 412 as a definitive loss, unless the request may have been a retry of the client's own earlier attempt (see section 3).
  - Treat 404 on If-Match as "pointer deleted".
- Do not commit through CompleteMultipartUpload. A 409 there forces a full MPU restart, and in-progress MPUs are invisible to preconditions. Keep commit records small enough for a single PutObject; the PutObject limit is 5 GB.
- Conditional deletes (`If-Match: <etag>`) make garbage collection of superseded manifests or leases safe against a concurrent re-write.
- The enforcement policy keys (`s3:if-none-match`, `s3:if-match`) can be used as a guard rail so that no client (e.g. an old binary) can do an unconditional overwrite of log keys.

### Gaps
- No official statement on whether requests that fail with 412 or 409 are billed as PUT requests; I found no AWS text either way.
- AWS documents 409 only as "conflicting operation … during the upload". There is no published bound on how often 409 occurs under heavy same-key contention.
- I found no evidence of RenameObject or append for general purpose buckets as of 2026-10-09.

---

## 2. S3 Express One Zone (directory buckets): latency claims, append, RenameObject, conditional writes, CreateSession, pricing, durability, limits

### Takeaway
S3 Express One Zone is a single-AZ storage class in "directory buckets". AWS claims single-digit-ms PUT and GET. It supports conditional writes and deletes and two primitives S3 Standard lacks:
- **Append**: PutObject with `x-amz-write-offset-bytes` equal to the current size. On a mismatch it returns `400 InvalidWriteOffset`, so it is effectively a CAS on object length. Limit: 10,000 parts per object.
- **RenameObject** (June 2025): atomic, with `If-None-Match`/`If-Match` on the destination, `x-amz-rename-source-if-*` on the source, and client-token idempotency.

After the 2025-04-10 price cut it costs $0.11/GB-mo, $0.00113 per 1k PUT, $0.00003 per 1k GET, plus $0.0032/GB upload and $0.0006/GB retrieval on all bytes. Directory buckets do not return lexicographically ordered LISTs, have no versioning, use random (non-MD5) ETags, and require CreateSession tokens that expire every 5 minutes.

### Cited Findings

**Positioning and latency claims**
- AWS: "purpose-built to deliver consistent, single-digit millisecond data access", "up to 10x faster" than S3 Standard, and recommended "if your application is performance sensitive and benefits from single-digit millisecond PUT and GET latencies." — [S3 User Guide: What is S3](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html) (fetched 2026-10-09)
- Data is "redundantly stored on multiple devices within a single Availability Zone". — [same](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html)
- Designed for 99.95% availability within a single AZ and backed by the S3 SLA. — [What is S3 Express One Zone (AWS China mirror)](https://docs.amazonaws.cn/en_us/AmazonS3/latest/dev/s3-express-one-zone.html)
- Request-rate claims: up to 2,000,000 GET TPS and 200,000 PUT TPS per directory bucket. — [AWS News Blog, price-reduction post, April 2025](https://aws.amazon.com/blogs/aws/up-to-85-price-reductions-for-amazon-s3-express-one-zone/)
- Directory buckets have "no prefix limits and individual directories can scale horizontally". The default quota is 100 directory buckets per account. — [S3 User Guide](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html)

**Append (launched Nov 2024; doc fetched 2026-10-09)** — [Appending data to objects in directory buckets](https://docs.aws.amazon.com/AmazonS3/latest/userguide/directory-buckets-objects-append.html)
- Mechanism: PutObject with `x-amz-write-offset-bytes` set to the current object size.
- Limits:
  - No minimum append size; at most 5 GB per append.
  - "each object can have up to 10,000 parts. This means you can append data to an object up to 10,000 times". Parts from an MPU count toward the 10,000. Exceeding it returns `TooManyParts`; "You can use the CopyObject API to reset the count."
- Each successful append is billed as a PutObject request.
- Requires CreateSession credentials. Not supported in Local Zones.
- API reference semantics:
  - "The offset must be equal to the size of the existing object being appended to. If no object exists, setting this header to 0 will create a new object."
  - Error: `InvalidWriteOffset`, "The write offset value that you specified does not match the current object size", HTTP 400.
  - [PutObject API reference](https://docs.aws.amazon.com/AmazonS3/latest/API/API_PutObject.html)
- Checksums: with CRC algorithms, HeadObject or GetObject returns a full-object CRC. With SHA-1/SHA-256, the per-append checksums come back in the PutObject responses. — [append doc](https://docs.aws.amazon.com/AmazonS3/latest/userguide/directory-buckets-objects-append.html)

**RenameObject (announced 2025-06-18; API reference fetched 2026-10-09)**
- Announcement: atomic rename "without any data movement" within one directory bucket, in all Express One Zone Regions. — [AWS What's New, June 2025](https://aws.amazon.com/about-aws/whats-new/2025/06/amazon-s3-express-one-zone-atomic-renaming-objects-api)
- Request: `PUT /{Key}?renameObject` with `x-amz-rename-source`.
- Destination conditions:
  - `If-None-Match: *` returns 412 if the destination exists.
  - `If-Match: <etag>` returns 412 on mismatch.
  - `If-Modified-Since` and `If-Unmodified-Since` are also accepted.
- Source conditions: `x-amz-rename-source-if-match`, `-if-none-match`, `-if-modified-since`, and `-if-unmodified-since`.
- Idempotency: `x-amz-client-token` (up to 64 ASCII chars). Retrying a completed request with the same token and parameters "succeeds without performing any further actions". The same token with different parameters returns `IdempotentParameterMismatch` (400).
- Express One Zone only; needs a ReadWrite session.
- [RenameObject API reference](https://docs.aws.amazon.com/AmazonS3/latest/API/API_RenameObject.html)
- Pricing: RenameObject is "priced the same as PUT, COPY, POST, LIST requests". — [S3 pricing page](https://aws.amazon.com/s3/pricing/) (fetched 2026-10-09)

**Conditional writes and deletes on directory buckets**
- If-None-Match (Aug 2024) and If-Match (Nov 2024) cover directory buckets. — [Aug 2024](https://aws.amazon.com/about-aws/whats-new/2024/08/amazon-s3-conditional-writes); [Nov 2024](https://aws.amazon.com/about-aws/whats-new/2024/11/amazon-s3-functionality-conditional-writes)
- Conditional CopyObject (Oct 2025) also covers directory buckets. — [Oct 2025](https://aws.amazon.com/about-aws/whats-new/2025/10/amazon-s3-conditional-write-functionality-copy-operations)
- Conditional deletes cover directory buckets. — [conditional deletes doc](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-deletes.html)
- Policy enforcement of conditional deletes is documented "at a general purpose bucket level". — [same](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-deletes.html)

**Directory-bucket differences that matter for a commit protocol (fetched 2026-10-09)** — [Differences for directory buckets](https://docs.aws.amazon.com/AmazonS3/latest/userguide/s3-express-differences.html)
- "For directory buckets, ListObjectsV2 does not return objects in lexicographical (alphabetical) order." Also, "prefixes must end in a delimiter and only '/' can be specified as the delimiter." LIST results also include prefixes that exist only because of in-progress MPUs.
- ETags "are random alphanumeric strings unique to the object and not MD5 checksums."
- Not supported: S3 Versioning, Object Lock, Replication, Event Notifications, MD5 checksums, SSE-C, object tags, and more.
- Deleting the last object in a "directory" recursively deletes the empty directories.
- MPU part numbers must be consecutive.

**CreateSession (session auth)**
- Zonal endpoint operations (PutObject, GetObject, RenameObject, …) use credentials from `CreateSession`. Each session is scoped to one bucket and expires after 5 minutes; it cannot be extended, only re-created. The token goes in `x-amz-s3session-token`.
- SDKs refresh tokens automatically. CopyObject does not use session credentials.
- [CreateSession API reference](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CreateSession); [RenameObject reference (session description)](https://docs.aws.amazon.com/AmazonS3/latest/API/API_RenameObject.html)

**Pricing change effective 2025-04-10 (us-east-1)** — [AWS News Blog](https://aws.amazon.com/blogs/aws/up-to-85-price-reductions-for-amazon-s3-express-one-zone/); [AWS What's New, Apr 2025](https://aws.amazon.com/about-aws/whats-new/2025/04/amazon-s3-express-one-zone-reduces-storage-request-prices/)

| Item | Before | After 2025-04-10 |
|---|---|---|
| Storage | $0.16/GB-mo | $0.11/GB-mo |
| PUT | $0.0025 /1k (≤512 KB) | $0.00113 /1k |
| GET | $0.0002 /1k (≤512 KB) | $0.00003 /1k |
| Upload | $0.008/GB (bytes >512 KB only) | $0.0032/GB (all bytes) |
| Retrieval | $0.0015/GB (bytes >512 KB only) | $0.0006/GB (all bytes) |

- Discrepancy: an AWS China blog copy lists $0.10 for the new storage price; the AWS global blog says $0.11. — [AWS China blog](https://www.amazonaws.cn/blog-selection/up-to-85-price-reductions-for-amazon-s3-express-one-zone/)
- Outdated-source flag: Bodner et al. (data from 2024) list Express at 20¢/M reads and 250¢/M writes. Those are pre-cut prices. — [arXiv 2501.07771](https://arxiv.org/html/2501.07771)

**Durability and availability caveats from practitioners**
- Nixiesearch (2025-12-01) notes S3 Express "is less reliable" and suggests publishing to multiple Express buckets to survive AZ outages. — [Nixiesearch substack](https://nixiesearch.substack.com/p/benchmarking-read-latency-of-aws)
- WarpStream's low-latency mode writes to S3 Express with a replication factor of 2 (its cost model multiplies PUTs by 2 for replication). — [WarpStream blog, 2025-04-18](https://www.warpstream.com/blog/warpstream-s3-express-one-zone-benchmark-and-total-cost-of-ownership)

### Inferences
- Append with `x-amz-write-offset-bytes` is a native conditional append: two concurrent appenders at the same offset cannot both succeed, and the loser gets `InvalidWriteOffset`. The docs state the offset must equal the current size but do not describe the concurrent-append race explicitly, so verify this empirically.
- The 10,000-part cap means a WAL-in-one-object design must roll to a new object at most every 10,000 commits, or CopyObject to compact.
- RenameObject + `If-None-Match: *` + client token gives an idempotent "publish staged object as log entry N" operation. It is the only S3 primitive with built-in retry idempotency, which makes hedging and retries of commits safe on Express.
- Commit designs that find the latest log entry via `ListObjectsV2` with `start-after` break on directory buckets because LIST order is not lexicographic. Use a pointer object, or probe sequence numbers with HEAD/GET.
- Single-AZ means the AZ is a failure domain for the authoritative state. An "authority lives in the bucket" database on Express needs either cross-AZ duplication (2 directory buckets, so 2x PUT cost) or acceptance of AZ-outage unavailability and possible loss.
- For small commit records, the post-2025 per-byte charge is negligible: 4 KB × 1M = ~4 GB, or about $0.013 upload.

### Gaps
- No numeric durability figure for Express One Zone in the sources I fetched (only "redundantly stored on multiple devices within a single AZ").
- Whether CreateSession calls are billed was not found.
- No AWS statement on append behaviour under concurrent appenders (beyond the offset rule).
- No independent measurement of Express PUT or append latency percentiles was found (see section 3).

---

## 3. Measured latency (S3 Standard, S3 Express, R2, Tigris, GCS, Azure), tail latency, and hedging

### Takeaway
Same-region small-object S3 Standard latency:
- GET p50 is about 22–27 ms, with p95 about 39–75 ms and p99 about 40–86 ms.
- PUT p50 is about 26–40 ms for 1 KB and about 70 ms for 500 KiB, with p99 about 137 ms (n=100).
- Rare outliers reach about 10 s (1M-sample study).

S3 Express GET is about 5–6.5 ms at both p50 and p95. No independent Express PUT percentile was found. Tigris (vendor benchmark) shows 1 KB GET p50 5.4 ms and PUT p50 12.9–16.8 ms. R2 numbers in that benchmark are much worse (PUT p50 197 ms), but the setup is questionable. No credible recent percentile data exists for GCS or Azure small objects. AWS explicitly recommends aggressive retries (hedging) for latency-sensitive requests.

### Cited Findings

**S3 Standard**
- **Bodner et al. (arXiv 2501.07771)**
  - Setup: us-east-1a, 1 KiB objects, 1,000,000 requests per service, 10 clients, experiments Feb–Oct 2024.
  - S3 Standard reads: median 27 ms, p95 75 ms, max "just over 10 s" (about 374x the median).
  - S3 Standard writes: median 40 ms.
  - S3 had "both the highest median … and tail latencies" of the services tested. p90/p99 exist only in a figure.
  - [arXiv HTML](https://arxiv.org/html/2501.07771)
- **TopicPartition (2025-03-04)**
  - Setup: EC2 eu-north-1 to a same-region bucket, 500 KiB objects, 100 PUT+GET pairs.
  - PUT: p50 69.75, p95 101.10, p99 137.23, max 142.31 ms.
  - GET: p50 26.13, p95 38.86, p99 86.13, max 86.66 ms.
  - The page's TL;DR swaps the PUT and GET medians; the detailed table is authoritative. With n=100, p99 is effectively the max.
  - [TopicPartition](https://topicpartition.io/misc/AWS-S3-PUT-latency-benchmark)
- **Tigris vendor benchmark (2025-07-08)**
  - Setup: client on OCI us-sanjose-1, S3 bucket in us-west-1, so not same-cloud.
  - 1 KB PUT (10M load): p50 25.7 ms, p90 37.8 ms.
  - Run phase: read p50 22.4 / p90 42.0 ms; write p50 27.0 / p90 41.2 ms. No p99 reported.
  - [Tigris blog](https://www.tigrisdata.com/blog/benchmark-small-objects/)
- **AWS Lambda test (ap-northeast-1, 64 KiB GET)**
  - Same key: p50 24.8, p90 27.7, p99 40.0 ms.
  - Different key per iteration: p50 about 54 ms, p99 about 105 ms.
  - [AWS Builder Center](https://builder.aws.com/content/2zfBu6Br5Z46ZaFsNrGfPtswmsX/testing-read-and-write-performance-with-s3-files-from-aws-lambda) (date not shown; via search excerpt)
- **Nixiesearch (2025-12-01)**
  - Random 4 KB GETs on m5id.large, same AZ: S3 Standard shows "enormous 100+ ms p99 tail latency".
  - Larger instances do not reduce per-request latency, only raise throughput.
  - [Nixiesearch](https://nixiesearch.substack.com/p/benchmarking-read-latency-of-aws)
- **Durner, Leis, Neumann (VLDB 2023), via GreptimeDB's summary**
  - Base latency about 30 ms median for 1 KiB, plus about 20 ms/MiB.
  - Fewer than 5% of 16 MiB requests had first-byte latency over 200 ms.
  - Fewer than 5% of 16 MiB requests were unfinished after 600 ms.
  - Optimal request size is 8–16 MiB, and 200–250 concurrent requests are needed for about 100 Gbps.
  - GreptimeDB's own measurements: manifest reads under 1 KiB at about 30 ms p50 and about 60 ms p99 (cold).
  - [GreptimeDB blog, 2024-01-31](https://greptime.com/blogs/2024-01-31-paper-sharing); paper: [PVLDB 16(11)](https://vldb.org/pvldb/vol16/p2769-durner.pdf)
- **Older AWS CloudWatch percentile example (about 2019; flagged as old)**: 50% within about 25 ms, 90% within about 75 ms, 99% within about 140 ms. — [AWS Storage Blog](https://aws.amazon.com/blogs/storage/amazon-s3-cloudwatch-percentiles/)
- **BtrLog (VLDB'26, arXiv 2026-06-25)**: object storage writes "take tens of milliseconds" and putting a WAL directly on object storage, "even its lower-latency variant, S3 Express", is "impractical due to high latency and high per-append cost". For comparison, EBS io2 median is about 318 µs and BtrLog's quorum log is 70 µs median / 79 µs p99. — [arXiv 2606.27051](https://arxiv.org/html/2606.27051)

**S3 Express One Zone**
- Bodner et al.: 1 KiB reads have "Median and 95th percentile around 5 ms". Write numbers are only in a figure. — [arXiv 2501.07771](https://arxiv.org/html/2501.07771)
- AWS re:Post article: a single connection got GET about 5.6 ms for 64 KB and about 6.5 ms for 512 KB. Its p99 of about 255 ms was for parallel 32 MB GETs and is transfer-dominated, so not comparable. — [re:Post](https://repost.aws/articles/ARwllpT3g9QH2zqh5Z5L2LgQ/s3-express-one-zone-throughput-capped-at-nat-gateway-bandwidth-use-a-vpc-gateway-endpoint)
- Nixiesearch: Express is "much, much faster" than Standard but 5–10x slower than EBS for 4 KB random reads. Latency improves as request volume rises (warm-up). — [Nixiesearch](https://nixiesearch.substack.com/p/benchmarking-read-latency-of-aws)
- WarpStream end-to-end Kafka produce on Express: p50 105 ms and p99 169 ms, "roughly 3x lower" than on S3 Standard. These are not raw PUT numbers, since batching is included. — [WarpStream, 2025-04-18](https://www.warpstream.com/blog/warpstream-s3-express-one-zone-benchmark-and-total-cost-of-ownership)

**Tigris and R2 (vendor benchmark; weigh accordingly)** — [Tigris blog, 2025-07-08](https://www.tigrisdata.com/blog/benchmark-small-objects/)
- Tigris, 1 KB:
  - PUT p50 16.8 / p90 35.9 ms (load phase).
  - GET p50 5.4 / p90 7.9 ms.
  - Update p50 12.9 / p90 16.5 ms.
- R2 (WNAM):
  - PUT p50 197.1 / p90 340.2 ms.
  - Read and write rows are both shown as p50 605.7 / p90 681.0 ms. The identical rows and inconsistent throughput figures suggest copy errors.
- Caveats: one run, buckets in different regions, client on a third cloud.

**R2 latency features**: Local Uploads (open beta 2026-02-03) writes object data near the client first and copies it asynchronously to the bucket's region. Cloudflare reports "up to 75% reduction in Time to Last Byte" for uploads. No extra cost; standard Class A pricing applies. Not available for jurisdiction-restricted buckets. — [Cloudflare blog: R2 Local Uploads](https://blog.cloudflare.com/r2-local-uploads); [InfoQ, Feb 2026](https://www.infoq.com/news/2026/02/cloudflare-local-uploads)

**GCS and Azure**: the only multi-provider time-to-first-byte comparison found is from 2015 (PerfKitBenchmarker, 1,000 runs: Azure ≈ S3, GCS more than 3x higher average), and it is outdated. 2025–2026 comparison blogs give contradictory ranges (50–200 ms, "~10 ms Azure") without methodology. — [Bjornson 2015](http://blog.zachbjornson.com/2015/12/29/cloud-storage-performance.html); [CloudExpat 2025](https://www.cloudexpat.com/blog/enterprise-cloud-storage-deep-dive-p2/)

**Hedging and retry guidance**
- AWS: "For latency-sensitive applications, Amazon S3 advises tracking and aggressively retrying slower operations. When you retry a request, we recommend using a new connection to Amazon S3 and performing a fresh DNS lookup." — [S3 performance design patterns](https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance-design-patterns.html) (fetched 2026-10-09)
- AWS: for requests under 512 KB, "median latencies are often in the tens of milliseconds range"; a good guideline is to "retry a GET or PUT operation after 2 seconds", then back off (a second retry after 4 more seconds). — [same](https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance-design-patterns.html)
- AWS: for fixed-size requests, "identify the slowest 1 percent of requests and to retry them. Even a single retry is frequently effective at reducing latency." For large requests (>128 MB), retry the slowest 5%. — [same](https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance-design-patterns.html)
- AWS also recommends reusing pooled HTTP connections to avoid TCP and TLS setup, and spreading requests over many S3 IPs (no DNS pinning). — [same](https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance-design-patterns.html)
- Bodner et al. used a 200 ms request timeout with exponential backoff and found that straggling clients under 503 throttling come from retry backoff. The query engine re-triggers stragglers after a size-based timeout. — [arXiv 2501.07771](https://arxiv.org/html/2501.07771)
- Alluxio notes that for objects under 100 KiB, connection handshake dominates and "can halve throughput". — [Alluxio S3 API benchmarks](https://documentation.alluxio.io/ee-ai-en/benchmark/s3-api)

### Inferences
- Suggested latency budgets for a same-region client, warm connections, and small commit records (1–64 KB):

| Operation | p50 | p99 | Basis |
|---|---|---|---|
| S3 Standard conditional PUT | about 30–40 ms | about 100–150 ms | 500 KiB data in TopicPartition; extrapolated to small records |
| S3 Standard GET/HEAD of pointer | about 25 ms | about 40–90 ms | |
| S3 Standard tail beyond p99.9 | seconds | seconds | Max about 10 s seen in 1M requests; budget only with hedging |
| S3 Express GET | about 5 ms | low tens of ms | Inferred; no published p99 for small objects |
| S3 Express PUT, append, rename | single-digit ms | — | AWS claim only, unverified |

- A commit = (GET pointer) + (conditional PUT) on S3 Standard therefore costs about 60–70 ms p50 and about 200+ ms p99 before hedging. On Express it is roughly 10–15 ms p50, by inference.
- Hedging conditional writes is not free of semantics. If a hedged duplicate `If-None-Match` PUT races the original, one returns 200 and the other 412. The client that sees 412 must not conclude it lost: it should GET the key and compare content or a writer-unique token embedded in the object or user metadata. On Express, RenameObject's `x-amz-client-token` provides real idempotency for retries.
- GETs and HEADs can be hedged freely. A hedge delay near the observed p95–p99 (about 50–100 ms on S3 Standard) is consistent with AWS's "retry the slowest 1%" advice. The 2-second guideline is AWS's conservative default, not a latency-optimal value.
- R2's latency depends heavily on client location relative to the bucket's home region. The Tigris numbers (client in San Jose, bucket in WNAM) should not be taken as R2's same-region latency.

### Gaps
- No independent, published p99/p99.9 for small conditional PUTs on S3 Standard or Express, and none at all for append or RenameObject.
- No credible 2024–2026 same-region small-object percentile data for GCS, Azure Blob, or R2 (a Workers-colocated client). The Durner paper's PDF could not be parsed (binary). Only a secondary summary was used.
- No quantitative study of hedging's effect on S3 tail latency (for example, p99.9 with and without hedge) was found.

---

## 4. S3-compatible stores and other clouds: conditional-write support in 2026; local test lanes

### Takeaway
Create-if-absent and ETag CAS are supported on R2 (PutObject only; not on CompleteMultipartUpload), Tigris (strong only for single- or multi-region buckets), MinIO (but the project is archived), Ceph RGW (Red Hat 8.1+, with bugs fixed in 2025), GCS (generation preconditions), and Azure Blob (ETag and `*`).

They are not supported on Garage ("structurally impossible") or, per a July 2025 report, Backblaze B2.

For local test lanes:
- MinIO's last community release is 2025-10-15 and the repo was archived April 2026.
- LocalStack's open-source repo was archived 2026-03-23, and the unified image needs an auth token.
- s3s-fs implements If-None-Match and If-Match but checks preconditions non-atomically, so it is fine for functional tests and wrong for race tests.

### Cited Findings

**Cloudflare R2**
- S3 API compatibility table (fetched 2026-10-09):
  - PutObject supports "If-Match, If-Modified-Since, If-None-Match, If-Unmodified-Since". GetObject and HeadObject support the same set.
  - CompleteMultipartUpload lists no conditional headers. DeleteObject lists none.
  - CopyObject lists only the `x-amz-copy-source-if-*` headers; UploadPartCopy marks them unsupported.
  - [R2 S3 API compatibility](https://developers.cloudflare.com/r2/api/s3/api/)
- R2 extensions (fetched 2026-10-09): destination-side CopyObject preconditions (beta), `cf-copy-destination-if-match`, `-if-none-match`, `-if-modified-since`, and `-if-unmodified-since`.
  - They "work akin to the similarly named conditional headers supported on PutObject", and failure returns `412 PreconditionFailed`.
  - Source and destination checks "are not atomic in relation to" each other.
  - [R2 S3 extensions](https://developers.cloudflare.com/r2/api/s3/extensions/)
- Consistency (fetched 2026-10-09):
  - Read-after-write is global ("readers will immediately see the latest object globally"), and LIST is point-in-time.
  - Concurrent unconditional writes: "the last writer to complete 'wins'".
  - IAM is eventually consistent (up to about 1 minute). Cached custom domains relax consistency; the S3 API and Workers bindings are unaffected.
  - [R2 consistency model](https://developers.cloudflare.com/r2/reference/consistency/)
- The Rust `object_store` crate documents R2 and MinIO as supporting conditional put via standard If-Match and If-None-Match. — [object_store S3ConditionalPut docs](https://docs.tvix.dev/rust/object_store/aws/enum.S3ConditionalPut.html)

**Tigris** (fetched 2026-10-09) — [Tigris conditional operations](https://www.tigrisdata.com/docs/objects/conditionals)
- Supports If-Match, If-None-Match (including `"*"` for create-only), If-Modified-Since, and If-Unmodified-Since. Conditions can be combined.
- Failure on PUT returns 412. CAS is documented as read ETag, then write with `If-Match`.
- Consistency depends on bucket type. Multi-region and single-region buckets "provide strong consistency globally". Global and dual-region buckets are "strong consistency within the same region and eventual consistency globally". Tigris recommends multi- or single-region buckets for conditional writes.
- Conflict: an earlier indexed version of the same page described an `X-Tigris-Consistent: true` header (route to leader). The page fetched today does not mention it, so the header may have been retired. — [search excerpt of the same URL](https://www.tigrisdata.com/docs/objects/conditionals)

**MinIO**
- MinIO's blog says MinIO honours both If-None-Match and If-Match on PUT. It also states AWS supports only If-None-Match, which is outdated since Nov 2024. — [MinIO blog](https://www.min.io/blog/leading-the-way-minios-conditional-write-feature-for-modern-data-workloads)
- Reported correctness bug: a conditional write may be accepted when the server cannot reach read quorum to evaluate the precondition. This comes from a secondary task description dated 2026-03-19, not MinIO's tracker. — [Harbor task minio-21653](https://hub.harborframework.com/tasks/abundant/minio__minio-21653)
- Project status timeline:
  - The community edition went to maintenance mode in December 2025, after earlier removal of console admin features. — [InfoQ, Dec 2025](https://infoq.com/news/2025/12/minio-s3-api-alternatives/)
  - Prebuilt binaries and Docker images stopped in October 2025. — [ayedo](https://ayedo.de/en/posts/minio-im-maintenance-mode/); [vonng](https://blog.vonng.com/en/db/minio-resurrect/)
  - The last release, `RELEASE.2025-10-15T17-29-55Z`, includes the CVE-2025-62506 fix. — [vonng](https://blog.vonng.com/en/db/minio-resurrect/)
  - "No longer maintained" status came on 2026-02-12; the repo was archived 2026-04-25. — [Pinggy blog](https://pinggy.io/blog/minio_archived_self_hosted_s3_alternatives/)
  - GitHub API, queried 2026-10-09: `minio/minio` has `archived: true`, last push 2026-04-24, and its latest release is `RELEASE.2025-10-15T17-29-55Z` (published 2025-10-16). — [github.com/minio/minio](https://github.com/minio/minio)
  - MinIO steers users to AIStor ("AIStor Free" requires a license key). Community forks exist. — [vonng](https://blog.vonng.com/en/db/minio-resurrect/); [LinuxToday on the fork](https://www.linuxtoday.com/blog/open-source-community-launches-minio-fork/)

**Garage**
- Official known-issues doc: "No conditional writes / locking / WORM support (`if-none-match`, ...)". It says "This is structurally impossible to implement in Garage due to the lack of a consensus algorithm", and "many practical use-cases for `if-none-match` cannot be supported (e.g. using it to implement mutual exclusion between concurrent writers)". It tracks the feature as issue #1052.
- Latest tag v2.4.1; the repo was active as of 2026-10-08.
- [Garage known-issues.md (GitHub mirror)](https://github.com/deuxfleurs-org/garage/blob/main/doc/book/reference-manual/known-issues.md)

**Ceph RGW**
- Red Hat Ceph Storage 8.1 errata added "support for conditional PUT and DELETE operations, including bulk and multi-delete requests"; conditional InitMultipartUpload is not implemented. — [RHCS 8.1 release notes](https://docs.redhat.com/en/documentation/red_hat_ceph_storage/8/html/8.1_release_notes/asynchronous-errata-updates)
- Bug: If-Match was ignored on conditional PUTs into versioned buckets. It was fixed in ceph-20.1.0-26 (RHCS 9.0) and backported to 19.2.1-258 (8.1z3). — [RH bug 2380738](https://bugzilla.redhat.com/show_bug.cgi?id=2380738); [RH bug 2383254](https://bugzilla.redhat.com/show_bug.cgi?id=2383254)
- An unverified report says RGW rejects quoted ETags in If-Match. — [GitHub issue](https://github.com/block/buzz/issues/3002)
- I found no upstream release note naming the version (Squid 19.x / Tentacle 20.x).

**Backblaze B2**: a July 2025 Proxmox Backup Server patch skips `If-None-Match` for B2 because B2 "fails with an error" when the header is present. I found no Backblaze documentation of support. — [Proxmox pbs-devel, July 2025](https://lists.proxmox.com/pipermail/pbs-devel/2025-July/014234.html)

**Other S3-compatibles with documented support**
- CoreWeave: the loser of concurrent If-None-Match requests gets `409 ConditionalRequestConflict`. — [CoreWeave docs](https://docs.coreweave.com/products/storage/object-storage/buckets/conditional-requests)
- OVHcloud: requests with both If-Match and If-None-Match get 400. — [OVHcloud guide](https://docs.ovhcloud.com/en/guides/storage-and-backup/object-storage/s3-conditional-writes)
- Scaleway. — [Scaleway docs](https://www.scaleway.com/en/docs/object-storage/api-cli/using-conditional-writes/)
- OpenStack Swift s3api: only `If-None-Match: *` on PUT. — [opendev commit](https://opendev.org/openstack/swift/commit/edd5eb29d7d6041ae0b16b78cbcb89f9dd95309f)
- Apache Ozone: not yet supported, per a draft design of Nov 2025. — [Ozone HDDS-13117](https://ozone.apache.org/docs/edge/design/s3-conditional-requests.html)

**Google Cloud Storage** (fetched 2026-10-09) — [GCS request preconditions](https://docs.cloud.google.com/storage/docs/request-preconditions)
- `ifGenerationMatch` (JSON) / `x-goog-if-generation-match` (XML) proceeds only if the live generation matches; otherwise `412 Precondition Failed`.
- `ifGenerationMatch=0` proceeds only "if no object with the specified name exists in the bucket or if there are only noncurrent versions". This is GCS's create-if-absent.
- `ifMetagenerationMatch` exists for metadata, and "you should always use a generation precondition as well".
- "Preconditions cannot be used in XML API multipart uploads" (returns `400 NotImplemented`).
- Google recommends generation numbers over ETags for preconditions because ETags are not consistent across APIs.
- Rate limit: about 1 write per second to the same object name; exceeding it may cause 429 throttling. Bucket-wide initial capacity is about 1,000 writes/s. — [GCS quotas & limits](https://docs.cloud.google.com/storage/quotas); [GCS request rate guide](https://docs.cloud.google.com/storage/docs/request-rate)

**Azure Blob Storage** (page updated 2026-01-20; fetched 2026-10-09) — [Specifying conditional headers for Blob service operations](https://learn.microsoft.com/en-us/rest/api/storageservices/specifying-conditional-headers-for-blob-service-operations)
- Put Blob, Put Block List, Append Block, Copy Blob (destination and `x-ms-source-if-*`), Delete Blob, and Lease Blob support If-Match, If-None-Match (ETag or `*`; `*` means "only if the resource doesn't exist"), If-Modified-Since, If-Unmodified-Since, and `x-ms-if-tags`.
- Every unmet write condition returns `412 Precondition Failed`.
- Only one ETag value is allowed per header on writes, and more than one conditional header (except the two allowed pairs) returns 400.
- Put Block, List Blobs, and Create Container do not support conditional headers.

**Local test lanes**
- **s3s / s3s-fs (Rust)**: source read via the GitHub API, file last changed 2026-10-06.
  - `put_object` and `complete_multipart_upload` implement `If-None-Match: *`. A non-`*` value returns `NotImplemented`.
  - They also implement `If-Match`, both `*` and a strong ETag compare, returning `PreconditionFailed`.
  - Tests exist: `test_if_none_match_wildcard`, `test_complete_multipart_if_none_match`, and `test_copy_object_if_none_match`.
  - The check is `object_path.exists()` / ETag compare before the body is streamed, and the object is published with `fs::rename`. No lock spans check and publish.
  - [s3s-fs src/s3.rs](https://github.com/s3s-project/s3s/blob/main/crates/s3s-fs/src/s3.rs); [s3s-fs src/fs.rs](https://github.com/s3s-project/s3s/blob/main/crates/s3s-fs/src/fs.rs); [s3s-fs conditional tests](https://github.com/s3s-project/s3s/blob/main/crates/s3s-fs/tests/aws/conditional.rs)
- **LocalStack**
  - S3 conditional-write work began in Aug 2024 (PR #11402, per a LocalStack engineer). — [arrow-rs issue #6285](https://github.com/apache/arrow-rs/issues/6285)
  - v4.14 added If-Match and If-None-Match to CopyObject. — [LocalStack v4.14.0 release notes](https://newreleases.io/project/github/localstack/localstack/release/v4.14.0)
  - The open-source repo was archived on 2026-03-23, and the Community Edition is being replaced by a single image requiring registration and an auth token. — [InfoQ, Feb 2026](https://www.infoq.com/news/2026/02/localstack-aws-community); [github.com/localstack/localstack](https://github.com/localstack/localstack)
  - GitHub API, queried 2026-10-09: `archived: true`, latest release v4.14.0 (2026-02-26).
- **arrow-rs `object_store`**: made native S3 conditional writes opt-in partly for compatibility with older LocalStack versions; PR #6682 implemented `PutMode::Create` and `copy_if_not_exists` for native S3. — [arrow-rs #6285](https://github.com/apache/arrow-rs/issues/6285); [arrow-rs PR #6682](https://github.com/apache/arrow-rs/pull/6682)

### Inferences
- **Portable primitives**:
  - **Create-if-absent on a fresh key** is the most portable. S3 `If-None-Match: *`, R2, Tigris, MinIO, Ceph 8.1+, Azure `If-None-Match: *`, and GCS `ifGenerationMatch=0` all support it.
  - **ETag CAS** on an existing key is also broadly available: S3, R2, Tigris, MinIO, Ceph, Azure, and GCS by generation.
  - **Conditional DELETE** is less portable. S3 has it since Sept 2025 and Azure, GCS (generation) and Ceph 8.1 support it, but R2's table lists none.
  - **Conditional CompleteMultipartUpload** is not portable: R2 and GCS XML do not support it. Commit records should be single-PUT objects.
- **GCS's 1 write/s per object name** caps a "single mutable head pointer" design at about 1 commit/s per pointer on GCS. An append-only log of new keys (create-if-absent) has no such cap. This argues for log-of-new-keys as the canonical protocol and pointer CAS only as an optimization.
- **Error-code portability**: the losing writer may see 412 (AWS, R2, Tigris, Azure, GCS) or 409 (AWS during an in-flight conflict; CoreWeave for concurrent If-None-Match). Client code should treat both as "not committed; re-read".
- **Test lane guidance**:
  - s3s-fs is suitable for functional single-writer protocol tests. It is unsuitable for concurrency or race tests, because two concurrent `If-None-Match: *` PUTs can both pass the existence check and both return 200, with the later rename winning. This is inferred from source reading; I did not run it.
  - MinIO (pinned to the last release or a maintained fork) is the more realistic multi-writer local target. It is unmaintained upstream, though, with a reported quorum-related precondition bug.
  - LocalStack now needs an auth token in CI.
  - Garage must be excluded from any lane that relies on conditional writes.
  - A small real-S3 (and optionally Express) lane is the only authoritative conformance test for 409/412 semantics.
- Tigris global or dual-region buckets should not hold the authority object, since conditions there may be evaluated against stale state across regions.

### Gaps
- I could not confirm whether R2 enforces `If-Match`/`If-None-Match` on PutObject atomically under concurrency, or which status the loser gets (412 vs 409). R2 docs list the headers but do not describe concurrent semantics.
- No Backblaze statement for 2026 on conditional writes.
- No upstream Ceph version for conditional PUT.
- No LocalStack changelog entry confirming `If-Match` on PutObject; it is likely present but unverified.
- I did not empirically verify the s3s-fs race.

---

## 5. Pricing (requests, storage, egress) and the cost of commits, LIST, and polling

### Takeaway
Per 1M operations:

| Store | Write (PUT-class) | Read (GET-class) |
|---|---|---|
| S3 Standard | $5.00 | $0.40 |
| S3 Express (post-2025) | $1.13 | $0.03 |
| R2 | $4.50 (first 1M/month free) | $0.36 |
| Tigris single-region | $5.00 | $0.50 |
| Tigris multi-region | $10.00 | $0.50 |

LIST is billed as a write-class operation everywhere checked, so polling by LIST costs about 12x more than polling by GET or HEAD on S3 Standard and R2. DELETE is free on S3, R2 and Tigris. R2 and Tigris charge zero egress. One million single-PUT commits per month costs about $5 on S3 Standard and about $1.13 on Express, so request cost is negligible next to compute at this scale. Per-client continuous polling is the cost driver.

### Cited Findings

**List prices**
- **S3 Standard (us-east-1)**
  - Requests: PUT/COPY/POST/LIST $0.005 per 1,000; GET/SELECT $0.0004 per 1,000. — [Cloudian pricing guide](https://cloudian.com/blog/5-components-of-aws-s3-storage-pricing.md) (secondary; AWS pricing tables did not render in fetch)
  - The AWS pricing page confirms "S3 GET requests from the S3 Standard storage class cost $0.0004 per 1,000 requests", that "DELETE and CANCEL requests are free", that LIST is charged at the PUT/COPY/POST rate, and that same-Region transfer to AWS services is free. — [S3 pricing](https://aws.amazon.com/s3/pricing/) (fetched 2026-10-09)
  - Storage: about $0.023/GB-mo ("$23 per TB per month"). — [GreptimeDB summary of Durner et al.](https://greptime.com/blogs/2024-01-31-paper-sharing)
- **S3 Express One Zone (us-east-1, from 2025-04-10)**: $0.11/GB-mo; PUT $0.00113/1k; GET $0.00003/1k; upload $0.0032/GB and retrieval $0.0006/GB on all bytes. — [AWS News Blog](https://aws.amazon.com/blogs/aws/up-to-85-price-reductions-for-amazon-s3-express-one-zone/)
- **Cloudflare R2 Standard** (fetched 2026-10-09) — [R2 pricing](https://developers.cloudflare.com/r2/pricing/)
  - Storage $0.015/GB-mo; Class A $4.50/M; Class B $0.36/M; egress free.
  - Class A includes ListObjects, PutObject, CopyObject, CreateMultipartUpload, CompleteMultipartUpload, and UploadPart. Class B includes GetObject and HeadObject.
  - Free: DeleteObject, DeleteBucket, and AbortMultipartUpload.
  - Monthly free tier: 10 GB-mo storage, 1M Class A, and 10M Class B.
  - Not charged when "the caller does not have permission" (401). The page says nothing about 412 or 304 responses.
  - Infrequent Access: $0.01/GB-mo, $9.00/M Class A, $0.90/M Class B, $0.01/GB retrieval, 30-day minimum.
- **Tigris** — [Tigris pricing (search-indexed)](https://www.tigrisdata.com/pricing.md)
  - Single-region Standard: $0.02/GB-mo, Class A (PUT, COPY, POST, LIST) $0.005/1k, Class B (GET, SELECT, others) $0.0005/1k, DELETE free.
  - Multi-region: $0.025/GB-mo and Class A $0.01/1k.
  - Egress is $0 for regional, inter-region and internet transfer. Free tier: 5 GB, 10k Class A, 100k Class B. Storage is metered in GiB.
  - Direct fetch of the pricing page failed, so these figures come from the search index.
- **WarpStream's Express cost model** is a worked example: 150 PUT/s × 2 replicas ≈ 777.6M PUTs/month. The page states $1,034/month, but the arithmetic gives about $879. — [WarpStream, 2025-04-18](https://www.warpstream.com/blog/warpstream-s3-express-one-zone-benchmark-and-total-cost-of-ownership)
- Bodner et al.: keeping S3 "warm" for 100K IOPS costs $144/hour, and they conclude serverless storage prices "are generally inadequate for fine-grained operational workloads". — [arXiv 2501.07771](https://arxiv.org/html/2501.07771)
- BtrLog's batched S3 archival costs about $3×10⁻¹⁰ per 1 KB append (16 MB batches). EBS io2 costs $0.0036 per 1M appends. — [arXiv 2606.27051](https://arxiv.org/html/2606.27051)

**Worked costs (my arithmetic from the list prices above; us-east-1; excludes storage and free tiers unless noted)**

| Scenario (per month) | S3 Standard | S3 Express | R2 | Tigris (single-region) |
|---|---|---|---|---|
| 1M commits × 1 PUT | $5.00 | $1.13 (+ $0.013 upload for 4 KB records; $3.20 for 1 MB records) | $4.50 ($0 within 1M free tier) | $5.00 |
| 1M commits × 2 PUTs (data + pointer CAS) | $10.00 | $2.26 | $9.00 | $10.00 |
| + 1 GET/HEAD per commit (pointer read) | +$0.40 | +$0.03 | +$0.36 | +$0.50 |
| 1 poller, HEAD/GET every 1 s (2.592M req) | $1.04 | $0.08 | $0.93 | $1.30 |
| 1 poller, HEAD/GET every 100 ms (25.92M req) | $10.37 | $0.78 | $9.33 | $12.96 |
| 1 poller, LIST every 1 s (2.592M req) | $12.96 | ≈$2.93 (assumes LIST billed as PUT; see Gaps) | $11.66 | $12.96 |
| 100 idle readers, HEAD every 1 s | $104 | $7.78 | $93 | $130 |
| Storage, 100 GB | $2.30 | $11.00 | $1.50 | $2.00 |

### Inferences
- At 1M commits/month (about 0.4 commits/s average), request cost is about $5–$10/month on S3 Standard. Choice of backend should be driven by latency and semantics, not request price, until about 100M commits/month ($500–$1,000 on S3 Standard, or $113–$226 on Express).
- Polling dominates cost for many-reader designs. Using HEAD or GET with `If-None-Match: <etag>` on a pointer is about 12x cheaper than LIST on S3 Standard and R2 ($0.40 vs $5.00 per M on S3).
- Long-poll intervals (≥1 s) or pushing change notifications out-of-band keep reader costs bounded. S3 Event Notifications are unavailable on directory buckets.
- Express is about 4.4x cheaper per PUT and 13x cheaper per GET than Standard, but storage costs about 4.8x more per GB. That suits a hot commit log with compaction to S3 Standard, the pattern WarpStream and BtrLog use for archival.
- On R2, deletes are free and the first 1M Class A operations each month are free, so a low-volume database can run at about $0 in request cost.

### Gaps
- Internet egress per-GB for S3 Standard: the AWS page did not render its tables. It confirms only that the first 100 GB/month is free across services.
- Whether failed conditional requests (412/409) are billed, on any provider.
- Whether ListObjectsV2 on directory buckets bills at the Express PUT rate. Inferred from AWS grouping RenameObject with "PUT, COPY, POST, LIST", not confirmed.
- GCS and Azure operation prices were not collected; I prioritized the four stores in scope.
- CreateSession billing for Express.

---

## 6. Request-rate limits and prefix design

### Takeaway
S3 general purpose buckets scale per partitioned prefix to at least 3,500 PUT/COPY/POST/DELETE and 5,500 GET/HEAD requests per second. Scaling is gradual, with 503 SlowDown responses during re-partitioning. Very hot single objects (more than about 5,000 req/s) can see 503s. Express directory buckets claim 200k PUT and 2M GET TPS per bucket, with no prefix limits. GCS limits writes to one object name to about 1 per second.

### Cited Findings
- "When your application consistently generates more than 3,500 PUT/COPY/POST/DELETE or 5,500 GET/HEAD requests per second per prefix, you should implement strategies to distribute requests". During S3's gradual scaling, "you might receive HTTP 503 (Slow Down) responses." — [S3 performance design patterns](https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance-design-patterns.html) (fetched 2026-10-09)
- "If an application generates high request rates (typically sustained rates of over 5,000 requests per second to a small number of objects), it might receive HTTP 503 slowdown responses." — [same](https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance-design-patterns.html)
- AWS recommends randomized or sequential prefix patterns (e.g. `a1b2/log-…`), gradual ramp-up, exponential backoff on 503, and monitoring 5xx in CloudWatch. For "consistent high performance", it points to S3 Express One Zone. — [same](https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance-design-patterns.html)
- Directory buckets: up to 2M GET TPS and 200k PUT TPS per bucket. — [AWS News Blog, April 2025](https://aws.amazon.com/blogs/aws/up-to-85-price-reductions-for-amazon-s3-express-one-zone/)
- Directory buckets have "no prefix limits and individual directories can scale horizontally". — [S3 User Guide](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html)
- Bodner et al. observed S3 request error rates "just above 10%" while scaling IOPS, and attribute stragglers to client retry backoff. — [arXiv 2501.07771](https://arxiv.org/html/2501.07771)
- GCS: at most about 1 write/s to the same object name (429 if exceeded) and about 1,000 writes/s initial bucket capacity, which ramps. — [GCS quotas](https://docs.cloud.google.com/storage/quotas); [GCS request rate](https://docs.cloud.google.com/storage/docs/request-rate)

### Inferences
- A single-database commit log, even at 100 commits/s, is far below the per-prefix limits. Prefix sharding matters only for multi-tenant buckets (many databases in one bucket) or for heavy read fan-out.
- Use a per-database prefix (e.g. `<db-id-hash>/log/…`). With hashed db IDs, tenants spread across partitions.
- Zero-padded sequence numbers under one prefix (`log/00000000000000001234`) keep lexicographic LIST useful on general purpose buckets. On directory buckets, LIST order is not lexicographic (section 2), so do not rely on it there.
- Hot-pointer contention, with many writers doing If-Match CAS on one key, is bounded by the commit protocol's retry loop long before S3's request limits. On GCS it is capped at about 1 successful write/s per pointer object.

### Gaps
- AWS does not publish a per-key write rate limit for S3 general purpose buckets beyond the "small number of objects / >5,000 req/s" 503 note.
- No published per-key limits for directory buckets, R2, or Tigris were found.
- The Azure per-blob request target was not collected.
