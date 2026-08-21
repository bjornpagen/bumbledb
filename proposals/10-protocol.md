# 10 — The object protocol

## Key layout

All keys live under a caller-supplied prefix. A prefix is a store; a tenant
is a prefix. Generation numbers render as zero-padded lowercase hex, 16
chars (`{g:016x}`), so lexicographic order is numeric order.

```
<prefix>/manifest.json                 — the pointer (CAS-guarded)
<prefix>/log/{g:016x}                  — command batch producing generation g (create-only)
<prefix>/ckpt/{g:016x}.mdb             — compacted data.mdb at generation g (immutable)
```

Per-tenant deployments use `<root>/t/<tenant-id>/…` with the shared
reference-data store at `<root>/t/_shared/…`. The driver takes the prefix as
an opaque string; tenancy is layout, not code.

## The manifest

Canonical single-line UTF-8 JSON (one spelling; producers emit exactly this
field order; consumers parse strictly, refuse unknown fields):

```json
{"v":1,"fingerprint":"<64 hex>","checkpoint":{"g":123,"key":"ckpt/000000000000007b.mdb","digest":"<64 hex>"},"log_floor":123,"writer":"<opaque id or empty>"}
```

- `v` — manifest format version; consumers refuse ≠ 1.
- `fingerprint` — the store's schema fingerprint (hex of the engine's
  32-byte fingerprint). Every reader and writer refuses a mismatch before
  doing anything else.
- `checkpoint` — the newest checkpoint: its generation, key, and blake3
  digest of the object bytes. `g = 0` with an empty key means "no
  checkpoint yet; bootstrap from an empty store via `Db::create`".
- `log_floor` — a **lower bound** on the tip. The manifest is advisory
  about the head; the truth is the log objects themselves.
- `writer` — advisory identity of the resident writer (empty in serverless
  mode). Informational only; arbitration is CAS, never this field.

Creation: `PUT manifest.json` with `If-None-Match: *`. Update (checkpoint
publication or floor advance): `PUT` with `If-Match: <etag read>`. A 412
means re-read and reconcile; the manifest is never blind-overwritten.

## Log objects

`log/{g:016x}` is created with `If-None-Match: *` and is **immutable
forever after** — never overwritten, never appended. Exactly one writer can
create each key; the 412 loser pulls the winner's object, applies it, and
retries its own commit at the next index. Contents: one command batch
(20-command-codec.md) whose header carries `base_generation = g − 1`.

Total order without consensus: the sequence `log/1, log/2, …` is the
history, and CAS on each key is the arbitration. There is no other
coordination primitive in the protocol.

## Tip discovery

From any known generation `k` (a replica's local generation, or
`manifest.log_floor`): probe `GET log/{k+1:016x}`; on success apply and
advance; on 404 the tip is `k`. Probing is the one discovery mechanism —
no LIST dependence (LIST is eventually-shaped on some vendors; GET-after-PUT
is strongly consistent on all supported ones). A 404 probe costs a GET
request (≈ $0.00003/1000 on Express).

## Checkpoints

A checkpoint is the engine's `compact()` output — the single `data.mdb`
file — uploaded as `ckpt/{g:016x}.mdb` where `g` is the store's generation
at compaction (compaction runs under a read transaction, so `g` is exact).
`lock.mdb` is never shipped; LMDB recreates it at open. After upload, the
publisher CAS-updates the manifest (`checkpoint`, `log_floor = g`).
Checkpoint cadence is a deployment knob: every K generations or B bytes of
log, whichever first (defaults: K = 256, B = 16 MiB).

Restore-side verification: after download, blake3(bytes) must equal
`checkpoint.digest`; the opened store's generation must equal
`checkpoint.g` and its fingerprint must match the manifest. Any mismatch is
a typed refusal, never a warning.

## Retention, truncation, PITR

- The PITR window `R` (days) is policy: object-store lifecycle rules delete
  `log/*` and `ckpt/*` older than `R`, **except** the newest checkpoint and
  all log objects `≥` its generation, which are exempt (implemented by
  lifecycle-on-prefix plus the publisher tagging the live checkpoint, or by
  a driver `gc` verb that deletes explicitly — the driver verb is the v1
  ruling; lifecycle rules are the v2 automation).
- PITR to generation `g`: pick the newest checkpoint with `ckpt.g ≤ g`,
  replay log to `g`, done. Restore never mutates the source prefix; it
  materializes into a fresh prefix or a local store.
- Bucket versioning is not required by the protocol (log objects are
  immutable; the manifest is the only mutable key, and its history is
  reconstructible from the log). Enabling it is harmless belt-and-braces.

## Consistency assumptions (and why they hold)

The protocol requires exactly three store properties, all verified on the
supported vendors (40-object-store.md): strong read-after-write for GET
after PUT; atomic create-only PUT (`If-None-Match: *`); atomic
compare-and-swap PUT (`If-Match: <etag>`). Nothing else — no atomic
multi-key operations, no LIST consistency, no append.

## Failure semantics

| Event | Outcome |
| --- | --- |
| Crash after log PUT, before local apply | Restart replays `log/{local+1}` — idempotent by the index=generation law |
| Crash after local commit, before log PUT (resident mode) | The one-slot sidecar republishes (60-writer.md); serverless mode cannot enter this state (publish precedes ack, local state is disposable) |
| CAS 412 on `log/{g+1}` | Pull winner, apply, retry the host write against the new state — semantically a retried write; the re-judgment verdict may legitimately change |
| Manifest CAS 412 | Re-read, keep the newer checkpoint, retry floor advance |
| Torn/partial download | Digest check refuses; re-pull |
| Fingerprint mismatch anywhere | Typed refusal; migration is out of scope (00) |
