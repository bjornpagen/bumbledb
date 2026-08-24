# 40 — The object-store capability

## The trait

The protocol needs exactly five operations. The trait is the protocol's
demand, not a vendor's offer — anything a vendor offers beyond this does
not appear:

```rust
pub trait ObjectStore: Send + Sync {
    /// GET. Ok(None) on 404.
    fn get(&self, key: &StoreKey) -> Result<Option<Fetched>>;

    /// GET with If-None-Match: <etag>. Ok(Unchanged) on 304 — the cheap
    /// manifest poll.
    fn get_if_changed(&self, key: &StoreKey, etag: &Etag) -> Result<Poll>;

    /// PUT with If-None-Match: "*". Ok(Created(etag)) or Ok(Exists) on 412.
    /// The log-slot arbitration primitive.
    fn put_create(&self, key: &StoreKey, bytes: &[u8]) -> Result<Create>;

    /// PUT with If-Match: <etag>. Ok(Swapped(etag)) or Ok(Moved) on 412.
    /// The manifest CAS primitive.
    fn put_swap(&self, key: &StoreKey, bytes: &[u8], etag: &Etag) -> Result<Swap>;

    /// DELETE (unconditional). The gc verb's tool.
    fn delete(&self, key: &StoreKey) -> Result<()>;
}

pub struct Fetched { pub bytes: Vec<u8>, pub etag: Etag }
pub enum Poll   { Unchanged, Changed(Fetched) }
pub enum Create { Created(Etag), Exists }
pub enum Swap   { Swapped(Etag), Moved }
```

