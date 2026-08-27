# 40 — The oracle

> **Decision.** The two-independent-readers witness is retired **on
> purpose, as a priced ruling** — and replaced by an oracle that is
> stronger where it matters and honest about where it is weaker. The
> corpus stops being "two hand-written readers cross-checking each
> other" and becomes: goldens generated independently of the shared
> core, compile-breaking drift locks on the boundary vocabulary, a
> standing hostile reader (the fuzz storm), and an FFI identity lane.
> This document exists because audit/80's strongest argument deserves a
> ruling on the page, not a shrug.

## The argument being ruled on

audit/80 B and audit/70 both land the same true point: with two
hand-written readers, the 191-file conformance corpus is a *cross-check
between independent minds* — it is how the bug bash's grammar criticals
(BOM row 6, interval-ceiling rows 37/49/63, hex-vs-base64 rows
9/107/130) were findable at all. Collapse to one reader and the corpus
degenerates into a single-reader regression pin: the shared core can be
wrong *consistently*, and no lane disagrees.

The counter-evidence, also on the page: the two-readers oracle only
works on surfaces it covers, and audit/20 caught **five live
divergences on the surfaces it didn't** — including a mutually
unreadable lock protocol shipping under a comment claiming unity. Two
readers bought spec-grade catches on the pinned 60% and bought nothing
on the dark 40%. The oracle was real, partial, and paid for with 6,482
lines of standing divergence risk.

## The replacement oracle, part by part

### 1. Goldens are generated independently of the reader

The corpus's authority moves from "two readers agree" to "the goldens
are produced by construction, not by the code under test":

- **Ok-goldens from the inventory as data.** The v:3 inventory already
  describes documents and batches as structured JSON metadata with hex
  dumps; the golden *bytes* are assembled by a dedicated generator that
  walks the inventory spec — a small, separate program whose only job
  is spelling the grammar from the written field rosters, kept
  deliberately independent of `bumbledb-log`'s encode paths. The shared
  core must *decode what the spec generator wrote* and *re-encode it
  byte-identically* — reader and spec disagree, the lane is red.
- **Refusal goldens are hostile by construction**: truncations, bad
  magic, trailing bytes, non-ascending rosters — generated
  mechanically from the ok-set (every prefix, every mutation class), so
  the refusal surface is enumerated rather than curated.

### 2. The fuzz storm is the standing hostile reader

The mutation storm (already a lane) runs against the one core with the
severity budget raised: it is now the *only* adversarial reader, so it
inherits the two-readers oracle's job of finding what no one wrote
down. Its recipes are pinned in the inventory; its accepted mutants
must be canonical fixpoints (decode∘encode = identity), which is a
property one reader can be held to *by itself* — no second mind needed.

### 3. The engine's drift locks, applied to the log vocabulary

From audit/10, the locks that made the query bridge safe for years:

- **`wire_tags!`-style exhaustive tables** for every log boundary enum
  (refusal kinds, outcome arms, document tags): a new arm in the core
  that is missing from the table breaks the *compile*, in both the
  bridge and — via the generated table below — the TS surface.
- **The generated identity table**: `DecodeError::identity` strings,
  refusal kinds, outcome arm names, and the shared machine constants
  ([30](30-pin-the-dark.md)) are emitted from the Rust core into one
  checked-in artifact that TS imports and the census diffs against a
  fresh regeneration — the `tags.json` three-way lock, generalized. A
  tail kind added unilaterally (which already happened, audit/40 §4)
  becomes a build failure.
- **Payload keys join the goldens** — the engine bridge's one unlocked
  axis (audit/10's wart list), locked here from day one.

### 4. The FFI identity lane (blocker B2's proof)

A thin conformance lane walks every row of the identity table through
the actual bridge: force each refusal in the core, catch it in TS,
assert the sentinel identity and cause shape match the table row. This
pins the mint-table (the one new place identity could be lost) with the
same one-row-one-assertion discipline as everything else.

### 5. What remains two-readers, on purpose

The **machines** keep two executors (essential, per
[20 §3](20-one-reader.md)) — so the crash matrices and the step-table
conformance stay genuinely cross-checked between two independent
implementations of the transition law. The **store contract** keeps the
cross-process interop lane racing both fs stores. The two-minds oracle
is retired only where the second mind is being deleted; where two minds
remain, their cross-check remains.

## The honest ledger

What is lost: a wrong-but-consistent shared core can no longer be
caught by a disagreeing sibling on grammar surfaces. What is gained:
the dark-surface class is gone ([30](30-pin-the-dark.md)), the
spec-generator disagreement lane is a *third* mind that never existed
before (spec vs implementation, rather than implementation vs
implementation), and every historical grammar catch the two readers
made (BOM, ceiling, encodings) is now a pinned fixture that the one
reader is regression-locked against forever. The trade is taken with
eyes open; this document is the receipt.

## The invariant

> **The grammar's truth does not live in any reader.** It lives in the
> inventory-as-data and the generator that spells it; readers —
> however many exist — are checked against it. A reader agreeing with
> itself proves nothing and is asked to prove nothing; a reader
> disagreeing with the spelled spec is red the hour it happens.

Dissolves: audit/80 B (by replacement, priced), audit/70's epistemic
cost (by the spec-generator third mind), audit/40 §4's generated-table
item, and audit/10's remaining warts (payload keys, comment-enforced
tripwires).
