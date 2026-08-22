# The impossible-states ledger

The design measured the only way that matters: by the bad states it
cannot represent. Part I is every state the prior-art systems must
represent — and therefore carry machinery to detect, repair, fence, or
apologize for — that is unrepresentable here, with the representation
feature that kills it. Part II is the same blade turned on ourselves:
states *our own PRD set* represented before the deep read, now deleted.
Every claim is grounded in the papers in this directory (page cites in
THESIS.md) or in the named proposal.

House law, restated once: a guard, a retry loop, a repair procedure, an
epoch, or an apology is always downstream of a representation that
admits a state it shouldn't. The fix is never a better guard.

## Part I — their states, our non-states

### SQLite-lineage WALs (Turso diskless included)

| Representable state of theirs | Their machinery | Why it cannot exist here |
| --- | --- | --- |
| A replayed frame corrupting the store (page images are order-fragile, non-idempotent) | Frame salts, checksums, applied-watermarks | Replay is idempotent by theorem (L10): re-application net-disposes to the engine's no-op arm. Recovery *is* replay (00 law 8) |
| Torn mid-checkpoint WAL↔db state | Checkpoint modes, busy-handler dances | Checkpoints are immutable content-addressed objects beside the log, never a mutation of it (10) |
| Per-DB serial writes, full stop | None — the single writer is the design | Per-braid parallel writers with proven commutation (L8/L9); the serial log survives only as the one-braid degenerate case |
| Restore points on one line | PITR to a frame | Restore points form a lattice: *any* vector is a legal serial state (L9) — per-braid PITR to every pointwise combination (10) |

### Aurora

| Their state | Their machinery | Our kill |
| --- | --- | --- |
| Holes in segment logs (4/6 quorum delivery) | Backlink gossip, SCL bookkeeping, permanent background repair fleet | One `Created` is durability; the store's internal replication is someone else's quorum. Chain slots fill contiguously — a writer can only create g+1 having applied g, and `prev` proves it |
| Complete-but-not-durable log band (VDL < LSN ≤ VCL) | CPL tagging, VDL computation, truncation ranges **versioned with epochs** so re-crashed recovery agrees with itself | Ack = the created object. Nothing above the tip exists; there is nothing to annul, so there is no truncation, so there are no truncation epochs |
| Unbounded engine-ahead-of-storage gap | LAL backpressure constant (10M LSNs) | Kept — as one knob (`max_pending`) on the one mode (`ack = local`) that reintroduces the gap, visible in the outcome type as `durability` (60) |
| In-flight transactions needing undo after crash recovery | Online undo phase | Rejections never touch the network; the log contains only accepted state-changing commits (the publish law, 10) |

### Delta Lake

| Their state | Their machinery | Our kill |
| --- | --- | --- |
| Two writers creating the same log record on S3 | A separate coordination service in Databricks deployments; a per-vendor LogStore plugin API; single-Spark-driver mode in OSS | `If-None-Match: *`. The 2020 coordination service is a 2024 header (40) |
| Visible log holes under eventually-consistent LIST | Readers wait out missing IDs below the largest listed | No LIST exists in the protocol; tips are walked (`GET k+1`), and strong read-after-write is a verified vendor precondition |
| Stale `_last_checkpoint`, checkpoint discovery by LIST-forward search | Designed-legal staleness + search | The manifest is CAS-current and checkpoints backlink (`prev` digest chain) — discovery is GETs alone (10) |
| Table state as file-membership metadata | `add`/`remove` tombstones retained across checkpoints on a wall-clock treaty with stale readers | State is the fold of set-semantic deltas; there is no file-membership ledger to keep consistent, and the tip-vs-hole rule is decided from the manifest checkpoint vector, not a clock (50) |
| Commit rate: "several transactions per second" per table, self-admitted | A hoped-for future low-latency LogStore | Braids × group commit × Express One Zone; 80's pins quote their ceiling as the baseline to embarrass |

### Calvin / Aria (the deterministic school)

| Their state | Their machinery | Our kill |
| --- | --- | --- |
| A transaction whose read/write set is unknown at sequencing | OLLP: reconnaissance query + recheck + deterministic restart loop, statistically bounded | Footprints are computed *after* the body runs, over recorded ops — post-execution certificates have no prediction to go stale |
| Locks held for the duration of execution (the "contention footprint") | Disk-latency prediction, prefetch delays, epoch tuning | Zero-duration coupling: writers meet only at an immutable CAS slot; conflict is computed over published bytes |
| One straggler stalling a 1,000-transaction epoch (−81 %) | Batch-size hand-tuning per benchmark and cluster | No epoch exists; the commit unit is one braid's drain, and braids proceed independently |
| Same-key writes aborting even when byte-identical; any two increments of one counter = WAW abort | Deterministic reordering (NP-complete optimal, greedy shipped), fallback mode switching on a moving-average heuristic | `fid = blake3(full row)` makes identical writes the F-commute cell; W-slack arithmetic composes concurrent deltas Aria must serialize — their own worst TPC-C pain point (`d_ytd`) is our showcase |
| Async-replication failover ambiguity ("which batch was the last valid batch… exactly what transactions that batch contained") | Reconstruction protocol over partial views | Create-only immutable slots + forward probing: the tip is discovered, never negotiated |

