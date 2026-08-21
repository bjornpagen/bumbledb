# 40 — The object-store capability

## The trait

The protocol needs exactly five operations. The trait is the protocol's
demand, not a vendor's offer — anything a vendor offers beyond this does
not appear:

```rust
pub trait ObjectStore: Send + Sync {
    /// GET. Ok(None) on 404.
    fn get(&self, key: &str) -> Result<Option<Fetched>>;

    /// GET with If-None-Match: <etag>. Ok(Unchanged) on 304 — the cheap
    /// manifest poll.
    fn get_if_changed(&self, key: &str, etag: &Etag) -> Result<Poll>;

    /// PUT with If-None-Match: "*". Ok(Created(etag)) or Ok(Exists) on 412.
    /// The log-slot arbitration primitive.
    fn put_create(&self, key: &str, bytes: &[u8]) -> Result<Create>;

    /// PUT with If-Match: <etag>. Ok(Swapped(etag)) or Ok(Moved) on 412.
    /// The manifest CAS primitive.
    fn put_swap(&self, key: &str, bytes: &[u8], etag: &Etag) -> Result<Swap>;

    /// DELETE (unconditional). The gc verb's tool.
    fn delete(&self, key: &str) -> Result<()>;
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
| Local filesystem | rename-based create-only | lockfile+rename CAS | the test/conformance impl; also the macOS sync target |

## Implementations shipped in v1

1. **`FsStore`** — local directory. Create-only = `O_CREAT|O_EXCL` temp +
   rename; CAS = etag-file compare under an flock. Exists for 80's
   conformance lanes (crash matrix, contention lane run in-process) and
   for case-2 local sync. No network dependency in the test suite.
2. **`S3Store`** — S3/Express/R2/OCI via one implementation, since all
   four speak SigV4 + the conditional headers.

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

TS side: `aws4fetch` (a ~4 KB SigV4 signer over platform `fetch`) rather
than `@aws-sdk/client-s3`. Recorded reasoning: the TS driver needs exactly
five HTTP verbs with signed conditional headers; the SDK's client stack
buys nothing but cold-start weight on Vercel.

## Retry law

`put_create` and `put_swap` are **never retried blindly** on ambiguous
outcomes (a timeout after the request may have landed): the follow-up is a
GET of the target key — if the body matches what we tried to write, the
operation succeeded (content-addressed comparison for log objects; etag
re-read for the manifest). 5xx/network on reads retry with jittered
backoff (base 50 ms, cap 2 s, 6 attempts) then surface as `Err`. This law
is what makes the crash matrix (80) total.
