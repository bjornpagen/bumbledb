# 00 — Thesis: fix the representation, delete the bugs

## The finding

A bug bash against `bumbledb-log` returned **141 defects**: 10 critical,
64 major, 67 minor. This proposal does not triage them. It reads them as
one signal and asks the only question that scales: *what representation,
had it been chosen, would have made these states impossible to write
down?*

The answer is the same for almost all 141. They are not 141 independent
mistakes. They are the shadows of **six representational decisions** that
were made the easy way, and each wrong decision casts dozens of shadows.
Patch the shadows and you get 141 patches and a 142nd bug next week.
Move the decision and whole families of bugs stop being expressible.

This is a proposal to move the six decisions. Zero backwards compat. Hard
cutover. One version number on the far side.

## The doctrine

> The biggest lever a programmer has is the **data representation**, not
> the control flow. Brooks: *"Show me your flowcharts and conceal your
> tables, and I shall continue to be mystified. Show me your tables, and
> I won't usually need your flowcharts; they'll be obvious."* Pike:
> *"Data dominates. If you've chosen the right data structures and
> organized things well, the algorithms will almost always be
> self-evident."* Torvalds: *"I will, in fact, claim that the difference
> between a bad programmer and a good one is whether he considers his code
> or his data structures more important. Bad programmers worry about the
> code. Good programmers worry about the data structures and their
> relationships."*

Three points of view follow, and they are the whole method of this
proposal:

- **SPOV1 — the representation determines the complexity.** Every one of
  the 141 lives at a place where the code carries a distinction the type
  does not. The generation is `chain.sum() + applied_pending` because
  `pending` is a flag beside the chain instead of a constructor of it.
  The lock breaks a live owner because liveness is a `bool` instead of a
  three-case sum with an `Unknown` arm that refuses to break. Fix the
  type; the arithmetic and the arm vanish.

- **SPOV2 — most branches guard states a precise representation makes
  impossible.** `refresh_braid` forgets to resolve pending (finding
  [5]); `checkpointer` forgets to guard against an applied pending
  (findings [1] [59] [72]); `waitFor` forgets the wholeness check
  (finding [41]). Three forgettings of the same thing, because the thing
  is a manual step and not a type the compiler forces every reader to
  destructure. Make the chain a sum and *there is no code path that can
  read it without deciding the pending arm* — the branch is not guarded,
  it is unwritable.

- **SPOV3 — the special case belongs to the representation.** The
  checkpoint `prev` backlink is rewritten by a losing publisher because
  it is a mutable field re-rendered on every attempt (findings [0] [10]
  [17] [128] [129]). Make `prev` part of the hashed content and the
  document is written once, addressed by a digest that *includes its
  spine*; there is no second write to race, no clobber to special-case.

The named laws we build to:

- **Make illegal states unrepresentable** (Minsky sum types). A state you
  cannot construct is a state you cannot ship broken.
- **Parse, don't validate** (King). Push every check to the boundary and
  return a *narrower type*, so the interior never re-checks and never
  disagrees with itself.
- **Null is the billion-dollar mistake** (Hoare). `Option`/`null` at a
  seam is a branch every consumer must remember; a total sum removes the
  forgetting.
- **Replace conditional with polymorphism** (Fowler). A verdict is a
  value with arms, not a boolean a caller re-interprets.
- **Half-open intervals** (Dijkstra) and **sentinel structure** (CLRS).
  The empty and the boundary case fall out of the representation instead
  of being enumerated (findings [37] [49] [63]).