### RedBlue / Indigo / Homeostasis (the reservation school)

| Their state | Their machinery | Our kill |
| --- | --- | --- |
| A mislabeled blue operation (labels are static promises; nothing checks at runtime) | Silent divergence or invariant breach | Footprints are carried **and recomputed** by every replica and every loser; a wrong claim is a typed corruption error (`FootprintMismatch`), never a silent state |
| 98 % of purchases queueing on one global red token rotating at ~1 s/site — for items that never interact | "Additional work is needed to identify an optimal strategy" (their words) | Per-pair conflict at raw-value keys; the arbitration unit is the braid slot; no global order object exists |
| Leaked / stranded / double-granted rights; revocation fan-out that "can fail when a right is being used"; unfenced reclamation from dead holders | Exponential-backoff rights juggling, exactly-once fenced DC recovery, primary-healed leaks | Reservations are rows (15/60): conservation is the capacity judgment, spend commutes by the W interval arithmetic while headroom holds, reclaim-vs-spend goes CONFLICT near the bound and re-judges honestly, expiry is an event in the arbitration domain. There are no rights objects to leak |
| One treaty violation stalling every site (state broadcast, voting, losers wait, 2PC renegotiation) | The Homeostasis cleanup round | Conflict is pairwise and loser-local; nobody else observes a re-judgment |
| Invariants enforceable only if annotations are honest | Trusted programmer postconditions fed to Z3 | The ops *are* the postconditions — typed inserts/deletes of raw values; there is nothing to annotate |

### CRDTs (the convergence school)

