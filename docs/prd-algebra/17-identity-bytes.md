# PRD 17 — Identity bytes: `bytes<N>`, the eighth type

**Depends on:** nothing in this set (schema/encoding/image layer; composes with
everything).
**Modules:** `crates/bumbledb/src/schema/` (type + validation),
`crates/bumbledb/src/encoding/`, `image/` (multi-word columns — largely
existing k-word machinery), `crates/bumbledb-macros` (grammar + host type),
`ir/validate/` (operator typing), bench oracle + querygen.
**Authority:** `10-data-model.md` (the seven types, the collision axiom, the
interning chapter), `50-storage.md` (guard width), `00-product.md` (census
law).
**Representation move — stated in the lineage's own terms.** Brooks: the
strategic breakthrough comes from redoing the representation of the data.
The dictionary is a representation tuned for one population of values —
**reuse-shaped** (names, labels: low cardinality, high reuse), where
content-addressing is compression and id-equality is the win. Digests are the
opposite population — **identity-shaped** (content hashes, external opaque
ids: maximal cardinality, near-zero reuse) — and forcing them through the
reuse representation *manufactures* the pathology the dict-GC OPEN item
guards: every distinct digest becomes a permanent dictionary entry that will
never be referenced twice. The accepted-leak axiom was written for
compression; digest churn turns it into an unbounded tax. Change the
representation and the special case stops being expressible (SPOV 3): inline
fixed-width bytes store the value *in the fact*, and the GC problem for this
population does not shrink — it ceases to exist. The engine already made this
exact choice for its own values: the `M` namespace stores 32 inline blake3
bytes, uninterned, for the same reason. One law, now uniform across engine
and schema.

## Context (decided shape)

- **The type:** `bytes<N>`, N ∈ 1..=64 bytes, a distinct structural type (the
  eighth; the census's "seven types" sentence is amended, not violated — the
  survey predates the deep-port analysis that surfaced the digest
  population). Canonical encoding: the N raw bytes themselves, identity =
  bytes (the existing law, no dictionary indirection), zero-padded to the
  word boundary in image columns (⌈N/8⌉ words per value; the pad is
  encoding, not data — a trailing-pad nonzero byte is corruption).
- **The boundary is reuse vs identity, not text vs binary.** `str` stays
  interned (names are reuse-shaped; interning remains correct and is
  compression there). Variable-width `bytes` stays interned for the same
  reason where reuse exists. `bytes<N>` is for values whose *cardinality is
  their nature*. The docs state the decision rule in one sentence: *intern
  what repeats; inline what identifies.*
- **N ≤ 64** (typed validation error above): 64 bytes = 8 words = two cache
  lines of key material, and the guard-width gate (`MAX_GUARD_WIDTH`) must
  admit FD projections carrying one. Digests in the wild are 16/20/32/64.
- **Host type:** `[u8; N]`, newtype-able (`as ContentHash`) — `Copy`,
  lifetime-free, no borrow-surface interaction (PRD 13's law untouched:
  fixed-width fields stay owned).
- **Operators:** `Eq`/`Ne` and membership sets only. **Order comparisons,
  `Min`/`Max` refused, recorded**: digests have no semantic order — the
  lexicographic order of a hash is an encoding artifact, and admitting it
  would make hash-function choice semantically visible. Not an interval
  element. Not `fresh`-generation-eligible (fresh mints u64 witnesses).
- **Execution:** multi-word equality is machinery the engine already has —
  k-word keys are the wordmap's native shape, and the ledger's own fact
  applies verbatim: **const-generic arity gets hash hoisting and gather-hash
  fusion free where runtime arity taxes 1.2–1.5×**
  (`m2max.probe.const-arity-tax`) — `bytes<N>` values enter seen-sets, group
  keys, and guards as N/8-word const-arity keys, monomorphized per width.
  Filter kernels are the existing fixed-width predicate scans widened by
  word count; no new NEON shapes.
- **What this deletes (the churn dividend):** the dict-GC OPEN item's
  content-churn trigger profile is **reassigned** — the digest population
  leaves the dictionary entirely, returning the accepted leak to its original
  compression scope. And a **contraction OPEN** is recorded: if the deep
  ports (payroll, primer) surface no surviving *variable*-width `bytes`
  sighting once digests are `bytes<N>` and large payloads are refs, variable
  `bytes` is deleted from the type roster. *Trigger:* the port schemas
  landing with zero variable-bytes fields.

## Technical direction

1. Schema: `ValueType::FixedBytes { len }` + validation (0 and >64 rejected,
   typed); fingerprint feeds the length (a width change is a new theory).
2. Encoding: raw bytes, word-padded; corruption check on nonzero pad;
   guard-key contribution = the padded words (memcmp order = byte order —
   order-*preserving* for the B-tree's purposes even though order ops are
   refused at the query surface; the guard needs sortedness, not semantics).
3. Image: ⌈N/8⌉ word columns per field (the interval two-column precedent,
   generalized); decode plan extension; distinct counter over multi-word
   values via the existing k-word map.
4. Macro: `hash: bytes<32>` grammar; `[u8; 32]` emission; `as` newtypes.
5. IR/validate: the operator roster above; membership sets carry N-byte
   elements; param binding by value (Copy, no borrow surface).
6. Oracle: SQLite fixed-length BLOB columns; naive model `[u8; N]`;
   querygen draws digests adversarially (shared prefixes, single-byte
   deltas, all-zeros, pad-boundary widths 7/8/9/63/64).

## Passing criteria

- `[test]` Round-trip, guard-key FD enforcement, and containment over a
  `bytes<32>` key: differential green.
- `[test]` `CountDistinct` and group-by over `bytes<N>`: const-arity path
  exercised, results match the naive model (widths 8, 16, 32, 64).
- `[shape]` No dictionary traffic exists for `bytes<N>` (grep: the type's
  encode/decode paths never touch `dict`); order ops and Min/Max rejected at
  validation with typed errors; N=0 and N=65 rejected.
- `[test]` Pad-corruption fixture returns the typed corruption error.
- `[shape]` The decision rule ("intern what repeats; inline what
  identifies"), the contraction OPEN, and the dict-GC reassignment are in
  the docs.
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`10-data-model.md`: the eighth type, the reuse-vs-identity rule, the
interning chapter scoped to reuse-shaped values. Architecture README: the
contraction OPEN; the dict-GC OPEN item's trigger profile amended.
`70-api.md`: grammar + host mapping. `00-product.md`: the census sentence
amended with the digest-population finding.