- **Reify control flow as data** (SICP; Greenspun's tenth rule). A spec
  hand-implemented twice is an interpreter written twice. Write the
  interpreter once; let both drivers *run* it.

## The meta-cause: one prose spec, implemented twice

The single largest theme across the corpus — by a wide margin — is
**Rust/TS divergence**: two conforming implementations of one protocol
that take *opposite arms on identical bytes*. The pid-lockfile that Rust
breaks and TS honors (findings [2] [4] [54] [65] [73]). The pending
bytes Rust writes as hex and TS writes as base64, so migrating a crashed
directory across drivers silently destroys a durable commit (findings
[9] [107] [130]). The fixed-interval ceiling Rust refuses and TS accepts
(findings [37] [49] [63]). The 409 Rust reads as `Exists` and TS reads as
infrastructure (findings [25] [28] [62] [66]). The wholeness check Rust
runs every pass and TS runs never (findings [42] [68]). Roughly half the
141 are of this shape or its near cousin, *"TS omits an arm Rust has."*

This is not a discipline failure to be fixed with more review. It is
**Greenspin's rule fired twice**: the protocol exists as English prose in
the numbered PRD set, and English prose was hand-compiled into Rust once
and TypeScript once. Every clause is a chance to diverge, and 141
findings say the chances were taken.

The representational cure is not "audit the two harder." It is: **the
protocol stops being prose and becomes an artifact.** Three artifacts,
one per seam, each with exactly one implementation both drivers execute:

| Seam | Today (prose, twice) | Cutover (one artifact) | Doc |
| --- | --- | --- | --- |
| The **format** — batch wire bytes, manifest/checkpoint/sidecar documents | Two hand-rolled parsers per document, diverging on numbers, encodings, and lenience | One grammar; one codec; parse-don't-validate returning narrow types | [60](60-codec-grammar.md) |
| The **store** — five verbs, outcomes, durability, keys, the lock | Two verb suites diverging on liveness, 409, durability, key grammar | One capability contract: total-sum outcomes, success = durable + visible, keys a grammar, the lock a CAS lease | [20](20-store-contract.md) |
| The **behavior** — replica refresh/apply, writer publish, checkpointer, leases | Two state machines diverging on which arms exist and when they fire | One transition table, arms as states, executed by both drivers | [10](10-protocol-machine.md) |

Conformance stops being "two suites we hope agree" and becomes "execute
the one artifact and assert its named outcome" — which is why the test
defects in the corpus (any-error-passes [118], never-checks-the-network
[121], can't-guarantee-a-pending [122]) are not fixed test-by-test but
**dissolved**: there is one normative outcome to assert against.

## The six decisions

Each has its own document. Each states the current representation, the
target representation as a concrete delta against real code, and the
invariant that makes its bug family unrepresentable.

1. **[10 — The protocol is one machine.](10-protocol-machine.md)** The
   replica/writer/checkpointer behavior is one transition table both
   drivers run; arms (per-braid wedge, gap→reseed, the retry law, the
   lease algebra, the deposition signal) are *states*, not code paths one
   driver forgot. Dissolves the behavioral-drift family and the
   duplicated-transition bugs (`waitFor` vs `refresh`, [41]).

2. **[20 — The store is one contract.](20-store-contract.md)** Five
   verbs; outcomes total sums including `Ambiguous → verify`; a verb's
   success *means* durable-and-visible; keys are a grammar disjoint from
   temp/lock names; the mutation lock is a fenced CAS lease, never
   probe-then-unlink; a replica directory has cross-process exclusivity
   and a refcounted handle. Dissolves the liveness/lock, durability,
   temp-litter, key-grammar, S3-grammar, credential, and handle-lifecycle
   families.

3. **[30 — Pending is a chain constructor.](30-pending-chain.md)** The
   chain is `Settled(vector) | Pending(vector, batch)`, a sum every
   reader destructures. `applied_pending: 0|1` and `pending: Option` die.
   The wholeness identity becomes *"the generation the chain says,"* by
   construction, checked the same way on every path. Dissolves the
   forgotten-pending family.

4. **[40 — The checkpoint chain is immutable and content-addressed.](40-checkpoint-chain.md)**
   `prev` is part of the hashed content; a checkpoint document is written
   once with `put_create` and never rewritten. The manifest points at the
   head of an immutable Merkle list; every retained checkpoint is
   reachable and no publisher can rewrite another's spine. Dissolves the
   prev-clobber and orphan-checkpoint families.

5. **[50 — The gc floor is a write-path invariant.](50-retention.md)**
   The published checkpoint vector is the one floor every writer,
   checkpointer, and sweep consults *before creating or deleting a slot*;
   the sweep is a resumable bottom segment; retention ages by a trusted
   publish clock, not a writer-claimed timestamp; every scratch dir,
   temp, and thread has an owner that reclaims it. Dissolves the
   slot-resurrection, stranded-sweep, and lifecycle-leak families.

6. **[60 — Parse, don't validate, at the codec.](60-codec-grammar.md)**
   One grammar; numbers are exact `u64`/`i64`, never JavaScript
   `number`; a row vector cannot claim more rows than its bytes back; a
   string cell is well-formed by construction; a 32-byte digest is
   `[u8; 32]`, wrong lengths unrepresentable; a fixed interval is
   half-open, so the domain ceiling is not a value. Dissolves the
   panic-on-hostile-bytes, precision-loss, and encoding-divergence
   families.

Then **[70 — The cutover.](70-cutover.md)** lists what is deleted and how
the fleet flips with no compatibility window, and
**[90 — Traceability.](90-traceability.md)** maps every one of the 141
findings to the decision that dissolves it.

## What this is not

Not a CRDT and not a rewrite of the product. The braids, the five
deployment cases, the L9/L10 laws, the "recovery is replay" thesis of the
existing PRD set all stand — this proposal changes *how the protocol is
represented in code*, not what it promises. It is normative in the same
sense the numbered docs are: the build implements these representations
or reports the gap; it does not improvise a seventh way to spell a
pending batch.
