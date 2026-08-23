# 00 — The operating law

This pass has one identity: **there is a single way to do everything, and
everything without a consumer dies.** Two instruments enforce it.

## The algorithm, in order, at every defect

1. **QUESTION the requirement.** Every requirement names its owner — a
   numbered doc, a named consumer, or the audit finding. The most dangerous
   requirements are the smart-sounding ones; this pass killed its own
   centerpiece (the conflict-algebra fast path) by questioning it.
2. **DELETE the part.** The best part is no part; the best branch is no
   branch; the best knob is no knob. If you are not slightly nervous you
   deleted too much, you didn't delete enough. Deletion count is the pass's
   headline metric.
3. **SIMPLIFY what survives.**
4. **ACCELERATE only after 1–3.** (This pass accelerates nothing: F11's own
   measurements showed the "fast" path slower than the general one. Speed
   claims without attribution are deleted as claims.)
5. **AUTOMATE the gate** — census lane, conformance fixture — so the defect
   class cannot return silently.

## The representation doctrine

- The data representation determines the program's complexity; control flow
  is downstream. At every fix the first question is never "is this branch
  right?" but **"what representation makes this state unrepresentable?"** A
  fix that adds a guard where a representation change would delete the case
  is a rejected fix.
- Illegal states unrepresentable: flag-sets become sums; a knob that can
  only express one honest value becomes no knob; two spellings of one fact
  become one spelling.
- Parse, don't validate: every wire boundary returns a refined value
  carrying its proof, or a typed refusal — never a checked-and-forgotten
  boolean. The codebase lives this; extend it, never dilute it.
- One coordinate system per question: two components speaking two dialects
  of one protocol is data denormalization — unify the representation, never
  bridge it. (30-store-protocol.md is this rule applied.)
- The limit is honest: representation removes accidental complexity, not
  essential. Where a case is genuinely essential — the ack-mode sum, the
  commit/commit_split verb split — implement it plainly and record why it
  is essential where it lives.

## The razor, as binding here as it was in the purge

A part dies iff it is (a) expensive regardless of usefulness, or (b)
useless regardless of cost. Cheap AND useful lives, even with no consumer
today. **Banned reasonings, applied to this pass's own material:**

- *Harness-as-consumer.* A test is not a consumer of the feature it tests.
  The footprint section's last surviving role was feeding the oracle that
  checked the footprint section — circular, banned, dead (10).
- *Thesis-as-consumer.* A doc's claim is not a consumer either. Where the
  product's stated thesis names a part nothing spends, the thesis is
  rewritten to what the system actually is (50). The braids carry the
  concurrency story; they always did.
- *Usefulness-punished-for-being-early.* Also banned in the other
  direction: nothing here dies merely for being unused this week. Things
  die because their one consumer was deleted (the W arithmetic's consumer
  was the fast path), because they are a second spelling (the TS etag
  dialect), or because they cannot express their advertised range (the
  max_pending knobs).

## Two standing owner laws this pass restores

- **Unbounded repair, legible scream.** A repair loop never caps and
  hard-errors; it repairs forever and screams legibly (a warning every Nth
  attempt, a recurrence alarm on a repeating signature). Repair bounds and
  their bookkeeping are deletion targets (40).
- **Never weaken a test to green it.** A red test is a real bug (fix the
  code) or a wrong requirement (amend the doc AND delete the test, with the
  reasoning recorded in the commit). This pass deletes many tests — every
  one because its requirement died, never because it was red.

## The consumer this pass serves

The moment this pass completes, work cuts over to primer-spec's parallel
scope loops: insert-only, content-keyed, no capacities, one braid, FsStore,
TS driver, document-per-minutes rates. When in doubt about where an hour
goes, it goes to what that consumer touches. The S3 tier keeps its recorded
reopen triggers; it buys no complexity today.