House laws applied: outcomes are sums, never booleans (`Exists`/`Moved`
are proved answers in the `ConditionalWrite::Moved` tradition, not errors);
infrastructure failures (network, 5xx, auth) are the `Err` channel; the
driver's own code is monomorphized over `S: ObjectStore` — **zero `dyn`**
in our code (dependency internals exempt, per the engine's census law).

## Vendor matrix (verified)

| Vendor | Create-only | CAS | Notes |
| --- | --- | --- | --- |
| S3 general purpose | `If-None-Match: *` (Aug 2024) | `If-Match` ETag (Nov 2024) | strong read-after-write; conditionals free |
| S3 Express One Zone | yes (directory buckets) | yes | single-digit-ms; the latency-tier ruling for serverless writers |
| Cloudflare R2 | yes (S3-API extension) | yes | zero-egress checkpoint pulls; test wildcard-etag behavior (known dev-tool parity bugs) |
| OCI Object Storage | native if-none-match | native if-match | case-3 target; resident mode doesn't even need CAS |
| Local filesystem | link(2) create-only | pid-lockfile CAS | one on-disk protocol, two conforming implementations (Rust and TS), raced against each other in conformance; also the macOS sync target |

## Protocol consumers of the five verbs (nothing else is permitted)

Log slots: `put_create`. Manifest: `get` + `put_swap` (+ `put_create` at
birth). Tip probing: `get` per braid. Manifest polling: `get_if_changed`
(the replica's gc-safety heartbeat, 50). Checkpoints: `put_create` for
both the `.mdb` and the checkpoint json (content-addressed keys make
`Exists` a benign duplicate, not a race). Id-lease counters (`ids/…`):
`put_create` at counter birth (body = the first lease's end, claiming
`[0, width)` — 10 owns the width), then `get` + `put_swap` (read n, swap
n + width; `Moved` ⇒ re-read and retry — unbounded on purpose, with the
recorded reason in 10: lease traffic is width-times rarer than slot
traffic, so the counter has no contention to valve). `gc`: `delete`.
Capacity
reservations consume **no verbs** — they are rows in the log (60). This
map is exhaustive on purpose; a consumer missing from it (the original
draft omitted checkpoint uploads) is the same design error as a sixth
verb.

## Implementations shipped in v1

1. **`FsStore`** — local directory, ONE on-disk protocol specified here
   and spoken by both language implementations. **Create-only** = write
   to an `O_CREAT|O_EXCL` temp file, fsync it, publish with `link(2)`
   to the final key path, fsync the parent directory — link is chosen
   over rename because POSIX rename replaces an existing destination
   and therefore cannot arbitrate exclusivity; `EEXIST` on the link is
   the honest `Exists`. **Etag** = `blake3(content)`, lowercase hex,
   computed on every read and **never stored**: the bytes already
   answer the question, a sidecar or a random token would be a second
   answer waiting to disagree, and at protocol object sizes the hash
   cost is noise. **The mutation lock** (put_swap's exclusivity) = a
   pid-lockfile beside the key, published with the same
   exclusive-temp-plus-link discipline so it can never exist without
   its body — the owner pid; a contender probes the owner's liveness
   and breaks the lock iff the owner is dead. `Created` and
   `Swapped` return only after fsync of the object file and its parent
   directory: 00 law 1 says an acked commit *exists*, and at power loss
   a filesystem "exists" means nothing less — the sidecar's write
   discipline (50), applied at the store. **One machine is
   load-bearing, not descriptive** — and it is what makes the pid lock
   sound: link exclusivity and pid liveness are the
   arbitration primitives, and network filesystems historically weaken
   both — an `FsStore` prefix on a network mount is a misdeployment; no
   syscall can prove a mount local, so the refusal lives here in the
   vendor row instead. **Production tier,
   not a test double**: it is the whole backend of deployment case 5
   (the local fleet — primer-spec's parallel scope loops), the macOS
   sync target of case 2, and the store 80's conformance lanes (crash
   matrix, contention lane) run against in-process. Both language
   drivers ship it at parity: every lane that runs on `S3Store` runs on
   `FsStore`, in Rust and in TS, or the gap is reported. No network
   dependency in the test suite — or in case 5's production loop.
2. **`S3Store`** — S3/Express/R2/OCI via one implementation, since all
   four speak SigV4 + the conditional headers.

## Storage-class and availability ruling

The `S3Store` constructor takes **one** target: endpoint, region, bucket,
credentials (static keys or a caller-owned refresh), key prefix. `ckpt/*`, `log/*`, and the manifest ride that
one storage class. The dual-class split (a hot class for `log/*` —
Express One Zone or R2 — and a standard class for `ckpt/*` +
`manifest.json`) is configuration that arrives with its measured
trigger, not a second constructor now. Express's trade is recorded
honestly: 11-nines durability but **single-AZ availability (99.95 %
SLA)** — a zone event pauses writes (acks stall; nothing is lost;
replicas keep serving). Write availability through a zone event is a
**recorded v2** (dual-PUT to a second zone's bucket, cost ≈ one
standard PUT), refused for v1 because it is not yet a design: the
second bucket has no named reader, no failover read rule, and an async
second write whose silent failure is a representable divergence —
exactly the unspecified-machinery shape this file exists to refuse.
Trigger: a deployment that measures a zone event it cannot ride out on
pause-and-resume, or a measured commit-latency need standard-class S3
misses.

## The dependency ruling

The Rust `S3Store` is built on the `object_store` crate (Apache): it
carries SigV4, retry/backoff, and conditional-put modes for every vendor in
the matrix, maintained upstream. Recorded reasoning: a hand-rolled SigV4 +
HTTP stack is ~2–3k lines of security-sensitive code with no
representational payoff; the dependency earns its place by deleting it.
Refused-for-now alternative (hand-rolled minimal client) reopens only if
`object_store`'s conditional support regresses or its dependency tree
becomes a build-time problem. The engine workspace stays heed+blake3-pure —
`bumbledb-log` is a separate crate outside that purity boundary, like
`bumbledb-c`.

TS side: `@aws-sdk/client-s3` (AWS-owned, maintained). The grail pass
named `aws4fetch`; the owner killed that dependency — no commits in
about two years, open issues on query canonicalization, header signing,
`X-Amz-Content-Sha256`, empty-body POST, and streams — and the official
client is the replacement. The five verbs still map to real HTTP
preconditions (`If-None-Match`, `If-Match`, DELETE); the SDK must send
those headers, not guess.

## Retry law

`put_create` and `put_swap` are **never retried blindly** on ambiguous
outcomes (a timeout after the request may have landed): the follow-up is a
GET of the target key — if the body matches what we tried to write, the
operation succeeded (content-addressed comparison for log objects; etag
re-read for the manifest). The same comparison is the first move on every
`Exists` (10) — ambiguity absorption and slot loss are one rule, not two.
5xx/network on reads retry with jittered backoff (base 50 ms, cap 2 s,
6 attempts — chosen constants of the ordinary exponential shape; the
gated vendor smoke re-sizes them if a vendor's tail demands it) then
surface as `Err`. This law is what makes the crash matrix (80) total.

One access pattern deserves a named smoke test per vendor:
probe-404 → create → immediate probe. GET-before-PUT on the same key is
the shape S3's negative caching historically poisoned (Delta Lake fought
it in production); strong consistency covers it on paper today, and the
conformance smoke proves it per vendor row rather than trusting the
label — the Feral lesson (PostgreSQL shipped years of "serializable" that
wasn't) applied to storage.
