# 30 — Pin the dark

> **Decision.** Every protocol surface gets a golden, or it is not a
> protocol surface. The audit proved the corpus's coverage boundary is
> exactly the drift boundary: all five live divergences sit on surfaces
> with no byte pin. Each dark surface gets one ruled spelling and a
> corpus golden the battery carries forever; the five live bugs are
> fixed **against** those ruled spellings, and the pin lands in the
> same commit as the fix so the class dies with the instance.

## The current representation

audit/20's pin inventory: byte-exact goldens exist for the batch codec
(31 cases), braids (4), chain (5), documents (23), and fuzz (33 + the
storm recipe). **No pin exists** for: the fs lease body, the id-lease
counter body, the ckpt-scratch body, the key grammar's accept/refuse
sets, the Vector wire form, or either state machine's constants. The
live findings map onto the dark set one for one:

1. **The fs lock protocols are mutually unreadable.** Rust:
   `LEASE/1\n…` four-line bodies under `~lease/{key}/`, 5 s TTL
   (`store.rs:215`, `store/fence.rs:291,15`). TS: three-line dotfile
   leases *beside the object*, 30 s TTL (`store.ts:126-145,28-38,111`)
   — under a header claiming "the one on-disk protocol, shared with the
   Rust driver." A Rust writer and a TS writer on one `FsStore` prefix
   do not see each other's locks; the interop lane and the Lambda
   example both put the two drivers on shared prefixes (audit/80 B).
2. **`waitFor` on a wedged braid polls forever in TS** — the
   `Waited::Wedged` arm is unsurfaced (audit/40; the arm lands via
   [10 §4](10-one-vocabulary.md)).
3. **`ErrExhausted` spells a cache miss in TS** and true exhaustion in
   Rust (`writer.ts:205-209`) — fixed by [10 §4](10-one-vocabulary.md)'s
   refill arm.
4. **The tilde-lookalike refusal sets differ**: 15 code points + NFKC
   on one side, 10 on the other (audit/50).
5. **`WAIT_FOR_POLL_MS` is 10 vs 20** (`replica.rs:42` vs
   `replica.ts:50`) — trivial, and exactly the kind of constant a pin
   catches for free.

Plus the corpse: `writeCanonicalLiteral` (`value.ts:278-323`) mirrors
the engine crate's `encode_literal` and has been dead since
`internalDescriptor` landed.

## The target representation

### 1. One spelling per surface, ruled here

- **The fs lease body**: the Rust spelling is the protocol —
  `LEASE/1` versioned body, `~lease/{key}/` placement inside the
  reserved namespace (a lock beside the object is a name outside the
  key grammar's reserved partition, which is the bug), and the 5 s TTL
  as the one constant. TS conforms. After [20](20-one-reader.md) the
  body's parse/render is the shared core anyway; this ruling fixes the
  *placement and TTL* facts that live in the TS store's IO half.
- **The id-lease counter body and the ckpt-scratch body**: already
  single-spelled post-cutover; they get goldens, not changes.
- **The key grammar**: one accepted set, one refused set, as corpus
  fixtures — including the tilde-lookalike table, which becomes **one
  generated table** both drivers consume (the Rust set is the writer;
  the TS set is emitted from it), so a refusal set cannot fork again.
- **Machine constants**: `WAIT_FOR_POLL_MS` and every
  tunable both steppers share moves into one constants table in the
  conformance inventory, asserted by both suites. The machines stay
  two executors; their *shared numbers* become one fact.
- **The Vector wire form**: [20](20-one-reader.md) deletes the TS
  encoder (grade-D, uncalled); the Rust form gets its golden so the
  next caller inherits a pinned spelling.

### 2. The pin law

A new census line enforces the class forever: **every file under
`conformance/v3/` names its surface, and every surface named in the
inventory manifest has at least one golden** — the manifest of
surfaces is itself data, so "we forgot to pin the lease body" becomes a
red gate, not a five-day-old discovery. Adding a protocol surface
without a pin is an incomplete addition, the same law as adding a
deletion without a roster line.

### 3. The bug fixes ride the pins

Each live bug's fix commits **with** its golden: the lease unification
with the lease-body and placement goldens; the tilde unification with
the generated-table fixture; the poll constant with the constants
table. The fix without the pin reverts the class to hope; the two land
as one representation change.

## What gets deleted

| Deleted | Because |
| --- | --- |
| the TS dotfile lease spelling, placement, and 30 s TTL | one lock protocol, the reserved-namespace one |
| the TS tilde set as hand-written code | generated from the one table |
| the drifted poll constant | the constants table is the writer |
| `writeCanonicalLiteral` | dead since the seal crossed the FFI |
| the "shared with the Rust driver" comment-as-hope | replaced by goldens that make it true |

## The invariant

> **Dark surfaces do not exist.** Every byte the protocol writes has a
> golden both drivers walk; every constant both machines share is one
> fact in one table; and a surface can be added to the protocol only by
> adding its pin — the inventory's completeness is itself a gate, so
> the coverage boundary and the protocol boundary are the same line.

Dissolves: audit/20's unpinned-surface list and live lease/constant
drift; audit/40's live bugs 2–3 (jointly with [10](10-one-vocabulary.md));
audit/50's tilde and dead-code finds. The oracle argument of audit/80 C
("drift risk is already ~zero") becomes true *after* this document — it
was not true before it.
