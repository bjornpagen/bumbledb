# 00 — Thesis: retire the second reader

## The finding

Eight investigators audited the engine/log seam (`audit/10`–`80`). The
raw numbers: `ts-log/src` is **6,482 lines re-implementing material
covered by ~10,685 lines of `crates/bumbledb-log/src`**, pair for pair —
codec, braids, Vector, documents, chain, key grammar, lease bodies, two
state machines (audit/20). The engine, meanwhile, already runs the
architecture this duplication is pretending to be: one core crate
exporting a boundary vocabulary of pure-data IR structs, tagged values,
opaque handles, and a closed error taxonomy, carried to three hosts by
deliberately dumb bridges, with drift locks that **break the compile**
when the core grows an arm (`wire_tags!`, the three-way `tags.json`
golden, the fingerprint pin — audit/10).

And the mirrored pair is not holding. The audit caught live divergence
on every surface the conformance corpus never pinned:

- The two drivers' **fs lock files are mutually unreadable** — Rust
  writes `LEASE/1` bodies under `~lease/{key}/` with a 5-second TTL; TS
  writes 3-line dotfiles beside the object with a 30-second TTL — under
  a TS header that claims "the one on-disk protocol, shared with the
  Rust driver" (audit/20).
- **ts-log's `waitFor` on a wedged braid polls forever**; Rust returns
  `Waited::Wedged` (audit/40).
- **`ErrExhausted` means two things**: a cache miss in TS, true
  exhaustion in Rust — one identity, two semantics (audit/40).
- The tilde-lookalike refusal sets differ (15+NFKC vs 10 code points);
  `WAIT_FOR_POLL_MS` is 10 in one driver and 20 in the other (audit/50,
  audit/20).

Every one of these lived in the dark: the corpus pins the codec
byte-exactly and never pinned the lease body, the counter, the scratch,
the key grammar, the Vector wire form, or either machine (audit/20).

## The diagnosis

This is Greenspun's rule, verbatim. The protocol grammar is one language
with one specification, and `ts-log` grew a second, informally-locked
interpreter of it by hand — not because anyone chose a second
interpreter, but because each module was mirrored one at a time and the
sum was never named. The doctrine names every part of the cure:

- **SPOV 1** — the leverage is the shape of the data. The engine's
  bridge works because the *boundary is a vocabulary* (typed payloads,
  tagged values, opaque handles), not a pile of per-function
  conventions. The log gets the same boundary.
- **SPOV 2 / Minsky** — two structurally identical value unions, two
  interval types, a `reserve` that returns four different shapes across
  four surfaces (audit/40): each is a state space wider than the facts
  it represents, and every widening is guarded somewhere downstream.
  One vocabulary, and the guards have nothing to guard.
- **King** — the corpus-as-transcript failure mode again: surfaces
  checked once by hand and never carried as a pin. Parsing keeps proof;
  [30](30-pin-the-dark.md) turns every dark surface into a golden the
  battery carries forever.
- **SICP / the ceiling** — the batch is already the AST and `apply` is
  already the evaluator. The only question this campaign answers is
  *how many implementations of the evaluator's grammar exist*, and the
  answer becomes one.
- **Brooks' limit, respected** — the async machines, the three stores,
  the tenants LRU, and everything that touches a Promise, an fd, a
  clock, or a process identity is **essential** per-language surface
  and stays TS (audit/70 drew the line precisely; audit/80's async
  objection is honored as a boundary, not fought).

## The four decisions

1. **[10 — One vocabulary.](10-one-vocabulary.md)** `ts-log` uses the
   engine SDK's types **themselves** — the same declarations, imported,
   never aliased: `FactValue`/`IntervalValue` at every value site, the
   sealed descriptor types imported not restated, `Batch` a structural
   subtype of `WriteTx`, `Commit` composing `Admission`, one exhaustion
   identity, one `FreshRange`, one name per coordinate. The Rust driver
   already lives this law (`use bumbledb::{Value, ...}`); the TS driver
   joins it. ~90% is TS-to-TS with two small engine exports (audit/30).
2. **[20 — One reader.](20-one-reader.md)** The sealed byte grammar —
   codec, braids, documents, sidecar/scratch parse+render — moves
   behind `ts/crate` as a `LogCodec` handle, exactly the query-builder
   recipe (audit/10), with its two named blockers paid: the
   `bumbledb-log` feature split so the cdylib stays lean, and the error
   mint-table so refusal identities survive the FFI. The machines,
   stores, Vector hot math, and key assembly stay TS. Roughly 2,900
   lines die (audit/50).
3. **[30 — Pin the dark.](30-pin-the-dark.md)** Every surface the
   corpus never covered gets a golden — lease body, counter body,
   scratch body, key grammar, machine constants — and the five live
   drift bugs are fixed against the ruled spellings, not around them.
4. **[40 — The oracle.](40-the-oracle.md)** The one honest cost of
   decision 2 — losing the second independent reader of the grammar —
   is taken as a ruling with a replacement: goldens generated
   independently of the shared core, the engine bridge's
   compile-breaking drift locks applied to the log vocabulary, and the
   fuzz storm as the standing hostile reader.

Then **[50](50-deferred-with-triggers.md)** writes down what this
campaign refuses (with reopen triggers), and
**[90](90-traceability.md)** maps every audit finding to its dissolving
decision.

## What this is not

Not a rewrite of the machines — the replica and writer steppers remain
two per-language executors of one transition law, because their inputs
are Promises and fds, which is essential complexity (Insight 16). Not a
C ABI for the log — deferred with its trigger written (audit/60). And
not a walk-back of anything the cutover, settlement, or lockstep
landed: the binary v:3 grammar, the one battery, the one version, and
the census roster all stand; this campaign is what they make cheap.
