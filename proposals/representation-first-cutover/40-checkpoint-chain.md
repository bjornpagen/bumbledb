# 40 — The checkpoint chain is immutable and content-addressed

> **Decision.** A checkpoint document's `prev` backlink is part of its
> hashed content, so the digest that names it names its whole spine. A
> document is written **once** with `put_create` and never rewritten. The
> manifest points at the head of an immutable Merkle list; every retained
> checkpoint is reachable by the backlink, and no publisher can rewrite
> another's `prev`.

## The current representation

The checkpoint document carries a mutable `prev` field that the publisher
**re-renders on every attempt** from whatever the manifest pointed at
when the loop last read it, and installs with an `upsert` that will
`put_swap` over non-equal bytes:

- On a same-candidate race, `publish_checkpoint` early-returns `Replaced`
  without repair, so the winner's `prev` — freshly clobbered by a loser's
  `upsert` that rewrote the document's backlink — orphans a checkpoint. A
  critical (finding [0]); the `upsert` rewrite-of-an-installed-prev is
  reported again as [129].
- `Published::Kept { incumbent }` is documented as "gc fodder," but the
  loser's `ckpt/{digest}.json` and `.mdb` are structurally
  **uncollectable**: nothing points at them, and gc only walks *reachable*
  backlinks, so they leak forever (findings [10] [17] [128]).
- The `.mdb` is uploaded with `put_create` *before* the publish decision,
  so a `Kept` outcome or a crash between upload and CAS strands a full
  compacted store copy (finding [17], and the crash-mid-checkpoint-sweep
  orphan [16]).
- The catalog claim the checkpoint carries is audited only on some paths:
  the lying-checkpoint replay audit never fires when catch-up passes
  through the floor vector (finding [32]), and TS never audits the
  catalog claim at all (finding [69]).

The root is SPOV3: `prev` is a *special case in the control flow* — "on a
CAS retry, re-render the backlink and hope the CAS proves it" — when it
should be a *fact of the representation*. A mutable field that a race can
rewrite is a mutable field a race will rewrite.

## The target representation

### 1. `prev` is inside the hash

A checkpoint document is content-addressed by the blake3 of its **full
bytes, including `prev`**. The digest therefore commits to the entire
spine: two documents with the same heads but different `prev` are
different digests, different objects, at different keys. There is no
"rewrite the prev of the installed document," because the installed
document's key *is* the hash of its prev; a different prev is a different
key. `upsert` and its `put_swap`-over-non-equal-bytes die entirely
(findings [0] [129]). A checkpoint document is written exactly once with
`put_create`; a second writer computing the same digest sees `Exists` and
that is proof of byte-identity, not a race to resolve.

### 2. The manifest points at the head of a Merkle list

The manifest's `checkpoint` field names the head digest. Because each
document immutably names its `prev`, the manifest plus the object store
*is* a singly linked immutable list: the backlink walk is deterministic,
every node is reachable from the head, and a middle node cannot be
unlinked because no one can rewrite a node (findings [16] [128]). The
checkpoint order (whose sum is greater) selects which head the manifest
CAS installs; a loser does not rewrite anything, it simply does not win
the manifest CAS, and its document — named by a digest no manifest points
at — is *known-orphan by construction*, not accidentally-orphan.

### 3. Orphans are collectable because they are addressable

A loser's or a crash's document is an object whose digest is not on the
reachable spine. Because the digest names the whole spine, gc can
recognize an unreachable checkpoint object *by walking the reachable set
once and deleting the complement* — the "gc fodder" claim becomes true
(findings [10] [17] [128]). The `.mdb` is uploaded under the same
content-addressed digest and collected with its `.json` as one unit; the
upload-before-decision strand (finding [17]) is swept because the object
is addressable and its reachability is decidable. (The *sweep mechanics*
— resumable, floor-guarded — are [50](50-retention.md).)

### 4. The catalog claim is audited by construction

The checkpoint carries the catalog digest of the store bytes it names;
seeding from a checkpoint recomputes `catalog_digest()` and compares —
one audit, on the one seed path, in both drivers, because seeding is one
transition ([10](10-protocol-machine.md)). A checkpoint whose bytes
disagree with its catalog claim is refused `CatalogMismatch` naming the
publisher, whether reached at seed or through the floor (findings [32]
[69]).

## The invariant

> **A checkpoint document is immutable and named by a digest that
> includes its backlink, so its spine cannot be rewritten and its
> reachability is decidable.** The manifest is a pointer to the head of a
> Merkle list; losers and crashes leave known-orphan, collectable
> objects, never live objects with clobbered links.

Dissolves: [0] [10] [16] [17] [32] [69] [128] [129]. The sweep that
collects the orphans and its floor discipline is [50](50-retention.md);
the exact-number and canonical-parse guarantees for the document are
[60](60-codec-grammar.md); the catalog audit runs inside the seed
transition of [10](10-protocol-machine.md).
