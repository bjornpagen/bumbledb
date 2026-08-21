# 80 — Conformance and gates

The driver gets the house assurance treatment: determinism is a pinned
oracle agreement, crashes are an iteration over reified protocol steps, and
every law is a test. All lanes run on `FsStore` — the suite needs no cloud
account; the `S3Store` gets a thin smoke lane gated behind credentials in
the environment (skipped-with-reason otherwise, per the ramdisk precedent).

## Lane 1 — replay determinism (the core oracle)

Generate random command sequences with the existing bench corpus generator
(theories + facts from `corpus_gen`), then compare three ways of arriving
at the same generation:

- **Direct**: apply batches straight to store A.
- **Replayed**: publish to a log via the writer, replay onto fresh store B.
- **Checkpoint-hopped**: checkpoint mid-sequence, restore store C from the
  checkpoint, replay the tail.

Gate: `catalog_digest(A) == catalog_digest(B) == catalog_digest(C)` at
every probed generation, across ≥ 100 generated worlds. This is the
three-oracle habit applied to replication; a disagreement is a trophy.

## Lane 2 — the crash matrix (protocol steps as data)

Both writer modes' sequences are reified enums (the `PublishStep`
discipline):

```rust
enum ServerlessStep { Encode, ApplyLocal, PutLog, Ack }
enum ResidentStep   { Encode, WriteSidecar, ApplyLocal, AckLocal, PutLog, DeleteSidecar }
```

For every proper prefix: execute the prefix against `FsStore`, kill,
recover (re-open per 50/60), and assert the postcondition table — no acked
commit is ever lost, no un-acked state survives observation, sidecar
resolution matches the two forced cases, and a fork discarded is a fork
gone. A new step extends the matrix by construction; a forgotten crash
case is a missing enum arm and fails to compile.

## Lane 3 — contention (serverless CAS arbitration)

N in-process writers (N ∈ {2, 4, 8}) commit interleaved random batches
against one `FsStore` log. Gates: the log is gap-free and each key was
created exactly once; every writer's acked commits appear in the log
exactly once; all writers' stores converge to equal `catalog_digest` after
final refresh; every `Contended` was followed by state advance. Fault
injection: the ambiguous-outcome retry law (40) is exercised by a store
wrapper that drops responses after applying writes — the GET-verify path
must resolve every ambiguity.

## Lane 4 — PITR and gc

Build a 500-generation history with checkpoints every 64. Gates: restore
to every generation g reproduces `catalog_digest` recorded at g during the
build; `gc` with window R deletes exactly the objects the retention law
names and a post-gc restore to any retained g still succeeds; a restore
into a gc'd gap refuses with `GapDetected`, never fabricates.

## Lane 5 — codec cross-goldens (Rust ⇄ TS)

A checked-in corpus of batches: every op kind, every value tag, boundary
values (`u64::MAX`, empty strings, max-width `FixedBytes`, half-open
interval edges), a multi-op group-commit batch, and every refusal case
(bad magic, version 2, flags ≠ 0, wrong fingerprint, wrong arity, wrong
tag, non-UTF-8 string). Both implementations must byte-produce and parse
the corpus identically; refusals must carry the same typed identity. Wired
into the `tags.json`-golden tier so drift is a compile/test failure on
either side.

## Lane 6 — engine-guarantee pins (from 30)

- Intern-mint determinism: same batch → two fresh stores → equal digests.
- Fresh-in-command: replayed fresh ids collide as ordinary functionality
  rejections; `ReplayDiverged` is the surfaced class during replay.
- Generation/no-op law: rejected and empty commits create no log objects
  and advance nothing.

## Law gates (the census tier)

- **Zero-dyn**: the engine census extends to `bumbledb-log`'s own code
  (dependency internals exempt).
- **Temporal law (TS)**: every exported async function awaits a store
  operation on some path; no async-in-name-only.
- **Allocation**: codec encode/decode allocate only output buffers (no
  per-row temporaries) — one `alloc_counter` window test; everything else
  in the driver is network-bound by construction and exempt by the
  recorded FFI/boundary rule.
- **Vocabulary census**: `spec-census.sh` gains the driver's tokens
  (manifest fields, op kinds, error identities) across Rust/TS/docs.

## Error identity (pinned here, used everywhere)

One taxonomy, sums not strings: `Refused` (fingerprint/version/manifest —
typed per cause), `GapDetected`, `ReplayDiverged` (corruption-class),
`Contended` (an outcome, never an error), `StoreErr` (the vendor `Err`
channel). TS mirrors with exported `Err*` values on the SDK's existing
idiom.

## Performance pins (attribution-first; no claims without these)

Recorded by a bench lane, not asserted in docs: serverless commit latency
(FsStore floor + S3 smoke when credentialed), resident local-ack latency,
group-commit throughput at batch sizes {1, 16, 512}, cold-open time vs
checkpoint size {8, 64, 256 MiB}, refresh probe cost. The 00-product
envelope is replaced by these numbers in the first release notes.
