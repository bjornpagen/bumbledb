# PRD 17 — Identity bytes: `bytes<N>` replaces `bytes` — the roster stays at seven

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

- **The cut, decided (owner-approved 2026-07-10): variable `bytes` is
  DELETED and `bytes<N>` takes its seat — the type roster stays at seven.**
  Variable-width, reuse-shaped, *binary* values — the only population
  variable `bytes` could serve once digests are inline and large payloads
  are refs — has zero sightings in either deep-port target. A type without a
  population is symmetry, not design. *Reverses if:* a real schema surfaces
  a variable-width binary value with genuine reuse; the dictionary machinery
  it would need survives intact under `str`.
- **The type:** `bytes<N>`, N ∈ 1..=64 bytes. Canonical encoding: the N raw
  bytes themselves, identity = bytes (the existing law, no dictionary
  indirection), zero-padded to the word boundary in image columns (⌈N/8⌉
  words per value; the pad is encoding, not data — a trailing-pad nonzero
  byte is corruption).
- **The boundary is reuse vs identity.** `str` stays interned (names are
  reuse-shaped; interning is compression there). `bytes<N>` is for values
  whose *cardinality is their nature*. The docs state the decision rule in
  one sentence: *intern what repeats; inline what identifies* — and after
  this PRD the two remaining byte-shaped types share no axis
  (variable/fixed, interned/inline, text/raw, reuse/identity).
- **The deletion's dividend: the dictionary becomes str-only.** The type-tag
  byte inside every dict key hash (`blake3(tag ‖ bytes)`) existed solely to
  segregate `str` from `bytes`; it dies with the type. Dict keys hash the
  raw bytes; `TAG_STRING`/`TAG_BYTES` and the tag's corruption arm are
  deleted; `ResolveMemo`'s key drops from `(word, tag)` to `word`; the
  interning chapter tightens to one sentence ("the dictionary is the
  compression representation for repeated text"). When deleting a type
  simplifies a storage namespace nobody was looking at, the type was
  accidental — the deletion confirming itself.
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
- **The dict-GC OPEN item's content-churn trigger profile is reassigned** —
  the digest population leaves the dictionary entirely, returning the
  accepted leak to its original compression scope (repeated text only).

## Technical direction

1. Schema: `ValueType::FixedBytes { len }` **replacing** `ValueType::Bytes`
   — the variable variant is deleted, not deprecated (no aliases, per set
   policy); validation (0 and >64 rejected, typed); fingerprint feeds the
   length (a width change is a new theory). Macro grammar: `bytes<N>` with
   the width mandatory; bare `bytes` is an unknown-type error.
1b. The dictionary contraction: tag byte out of the key hash, `TAG_*`
   constants and the tag corruption arm deleted, `ResolveMemo` keyed by
   word alone, `ir::Value`'s variable-bytes variant deleted (params and
   literals carry `[u8; N]`-shaped values or `str`).
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
- `[shape]` Variable bytes is gone: `grep -ri "TAG_BYTES\|ValueType::Bytes\b"`
  returns nothing; the dict key hash carries no tag; bare `bytes` in the
  macro is a compile error at the call site.
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
