# 70 — The cutover

> **Decision.** Hard cutover. No compatibility window, no dual-read path,
> no migration shim. The new representations replace the old ones in one
> release; a store written by the old format is not read by the new code
> and is not meant to be. One version number stands on the far side.

The existing PRD set already ran one hard cutover (the Log era replaced
its predecessor with zero back-compat and the tally to prove it). This is
the second, and it is larger, because it moves the representation under
both drivers at once. The discipline is the same: **delete the old shape
so no code can reach for it.**

## What the store format becomes

- **Document version `v:3`.** The manifest, checkpoint, and sidecar
  documents carry `"v":3`. The parser refuses `v:2` — there is no
  translator. A `v:2` store is migrated by a one-shot offline re-checkpoint
  (replay to a fresh `v:3` genesis), not by reading `v:2` at runtime.
- **One canonical encoding for every field.** Pending bytes are the
  codec's canonical rendering only; the base64 rendering does not exist.
  Every numeric field is exact; the JSON-`number` path does not exist.
- **A new package major** for `@bjornpagen/bumbledb-log` and a new crate
  version for `bumbledb-log`, bumped in lockstep to the same number, the
  way the workspace already gates lockstep. The peer range moves with it.
  (Agents never publish or tag; the owner runs that ceremony, interactive
  OTP.)

## What gets deleted

These are not refactors. They are removals — the old shape must not be
constructible.

| Deleted | Because | Doc |
| --- | --- | --- |
| `applied_pending: 0\|1` and the `+ applied_pending` addend | the generation is a total function of the `Chain` sum type | [30](30-pending-chain.md) |
| `pending: Option<…>` as a field beside the chain | `Pending` is a constructor of the chain, not a side-flag | [30](30-pending-chain.md) |
| `upsert` and its `put_swap`-over-non-equal-bytes | a checkpoint document is written once and never rewritten | [40](40-checkpoint-chain.md) |
| the mutable `prev` re-render inside `publish_checkpoint`'s loop | `prev` is inside the content hash | [40](40-checkpoint-chain.md) |
| `pid_alive` / `pidAlive` and every `kill(0)` probe | liveness is a 3-case sum; the lock is a fenced CAS lease | [20](20-store-contract.md) |
| `Ok(status.success())` and `code !== "ESRCH"` liveness readings | there is no liveness boolean to read | [20](20-store-contract.md) |
| the read-owner → probe → `rm(lockPath)` break sequence | a lease is broken only by expiry through the store CAS | [20](20-store-contract.md) |
| the base64 sidecar pending rendering (TS) | one canonical encoding, byte-identical across drivers | [60](60-codec-grammar.md) |
| the JSON-`number` round-trip for `u64` fields (TS) | numbers parse to `bigint`, exact | [60](60-codec-grammar.md) |
| `refresh_braid` as a second copy of the refresh transition | one stepper; `refresh`/`waitFor`/`catchUp` share it | [10](10-protocol-machine.md) |
| `waitFor` as a hand-transcribed refresh | `waitFor` is `refresh` with a predicate | [10](10-protocol-machine.md) |
| the downward `break`-on-hole log sweep | the sweep is a resumable bottom segment `[0, marker)` | [50](50-retention.md) |
| retention aging by `batch.header.timestamp` | retention ages by the trusted publish clock | [50](50-retention.md) |
| the "gc fodder" comment on `Published::Kept` | orphans are addressable and actually collected | [40](40-checkpoint-chain.md) [50](50-retention.md) |
| the manifest-birth arm on the replica read path (TS) | only the writer births a store; a replica refuses `ManifestMissing` | [10](10-protocol-machine.md) |

## The order of operations

The representations depend on each other, so they land in dependency
order, each behind the same green battery the workspace already runs
(rust fmt / clippy `-D warnings` / test; `check.sh`; `lean.sh`;
`spec-census.sh`; ts 403 + tsc + biome; ts-log + tsc + biome):

1. **[60](60-codec-grammar.md) — the grammar and codec.** Everything
   above it reads types this produces. Land the one grammar, the exact
   numbers, the half-open interval, the bounded row vector, the
   well-formed string, first.
2. **[20](20-store-contract.md) — the store contract.** The lease, the
   durable-success verbs, the total-sum outcomes, the key grammar, the
   handle lease. The machine reads outcomes from here.
3. **[30](30-pending-chain.md) — the chain sum** and
   **[40](40-checkpoint-chain.md) — the immutable checkpoint chain.**
   Independent of each other; both consume the codec and the store.
4. **[50](50-retention.md) — the floor invariant and the resumable
   sweep.** Consumes the checkpoint chain and the store.
5. **[10](10-protocol-machine.md) — the one machine.** Consumes all of
   the above; the drivers become thin executors of the transition table.
6. **Conformance flips to executing the table** ([10](10-protocol-machine.md)):
   the weak assertions are replaced by named-outcome assertions, the TS
   crash matrix and codec fuzz lanes are added, the store smoke lane ties
   outcomes to bytes and cleans its bucket.

## The proof obligation

The cutover is done when:

- No `kill(0)`, no `applied_pending`, no `upsert`, no base64 pending, no
  JSON-`number` `u64`, no `refresh_braid`, and no downward-break sweep
  appears anywhere in either driver — verified by grep, the same way the
  deletion tallies in the existing rollout receipts are verified.
- The conformance corpus is executed by *one* table and both drivers pass
  it with identical named outcomes on identical bytes — the parity lane's
  purpose becomes "prove the two thin drivers agree because they run one
  machine," and a divergence is a lane failure.
- The 141-row traceability table in [90](90-traceability.md) has every
  row resolved by a landed representation, not a patch.

## What does not change

The product laws (L1–L10), the braids, the five deployment cases, the
"recovery is replay" thesis, and the resident-writer mode all stand. This
cutover changes the representation of the protocol in code; it does not
renegotiate what the protocol promises. The numbered PRD set
(`00`–`90`) remains the product law; this subdirectory is how the
implementation is made to *be* that law rather than to approximate it
twice.