| Their state | Their machinery | Our kill |
| --- | --- | --- |
| States no execution produced (add-wins mixtures with no sequential explanation; multi-value registers) | "-wins" policies, vertex-cut arbitration algorithms, per-type semantic zoos (three set semantics, three map semantics…) | Every observable state is a serial prefix (L8 + `catalog_digest`); there is no merge, so there is no policy to choose and no zoo to curate |
| Invariant-violating replica states (the survey's own verdict: CRDTs "are unable to enforce such global invariants") | Escrow bolt-ons, causal-delivery preconditions, apology flows | Only judged batches exist in the log; every reachable vector state passed the full theory — the reads law (00 law 9) is a sentence no CRDT paper can write |
| Arbitration regression (cast-off updates resurfacing; the shipped register bug from premature metadata GC) | Retain-until-safe rules over causal frontiers | Arbitration is slot ownership, decided once at CAS time, recorded as which object exists — never recomputed from timestamps |
| Tombstone metadata dwarfing data; "not created" vs "deleted" indistinguishable | Vector-clock summaries, tombstone GC treaties | Ids ride concretely in ordered commands; absence is decidable against a prefix; deletes are ops in batches, not graveyard state |

### Data exchange (the theory frame)

| Their state | Their machinery | Our kill |
| --- | --- | --- |
| Labeled nulls in instances | Null-aware homomorphisms, cores, the ↓ operator, certain-answer coNP walls, FO-inexpressibility theorems | `Var(store) = ∅`: writers discharge every existential at the boundary under an id lease (skolemization of the *write*); the wire has no null encoding. The entire apparatus is about a set we keep empty |
| Many solutions per source; query answers quantified over all of them | Universal solutions, certain-answer evaluation *by chase* | Every state is the unique model of its own history; naive evaluation is sound for all queries (their Prop 4.2, satisfied trivially) |
| Non-terminating and order-divergent chases | Weak-acyclicity schema tests, canonical-solution caveats | No chase step ever fires: admission is the successful *zero-step* chase, and refusal is the ground chase's ⊥, delivered pre-commit with citations |

## Part II — our own states, deleted this pass

The same lens, applied by six adversarial deep-reads of the PRD set
against the papers. Each row: the state we could represent, who caught
it, and the representation that now forbids it.

| The state we admitted | Caught by | The kill |
| --- | --- | --- |
| The serverless fork: a locally-committed batch the bucket never assigned — recovery's forced cases *completed* the fork, then the store silently diverged forever | I-confluence read; independently by the log-storage and deterministic reads | The pending slot is both modes' law: no local commit exists without its bytes in `pending` until its slot exists (60). Plus the total detector: `generation ≡ Σ vector` or discard (50) |
| Sidecar intent field + two forced recovery cases + `AlreadyApplied` — a three-way decision procedure over a crash window | The engine itself (`COMMIT_NOOP`: empty delta ⇒ no commit, no generation advance, no judgment) | Idempotent replay (L10): recovery is the catch-up loop; the decision procedure had nothing left to decide (20/50) |
| A no-op log slot (loser republishing a batch the winner fully absorbed) desyncing generation from vector-sum *permanently* | Deterministic read | The publish law — published ⟺ local apply advanced the generation — and the subsumption arm of the loser algebra (10/15/60) |
| 15 and 60 disagreeing on what a conflicting loser re-runs (recorded ops vs host body) — body re-runs could double-apply split siblings and re-mint fresh ids | Deterministic read | One semantics: re-judge recorded ops, never re-invoke the closure; the engine's own "ships the outcome, never a loop" law, one layer up (15/60) |
| 404 meaning both "tip" and "gc'd hole" — a hibernated replica serving arbitrarily stale reads as fresh, forever | CALM read and log-storage read, independently | Tip iff slot > manifest checkpoint vector (the gc exemption law already guaranteed it); manifest heartbeat bounds detection staleness by law (10/50) |
| The escrow fast path publishing a ceiling violation into the immutable log (grants invisible to judgment; TTL trust) | Reservation read (the one outright soundness bug found) | Escrow deleted as a subsystem; reservations are rows judged by the theory they reserve against (15/60) |
| `ckpt/{sum}` colliding for distinct vectors with equal sums — wedging bootstrap with no bug required | Log-storage read | Content-addressed checkpoints: `ckpt/{digest}` (10) |
| A wrong-base or out-of-sequence slot replaying as silent acceptance or mislabeled corruption | Log-storage read | The `prev` chain hash in every header; `ChainMismatch` before any apply; checkpoint `heads` seed the chain across bootstrap jumps (20/10) |
| "Naming both writers" in an error no field could construct | Log-storage read | The writer id in every batch header (20) |
| `manifest.writer: ""` — an advisory arrangement whose null was the empty string, and `floors` — a second answer to "where is the head?" with no named consumer | CALM + I-confluence + log-storage reads, unanimously | Both fields deleted; the manifest is three fields; slot arbitration is the one answer to "who writes" (10) |
| PITR by-time mapping through unordered timestamps — a non-prefix "restore point" | Deterministic + CALM reads; also caught in-house | Publish-clamped monotone timestamps, refused at apply; restored *vectors* are the reported truth (10/20) |
| Split outcomes handed to hosts that never asked ("I thought this was atomic") | I-confluence read, seconded by reservation read | `commit` refuses spanning batches; `commit_split` is the explicit verb — splitness chosen at the call site (60/70) |
| Ack durability living in a constructor secret while every `Accepted` looked alike | I-confluence read | `durability: Published | LocalPending` in the value (60/70) |
| A lying winner steering losers off the re-judgment path fleet-wide | Deterministic read | Losers recompute the winner's footprint from its ops; carried sections are never trusted on any path (15) |
| The point-Δ W test certifying commutation for a batch whose delete evaporates against the moved base — a republish-without-re-judge that could publish a ceiling violation | The final pass's own adversarial trace (reclaim ∥ spend ∥ consumer) | W deltas are op-derived bounds, never effect claims; the commute test runs on evaporation intervals whose endpoints encode exactly the verdict-flip boundary (15/20) |

| The disjoint fast path republishing a batch whose ops evaporate at the new base (one shared commute-cell F key + base-redundant remainder) — a no-op log slot and a fleet-wide infinite discard loop, from two honest writers | The final pass's protocol adversary (its one critical trace) | Disjointness means *zero shared keys of any class* — the strictness is L6/L7's hypothesis now; anything shared re-judges, and a first-applied slot that net-noops is a typed publish-law refusal naming its writer (15/20) |
| A born-rejected or born-no-op batch resurrected by a crash between pending-fsync and first judgment, published by a recovery rule with no verdict arms | Protocol adversary | Recovery is three forced arms decided by the verdict plus the wholeness instrument; rejected and born-no-op pendings clear without publishing (60) |
| A checkpoint verified only against its own name — a poisoned `.mdb` with honestly copied heads becoming unauditable truth once gc passes | Protocol adversary | The checkpoint json carries the `catalog_digest` content claim and its publisher; opens verify it, replay-reaching stores compare-and-refuse, and the gc window is named as the audit window (10) |
| A recovered writer burning its whole contention bound on losses to *history*, then dropping an acked pending on a typed error with no disposition | Protocol adversary | History never counts — catch up first, one attempt at the tip; `Err::Contention` is a cause sum (hot-key vs slot-race), and a pending surviving it stays applied, counted by the identity, and retried (15/60/50) |

Three exposures are *recorded* rather than killed, deliberately, each
with its trigger: host-retry duplicate entities under crash-before-ack
(Delta's `txn` idempotence keys are the v2 candidate; set semantics
already absorbs identical rows — only `reserve`-minted ids duplicate);
version-evolution gates in the manifest (activates the first day two
deployment versions coexist); and position-level braids (the
data-exchange dependency graph is finer than the statement graph;
trigger is a measured hot braid with disjoint projections).
