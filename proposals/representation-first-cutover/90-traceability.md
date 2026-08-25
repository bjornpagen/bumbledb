# 90 — Traceability: 141 findings → the decision that dissolves them

Every finding from the bug bash, in id order (the original array index),
with the one decision that makes its state unrepresentable. **Sev**: C
critical, J major, m minor. **Doc** is the *primary* lever — the deepest
representational move; a finding may also be touched by a document's
broader "Dissolves" line, but each is resolved here exactly once. A row
is closed when its doc's representation lands ([70](70-cutover.md)), not
by a patch at the site.

| # | Sev | Doc | Finding | Dissolving move |
| --- | --- | --- | --- | --- |
| 0 | C | [40](40-checkpoint-chain.md) | Same-candidate publish race clobbers the winner's `prev` | `prev` inside the content hash; document written once, never rewritten |
| 1 | C | [30](30-pending-chain.md) | Checkpointer publishes a forged checkpoint on an applied pending | compaction input is `Settled`; a `Pending` chain cannot compact (type error) |
| 2 | C | [20](20-store-contract.md) | Rust `pid_alive` treats EPERM as dead, breaks live locks | liveness is `Alive\|Dead\|Unknown`; lock is a fenced CAS lease, no `kill(0)` |
| 3 | C | [20](20-store-contract.md) | Lock-break is read/probe/unlink, not atomic | lease broken only by expiry through the store CAS; no path to unlink |
| 4 | C | [20](20-store-contract.md) | Rust breaks a live lock on EPERM while TS honors it | one lease primitive, one liveness sum, both drivers |
| 5 | C | [30](30-pending-chain.md) | `refresh_braid` skips pending resolution, merges a fork | `Chain` is a sum; no reader can skip the `Pending` match |
| 6 | C | [60](60-codec-grammar.md) | Leading BOM decodes to different values on Rust vs TS | one string grammar, bytes-in/bytes-out, one codec |
| 7 | C | [50](50-retention.md) | Stale writer resurrects a retention-swept slot | floor is a write-path precondition; a below-floor create is refused |
| 8 | C | [20](20-store-contract.md) | Unconditional `rm` lets two processes hold one key lock | lease identity is a fencing token; two current holders unrepresentable |
| 9 | C | [60](60-codec-grammar.md) | `chain.json` pending: Rust hex vs TS base64, cross-open destroys pending | one canonical encoding, byte-identical across drivers |
| 10 | J | [40](40-checkpoint-chain.md) | Kept/crashed checkpoint candidates leak `ckpt` objects forever | orphans are digest-addressable and reachability is decidable → collected |
| 11 | J | [60](60-codec-grammar.md) | `schema_file` panics (index OOB) on malformed theory | parse over a validated grammar returning `TheoryFile::Shape` |
| 12 | J | [60](60-codec-grammar.md) | `unhex` slices `&str` at fixed offsets → UTF-8 boundary panic | grammar parse, never byte-offset indexing |
| 13 | J | [50](50-retention.md) | Interrupted log sweep strands all slots below the gap | sweep is a resumable contiguous bottom segment `[0, marker)` |
| 14 | J | [50](50-retention.md) | duty swallows `Published::Refused`/`Gc::Refused`, reports success | duty result is a total sum; exit code a total function; refusal ≠ success |
| 15 | J | [50](50-retention.md) | Crash mid-sweep permanently strands old slots; sweep can't resume | upward-from-marker sweep resumes from where it stopped |
| 16 | J | [50](50-retention.md) | Crash mid checkpoint-sweep strands checkpoints, orphans a `.mdb` | immutable Merkle backlink walk; `.json`+`.mdb` deleted as one unit |
| 17 | J | [40](40-checkpoint-chain.md) | Loser/pre-CAS checkpoint objects are uncollectable | known-orphan by digest; complement of reachable set is swept |
| 18 | J | [50](50-retention.md) | Retention ages by writer-claimed batch timestamp | retention ages by the trusted publish clock |
| 19 | J | [20](20-store-contract.md) | `delete` removes the lockfile of a live `put_swap` | no lockfile to remove; the lease is a CAS'd object with a token |
| 20 | J | [20](20-store-contract.md) | `put_swap` check-then-rename; other verbs bypass the lock | every mutation carries a fencing token the CAS enforces |
| 21 | J | [20](20-store-contract.md) | Rename/delete not dir-fsynced; power loss reverts manifest | success means durable-and-visible; fsync before ack, always |
| 22 | J | [20](20-store-contract.md) | Pid reuse, zombies, pid 0, huge pids wedge `put_swap`; drivers disagree | lease has an expiry, not a pid; no foreign-process probe |
| 23 | J | [20](20-store-contract.md) | Lock-break TOCTOU produces two `putSwap` holders | acquisition is one CAS; a fresh lease cannot be unlinked |
| 24 | J | [20](20-store-contract.md) | The retry/GET-verify ambiguity law is absent from the TS store | `Ambiguous` is an outcome arm the machine must resolve, both drivers |
| 25 | J | [20](20-store-contract.md) | s3Store maps 409 to the infra channel; Rust maps it to Exists | 409 is `Ambiguous`; one GET-verify law resolves it |
| 26 | J | [20](20-store-contract.md) | TS credential refresh callback memoized once, breaks rotation | credentials consulted per request by contract |
| 27 | J | [20](20-store-contract.md) | Acked `putCreate` vanishes at power loss: ancestor dirs unfsynced | ack minted after object + parent-dir fsync |
| 28 | J | [20](20-store-contract.md) | Rust reports 409 on `put_create` as a *proved* `Exists` | 409 is `Ambiguous`, never a proved outcome |
| 29 | J | [20](20-store-contract.md) | Verbs panic via `block_on` inside an async context | async/sync boundary explicit in the trait; misuse won't compile |
| 30 | J | [10](10-protocol-machine.md) | Refused re-establish leaves the replica db-less; next refresh panics | "db-less" is a represented state the machine refuses, not a panic |
| 31 | J | [30](30-pending-chain.md) | Parse-valid sidecar corruption wedges a braid, names an innocent writer | sidecar read is a sum; `Corrupt` → discard-and-re-pull |
| 32 | J | [40](40-checkpoint-chain.md) | Catalog audit never fires when catch-up passes through the floor | catalog claim audited inside the one seed transition |
| 33 | J | [30](30-pending-chain.md) | Detached composite loses one-by-one fallback on a competing drain | resolution is one fold over `Pending`, shared by all resolvers |
| 34 | J | [30](30-pending-chain.md) | Loss-path fallback aborts on first `Err`, drops remaining segments | the fold returns remaining segments as data, never aborts |
| 35 | J | [30](30-pending-chain.md) | A wedged braid's backlog blocks every braid; writer unopenable | wedge is a per-braid marking; backlog is per-braid `Pending` |
| 36 | J | [30](30-pending-chain.md) | Crash in open catch-up destroys a recoverable pending (identity misfire) | one `generation` function; recovery cannot misfire the identity |
| 37 | J | [60](60-codec-grammar.md) | Fixed interval ending at MAX accepted by TS, refused by Rust | half-open interval: the ceiling is not a value |
| 38 | J | [20](20-store-contract.md) | `tenants.get` returns an already-disposed replica | handle is a refcounted lease; a borrow pins it live |
| 39 | J | [20](20-store-contract.md) | Concurrent gets open two replicas on one dir; each sweeps the other | directory has cross-process exclusivity (a lease) |
| 40 | J | [50](50-retention.md) | `adoptManifest` commits the etag before the checkpoint fetch → frozen floor | adopt-pointer + adopt-checkpoint is one atomic transition |
| 41 | J | [10](10-protocol-machine.md) | `waitFor` bypasses heartbeat, wholeness, pass counting | `waitFor` is `refresh` with a predicate; one shared stepper |
| 42 | J | [30](30-pending-chain.md) | Steady-state refresh never re-checks the wholeness identity | generation is a total function of `Chain`, checked on every path |
| 43 | J | [10](10-protocol-machine.md) | Open-phase decided by code path, not provenance | provenance is a `ReplicaState` value the arm matches on |
| 44 | J | [10](10-protocol-machine.md) | No per-braid wedging; one corruption takes down all braids | `Wedged{braid}` is a per-braid marking, not a whole-refresh abort |
| 45 | J | [30](30-pending-chain.md) | `pending:null` persisted before re-judgment; crash drops the batch | transition to `Settled` written only after the fold resolves |
| 46 | J | [50](50-retention.md) | `resolveColdPending` re-judges an already-published batch (gc'd slot) | pending fold consults the floor; below-floor = published, not re-judged |
| 47 | J | [30](30-pending-chain.md) | Corrupt `chain.json` wedges open instead of discard-and-re-pull | `Corrupt` arm routes to the disposable-law discard |
| 48 | J | [20](20-store-contract.md) | No cross-process exclusivity on the replica dir | one owner per dir via the lease primitive |
| 49 | J | [60](60-codec-grammar.md) | TS accepts a fixed interval whose end is the domain ceiling | half-open interval; ray unrepresentable on decode and encode gate |
| 50 | J | [60](60-codec-grammar.md) | Row-count amplification: zero-field relation → billions of rows (OOM) | row count and bytes are one length-delimited type; count can't outrun bytes |
| 51 | J | [10](10-protocol-machine.md) | Any id demand > 4096 can never succeed; each retry burns the pool | lease algebra: `Refused(OverWidth)`, one contiguous draw |
| 52 | J | [10](10-protocol-machine.md) | Async commit body's post-await ops silently dropped, acks accepted | body awaited to completion before the batch is sealed |
| 53 | J | [20](20-store-contract.md) | S3 smoke round-trip passes a wrong-arity row, can never succeed | store lane tests the one contract with a correct-arity row |
| 54 | J | [20](20-store-contract.md) | Rust EPERM-as-dead breaks live locks the TS store honors | one liveness sum with an `Unknown` arm that never breaks |
| 55 | J | [20](20-store-contract.md) | Lambda module-scope replica promise poisons the sandbox on failed open | replica held as a value handle, not a memoized promise |
| 56 | J | [10](10-protocol-machine.md) | TS has no crash-arm coverage for its pending-recovery machinery | conformance executes the one table; TS runs the same matrix |
| 57 | J | [60](60-codec-grammar.md) | TS decoder/parsers have no fuzz/truncation coverage | codec fuzz lane mirrors the Rust mutation lane against one grammar |
| 58 | J | [20](20-store-contract.md) | `put_create` visibility precedes durability | `Created` minted after fsync; checkpoint can't reference a lost slot |
| 59 | J | [30](30-pending-chain.md) | Checkpointer publishes a poisoned checkpoint on an applied pending | compaction takes `Settled`; `Pending` cannot compact |
| 60 | J | [10](10-protocol-machine.md) | TS catch-up gap arm throws instead of discard-and-reseed | `Reseed` is a `RefreshOutcome` arm both drivers run |
| 61 | J | [10](10-protocol-machine.md) | TS corruption aborts the whole refresh; later braids starve | `Wedged{braid}` marks one braid; the pass steps over it |
| 62 | J | [20](20-store-contract.md) | TS writer lacks the ambiguous-create retry law; 409 → ErrStore | `Ambiguous` outcome + the GET-verify law, both drivers |
| 63 | J | [60](60-codec-grammar.md) | FixedInterval ceiling divergence on decode and the encode gate | half-open interval type; ceiling not a value |
| 64 | J | [10](10-protocol-machine.md) | TS has no corruption wedge; publish-law check after chain advanced | `Wedged` arm + `Pending` fold: a refusal never advances |
| 65 | J | [20](20-store-contract.md) | fs lock probe: `kill -0` treats alive other-uid owner as dead | liveness `Unknown` on EPERM; lease never broken on `Unknown` |
| 66 | J | [20](20-store-contract.md) | S3 409 arm divergence: Rust → loss path, TS → infra error | 409 is `Ambiguous`; one resolution law |
| 67 | J | [50](50-retention.md) | TS `adoptManifest` commits the etag before the checkpoint → frozen floor | adopt is one atomic transition |
| 68 | J | [30](30-pending-chain.md) | TS omits the steady-state wholeness check | generation is a total function of `Chain`, checked every pass |
| 69 | J | [40](40-checkpoint-chain.md) | TS never audits the checkpoint catalog claim | catalog audited inside the one seed transition, both drivers |
| 70 | J | [50](50-retention.md) | `publishPending` re-creates a vanished slot, forges history below floor | below-floor create is a refused write |
| 71 | J | [20](20-store-contract.md) | Tenant LRU evicts and disposes the replica it is returning | borrow pins the handle's refcount against eviction |
| 72 | J | [30](30-pending-chain.md) | Checkpointer compacts a store carrying an applied-but-unpublished pending | compaction takes `Settled`; mdb-gen > vector-sum unrepresentable |
| 73 | J | [20](20-store-contract.md) | Rust breaks a live lock owned by another user (EPERM), diverging from TS | one liveness sum; lease not broken on `Unknown` |
| 74 | m | [60](60-codec-grammar.md) | u64 vector-sum overflow: hostile checkpoint g values panic/wrap the order | parser bounds values; sums are checked/saturating |
| 75 | m | [50](50-retention.md) | Stale sidecar temp files from a crashed process never cleaned | temps in a reserved namespace, swept at open by any successor |
| 76 | m | [60](60-codec-grammar.md) | `parse_value` resolves multi-arm objects by alphabetical key order | exactly-one-arm sum; a multi-arm object is refused |
| 77 | m | [60](60-codec-grammar.md) | Checkpoint sums are unchecked u64 additions over parsed values | checked/saturating sums; bounded parse |
| 78 | m | [60](60-codec-grammar.md) | `restore_to_vector` silently ignores unknown braid ids in the target | target parsed against the derived braid set; unknown → refuse |
| 79 | m | [60](60-codec-grammar.md) | duty argv parser lets a flag swallow the next flag as its value | argv parsed as a grammar; a flag without a value is refused |
| 80 | m | [50](50-retention.md) | duty tests assert slack arms, never observe the publish through the binary | duty outcome is a sum; the test observes publish + exit code |
| 81 | m | [20](20-store-contract.md) | Temp namespace collides with legal StoreKeys and crash litter | temp/lease namespace disjoint from the `StoreKey` grammar |
| 82 | m | [20](20-store-contract.md) | `put_create` answers `Exists` when the destination is a directory | a directory at a key is a key-shape fault, not `Exists` |
| 83 | m | [20](20-store-contract.md) | TS `memStore` returns aliased internal buffers; etag goes stale | every impl reads a fresh buffer out (mem clones like fs) |
| 84 | m | [20](20-store-contract.md) | S3 body-stream failures escape the `ErrStore` channel | every store failure wraps `ErrStore`, including the body stream |
| 85 | m | [20](20-store-contract.md) | Pid reuse wedges `putSwap` unboundedly | lease has an expiry, not a pid to reuse |
| 86 | m | [50](50-retention.md) | Stale same-pid temp litter → spurious ErrStore; orphans never swept | reserved temp namespace; swept at open |
| 87 | m | [20](20-store-contract.md) | `memStore.get`/`getIfChanged` return the live internal buffer | fresh buffer out on every read |
| 88 | m | [20](20-store-contract.md) | `object_store` retry re-sends conditional PUTs → false `Exists`/`Moved` | a re-sent conditional write is `Ambiguous`, verified by GET |
| 89 | m | [20](20-store-contract.md) | S3 constructor accepts prefixes later verbs reject; TS uses them | one key/prefix grammar across all three impls |
| 90 | m | [20](20-store-contract.md) | S3Store rejects control-char keys that fs/mem accept | one `StoreKey` grammar, one accepted set |
| 91 | m | [20](20-store-contract.md) | Rust accepts `region:"auto"` without endpoint; TS refuses it | one construction contract, one accepted config |
| 92 | m | [20](20-store-contract.md) | Refresh credential closure runs blocking I/O on tokio workers | credentials consulted off the worker threads by contract |
| 93 | m | [20](20-store-contract.md) | s3_smoke prefix is pid+seq; collides across machines/re-runs | collision-free derived prefix |
| 94 | m | [20](20-store-contract.md) | Smokes never clean their bucket objects; unbounded litter | the store lane cleans its bucket |
| 95 | m | [20](20-store-contract.md) | Create-only race test never ties the winner's `Created` to bytes | outcome tied to persisted bytes |
| 96 | m | [10](10-protocol-machine.md) | Scream never fires for alternating (A,B,A,B) repair signatures | scream tracks the *set* of recent signatures, not the last |
| 97 | m | [60](60-codec-grammar.md) | Corrupt sidecar g values overflow the wholeness arithmetic, panic | bounded parse; overflow is a refusal, not a panic |
| 98 | m | [50](50-retention.md) | duty resets `log_bytes` to zero, discarding window bytes | meter subtracts the snapshot's share, never zeros |
| 99 | m | [50](50-retention.md) | `ckpt_sum` never refreshed on `re_establish` → spurious duties | `ckpt_sum` re-seeded from the adopted floor |
| 100 | m | [50](50-retention.md) | duty swallows every failure silently, including corruption refusals | duty outcome sum; refusal screams, exit code reflects it |
| 101 | m | [10](10-protocol-machine.md) | Deposition/ack-drop skipped when the usurper's bytes fail to decode | deposition derived from slot ownership (header), not body decode |
| 102 | m | [50](50-retention.md) | `JoinHandle` vector grows unbounded over a writer's lifetime | finished handles reaped in steady state, not only at `quiesce` |
| 103 | m | [50](50-retention.md) | Crash mid-duty leaves `<dir>.ckpt{seq}` scratch nothing cleans | scratch is a lease with expiry, swept at open |
| 104 | m | [60](60-codec-grammar.md) | `decodeBatch` has no row-count cap; tiny object → OOM | row count and bytes are one length-delimited type |
| 105 | m | [60](60-codec-grammar.md) | `encodeBatch` substitutes U+FFFD for lone surrogates | string cell is `WellFormedUtf8` via a fatal encoder |
| 106 | m | [60](60-codec-grammar.md) | `encodeBatch` doesn't validate `prev`/fingerprint byte length | digest fields are `[u8;32]`; wrong length unconstructible |
| 107 | m | [60](60-codec-grammar.md) | Sidecar pending: Rust hex vs TS base64 in one `v:2` format | one canonical encoding |
| 108 | m | [60](60-codec-grammar.md) | Sidecar/checkpoint u64 round-trip through JS `number` → precision loss | numbers parse to `bigint`, exact |
| 109 | m | [10](10-protocol-machine.md) | Catch-up drains one braid to its tip before the next (no round-robin) | stepper takes one slot per braid per round |
| 110 | m | [30](30-pending-chain.md) | Checkpoint `data.mdb` written without fsync before the fsynced sidecar | seed mdb + sidecar are one crash-consistent unit |
| 111 | m | [50](50-retention.md) | `writeSidecar` leaks temp files; none swept after SIGKILL | reserved temp namespace, swept at open |
| 112 | m | [20](20-store-contract.md) | `waitFor` keeps fetching/persisting into a disposed replica | a disposed handle is a distinct type every verb refuses |
| 113 | m | [60](60-codec-grammar.md) | Parsers surface `RangeError` on fractional numbers | fractional number is a typed parse refusal at the boundary |
| 114 | m | [10](10-protocol-machine.md) | Heartbeat/pass accounting diverges (TS counts braid refreshes, Rust not) | heartbeat/pass counter live inside the one shared stepper |
| 115 | m | [20](20-store-contract.md) | Tenant id validation diverges: TS refuses ids Rust accepts | one tenant-id grammar shared with the key grammar |
| 116 | m | [50](50-retention.md) | Exists-then-vanished slot retried unbounded; retry forges swept slot | below-floor create refused; not a loop |
| 117 | m | [10](10-protocol-machine.md) | HotKey contention payload diverges; empty-violation silent in TS | one contention outcome value; empty-violation is impossible/refused |
| 118 | m | [10](10-protocol-machine.md) | Spanning-commit test never asserts `ErrSpanningCommit` | conformance asserts the table's named outcome |
| 119 | m | [10](10-protocol-machine.md) | `reserve` with negative/zero count silently returns `[]` | lease `count` unsigned; a negative demand is unconstructible |
| 120 | m | [30](30-pending-chain.md) | TS `applySlot` advances/persists the chain before the publish-law refusal | advancing *is* the transition the refusal does not produce |
| 121 | m | [10](10-protocol-machine.md) | "Rejected commit never reaches network" test never checks the network | "never reaches the network" is a named outcome the lane asserts |
| 122 | m | [10](10-protocol-machine.md) | Multiprocess recovery test can't guarantee a pending existed | pending state is constructible on demand; the kill is scripted |
| 123 | m | [10](10-protocol-machine.md) | Parity golden defaults a missing `writer` to `0n` via `BigInt("")` | lane asserts a present field; no silent default |
| 124 | m | [20](20-store-contract.md) | TS `pidAlive` spins forever on a pid exceeding int32 | no pid probe; the lease has an expiry |
| 125 | m | [60](60-codec-grammar.md) | Lambda handler surfaces a malformed POST id as a runtime crash | request parsed as a grammar → domain response |
| 126 | m | [50](50-retention.md) | Checkpointer scratch dir leaks on crash, reclaimed only next cadence | scratch is a lease with expiry, swept at open |
| 127 | m | [50](50-retention.md) | TS exists-then-vanished blindly reissues the create | below-floor create refused |
| 128 | m | [40](40-checkpoint-chain.md) | Checkpoint objects orphaned by Kept/crash/gc-truncation unreachable | immutable Merkle list; orphans are addressable and collected |
| 129 | m | [40](40-checkpoint-chain.md) | `upsert` rewrites an installed checkpoint's `prev`, unlinking a middle | `prev` inside the hash; documents written once, never rewritten |
| 130 | m | [60](60-codec-grammar.md) | Sidecar format diverges: hex vs base64, strict vs lenient parse | one grammar, one canonical encoding, one parser |
| 131 | m | [30](30-pending-chain.md) | TS `readSidecar` swallows every read error as "no sidecar" | sidecar read is a sum; `Absent` is `NotFound` only |
| 132 | m | [20](20-store-contract.md) | TS checkpoint seed skips fsync and the catalog audit | seed acked only after fsync (mdb+sidecar one durable unit); catalog audited at seed ([40](40-checkpoint-chain.md)) |
| 133 | m | [10](10-protocol-machine.md) | TS replica births the manifest on the read path | role is a field on the handle; a replica refuses `ManifestMissing` |
| 134 | m | [50](50-retention.md) | Writer duty scratch never swept; `duty_busy` leaks on panic | scratch is a lease; `duty_busy` released on unwind |
| 135 | m | [10](10-protocol-machine.md) | Detached publisher discards `resolve_backlog` errors silently | the publisher result is `#[must_use]`; the writer consumes it |
| 136 | m | [10](10-protocol-machine.md) | Id-lease diverges: no OverWidth, no exhaustion, re-runs the body | one lease algebra; body runs once; `OverWidth`/`Exhausted` refusals |
| 137 | m | [60](60-codec-grammar.md) | `encodeBatch` validation gaps: no Arity, no ClosedRelation, wrong shapes | encode enforces arity, the closed-relation identity, value shapes |
| 138 | m | [60](60-codec-grammar.md) | TS `parseCheckpoint` reads u64 `writer` through JSON number | numbers parse to `bigint`; canonical form + duplicate enforcement |
| 139 | m | [30](30-pending-chain.md) | Sidecar read-arm divergence: io fault vs parse-discard conflated | sidecar read is a sum: `Absent`/`Fault`/`Corrupt`/`Read` |
| 140 | m | [10](10-protocol-machine.md) | Inherited pending publishes at open in Rust, next commit in TS | the publish happens in the shared `open` transition both drivers call |

## Roll-up by decision

| Doc | Decision | Count |
| --- | --- | --- |
| [10](10-protocol-machine.md) | the protocol is one machine | 24 |
| [20](20-store-contract.md) | the store is one contract | 45 |
| [30](30-pending-chain.md) | pending is a chain constructor | 17 |
| [40](40-checkpoint-chain.md) | the checkpoint chain is immutable, content-addressed | 7 |
| [50](50-retention.md) | the gc floor is a write-path invariant | 23 |
| [60](60-codec-grammar.md) | parse, don't validate, at the codec | 25 |
| **Total** | | **141** |

The distribution is the argument. No decision resolves fewer than seven
findings; the store contract alone resolves 45 — nearly a third — and
together with the codec grammar it accounts for half the corpus. Every
one of the ten criticals is resolved by moving a representation, not by
patching a site. That is what "141 shadows of six decisions" means in
numbers.
