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

    /// PUT with If-None-Match: "*". Created, Exists, or Ambiguous
    /// (409 / timeout / a retried PUT the transport cannot prove).
    /// The log-slot arbitration primitive.
    fn put_create(&self, key: &StoreKey, bytes: &[u8]) -> Result<Create>;

    /// PUT with If-Match: <etag>. Swapped, Moved, or Ambiguous.
    /// The manifest CAS primitive.
    fn put_swap(&self, key: &StoreKey, bytes: &[u8], etag: &Etag) -> Result<Swap>;

    /// DELETE (unconditional). The gc verb's tool.
    fn delete(&self, key: &StoreKey) -> Result<()>;
}

pub struct Fetched { pub bytes: Vec<u8>, pub etag: Etag }
pub enum Poll   { Unchanged, Changed(Fetched) }
pub enum Create { Created(Etag), Exists, Ambiguous }
pub enum Swap   { Swapped(Etag), Moved, Ambiguous }
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
| Local filesystem | link(2) create-only | fenced CAS lease | one on-disk protocol, two conforming implementations (Rust and TS), raced against each other in conformance; also the macOS sync target |

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
   cost is noise. **The mutation lock** (put_swap's exclusivity) is a
   fenced CAS lease `{holder, token, expires}` — an object acquired
   and broken only through the store's own CAS, never through
   read-owner → probe → unlink. A contender breaks a lease iff it is
   *expired* (a fact of the lease's own bytes). Every write carries
   its fencing token, so a stale holder's write is rejected by the
   CAS it no longer wins. Liveness is `Alive | Dead | Unknown`;
   `Unknown` never breaks a lease. `Created` and `Swapped` return only
   after fsync of the
   object file and its parent directory: 00 law 1 says an acked commit
   *exists*, and at power loss a filesystem "exists" means nothing
   less — the sidecar's write discipline (50), applied at the store.
   **Keys are a grammar**; the temp and lease namespaces are disjoint
   by construction (a reserved prefix no `StoreKey` can spell) and
   swept at open. **One machine is load-bearing, not descriptive** —
   link exclusivity is a local-filesystem primitive, and network
   filesystems historically weaken it — an `FsStore` prefix on a
   network mount is a misdeployment; no syscall can prove a mount
   local, so the refusal lives here in the vendor row instead. **Production tier,
   not a test double**: it is the whole backend of deployment case 5
   (the local fleet — primer-spec's parallel scope loops), the macOS
   sync target of case 2, and the store 80's conformance lanes (crash
   matrix, contention lane) run against in-process. Both language
   drivers ship it at parity: every lane that runs on `S3Store` runs on
   `FsStore`, in Rust and in TS, or the gap is reported. No network
   dependency in the test suite — or in case 5's production loop.
2. **`S3Store` / `s3Store`** — S3/Express/R2/OCI via one constructor in
   each language, since all four speak SigV4 + the conditional headers.
   Rust ships `S3Store` over `object_store` (receipt box B). TypeScript
   ships `s3Store` over `@aws-sdk/client-s3` (receipt box C). Same five
   verbs, same retry/GET-verify law, same one-target constructor. The
   vendor ETag rides the opaque token verbatim. Both gated smokes share
   one env contract.
3. **`MemStore` / `memStore`** — the third store: the five verbs over
   one in-process map. Etags are blake3 of the bytes, the same mint
   `FsStore` uses; the brand is the contract, not the hash algorithm.
   Create-only and CAS are trivially atomic under single-process
   semantics — which is the honest scope statement, declared where the
   type lives. No persistence, no cross-process claim, no configuration.
   Its consumer is every store-semantic and retry-law test that never
   touches a disk (those bodies migrated off `FsStore` tempdirs) and
   every library user's unit tests. Multiprocess stays on disk.

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

ACCEPT `object_store` + `tokio` as unconditional crate deps. One way to
build the crate; no feature matrix. The embedded use case's TS consumers
never compile this crate. Reopen: an embedded Rust consumer that
measures the build cost.

Keep a multi-thread Tokio runtime. The writer's publisher and the duty
thread call store verbs on other OS threads; a current-thread runtime
cannot drive two `block_on` callers. Construct `S3Store` outside an
async context — `Handle::try_current` at `new()` is a typed refusal.
Reopen: a consumer that only ever calls verbs from one thread and
measures the extra workers as cost.

TS side: `@aws-sdk/client-s3` (AWS-owned, maintained). The five verbs
map to real HTTP preconditions (`If-None-Match`, `If-Match`, DELETE);
the SDK sends those headers, this module does not guess. Node **>=24**
is the floor everywhere the TypeScript store ships or tests (`engines`
in `ts/`, `ts/npm/*`, and `ts-log`; the `.ts` test runner and build
scripts; AL2023 CI cells install `nodejs24`; Lambda is `nodejs24.x`).
Never 22.

Both-language credentials are one sum: static keys (id, secret,
optional session token) | a caller-owned refresh callback. This is not
the SDK default provider chain and not a generic on the store. Rust
spells the refresh arm as `dyn` (the enum arm, the `RefreshProvider`
field, and the boxed future `CredentialProvider` forces) — three exact
lines pinned in the census on the `Error::source` precedent, reason
attached: caller-owned credential behavior at a foreign async-trait
boundary; cold path. Zero other log-driver dyns.

## Receipt boxes

- [x] B: Rust `S3Store` over `object_store` — the duty binary is the
      cloud consumer whose absence kept the store out of scope. Five
      verbs, one storage class, retry/GET-verify, gated smoke.
      Landing `f6c338e0`. Refresh kept `bc7ef05b`. Multi-thread runtime
      `44e69915`. Smoke `ff097be2`.
- [x] C: TS `s3Store` over `@aws-sdk/client-s3` — the app Lambda's
      writer is the consumer. Same five verbs and one constructor.
      Official-client landing `7ada883d`. `memStore` `6de97425`. Node
      floor 24 `06f767f2`. Smoke `ff097be2`.

## Retry law

A conditional write whose result the transport cannot prove (S3 409,
timeout, a retried PUT) is `Ambiguous`, never a proved `Exists` or
`Moved`. `put_create` and `put_swap` are **never retried blindly**
on that arm: the machine resolves it with the GET-verify law — if the
body matches what we tried to write, the operation succeeded
(content-addressed comparison for log objects; etag re-read for the
manifest). The same comparison is the first move on every `Exists`
(10) — ambiguity absorption and slot loss are one rule, not two. `put_create`
against a directory is a key-shape fault, not `Exists`.
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
