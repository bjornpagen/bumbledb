# 60 — Parse, don't validate, at the codec

> **Decision.** The batch wire format and every protocol document
> (manifest, checkpoint, sidecar) are **one grammar** with **one codec**,
> shared by both drivers. The codec *parses* into narrow types — numbers
> are exact `u64`/`i64`, a row vector cannot claim more rows than its
> bytes back, a string cell is well-formed by construction, a 32-byte
> digest is `[u8; 32]`, a fixed interval is half-open — so the interior
> never re-validates and the two drivers cannot disagree.

## The current representation

The formats are prose, hand-parsed twice, and the two hand-parsers admit
different languages:

- **Numbers are not exact.** The TS sidecar and checkpoint parsers round
  `u64` fields through JavaScript `number`, losing precision above 2^53,
  so a large `g`, `ts`, or `writer` id is silently corrupted and
  publishers are misnamed (findings [108] [138]); fractional numbers
  surface a raw `RangeError` instead of a typed refusal (finding [113]).
  Rust sums are unchecked `u64` folds over parsed values, so a
  hostile-but-canonical checkpoint panics in debug and wraps the
  checkpoint order in release (findings [74] [77] [97]).
- **Encodings diverge.** The sidecar's pending bytes are lowercase hex in
  Rust and base64 in TS, in the same `v:2` document, so migrating a
  crashed directory across drivers silently destroys a durable pending
  (findings [9] [107] [130]); a leading UTF-8 BOM in a string field
  decodes to different values on the two drivers (finding [6]).
- **Row counts are unbounded.** `decodeBatch` reads an untrusted `u32`
  row count and loops with no relation to the bytes remaining; a
  zero-field ordinary relation consumes zero bytes per row, so a
  ~113-byte object drives billions of allocations — OOM — on *both*
  codecs, because Rust's `capped()` caps only the initial `Vec` capacity,
  not the loop (findings [50] [104]).
- **Strings are silently mangled.** `writeTagged` encodes with the
  non-fatal UTF-8 encoder, substituting U+FFFD for lone surrogates
  instead of refusing, so `encodeBatch` emits a value different from its
  input (finding [105]).
- **Widths are unchecked.** `encodeBatch` writes `fromHex(header.prev)`
  with no check that `prev` is 32 bytes, emitting a short header from a
  wrong-length string (finding [106]); encode has no `Arity`, no
  `ClosedRelation` identity, and unchecked value shapes can encode wrong
  bytes (finding [137]).
- **The interval ceiling is a value in one driver.** A fixed-width
  interval whose end reaches the domain MAX is a *ray*, which the Rust
  codec and engine law refuse (`IntervalOverflow`) and TS accepts on both
  decode and the `checkAgainst` encode gate (findings [37] [49] [63]).
- **Byte offsets panic on malformed input.** `schema_file` indexes out of
  bounds on a malformed theory and slices a `&str` at fixed offsets that
  land mid-UTF-8-char, panicking instead of returning a typed refusal
  (findings [11] [12]); `parse_value` resolves a multi-arm value object
  by serde's alphabetical key order, silently loading a value the author
  never wrote (finding [76]); `restore_to_vector` silently ignores
  unknown braid ids in the target (finding [78]); the duty argv parser
  lets a flag swallow the next flag as its value (finding [79]); the
  Lambda handler surfaces a malformed POST id as a runtime crash, not a
  domain response (finding [125]); the TS decoder has no fuzz/truncation
  coverage (finding [57]).

Every one is a boundary that *validated* (checked, then passed the wide
type inward) instead of *parsed* (checked, then passed a narrow type
inward). A wide type inward is a second chance to disagree.

## The target representation

### 1. One grammar, one codec, exact numbers

The wire format and each document have one grammar and one
encoder/decoder both drivers link against. Every numeric field parses to
an exact `u64`/`i64` (a `bigint` in TS, never `number`), so precision
loss above 2^53 (findings [108] [138]) is unrepresentable; a fractional
or out-of-range number is a typed parse refusal at the boundary, never a
`RangeError` escaping upward (finding [113]); sums are `checked`/
`saturating` and the parser bounds each value so a canonical-but-hostile
document is refused, not summed into a panic or a wrapped order (findings
[74] [77] [97]).

### 2. One encoding, canonical bytes

Pending bytes have one rendering — the codec's, byte-for-byte identical
across drivers — so a directory written by one driver reads on the other
(findings [9] [107] [130]). A string field is bytes-in, bytes-out with no
BOM handling and no re-interpretation, so the two drivers decode the same
value (finding [6]).

### 3. A row vector cannot outrun its bytes

The row count and the row bytes are **one length-delimited type**: the
decoder cannot enter the loop for a row the remaining bytes cannot back,
so a declared count larger than the bytes is a `Truncated` refusal
immediately — the zero-field relation cannot amplify because "zero bytes
per row" is a grammar the parser rejects for a nonzero count with no
backing bytes (findings [50] [104]). This is the same cap on both
codecs, in the parser, not in an initial capacity hint.

### 4. Strings and widths are types

A string cell is a `WellFormedUtf8` produced by a *fatal* encoder that
refuses lone surrogates, so `encodeBatch` cannot emit a mangled value
(finding [105]); `header.prev` and every digest field is `[u8; 32]`, so a
wrong-length backlink is unconstructible and the short-header emission
(finding [106]) cannot happen; encode enforces `Arity` and the
`ClosedRelation` identity and the value shapes it writes, so wrong bytes
are a refusal, not output (finding [137]).

### 5. Intervals are half-open (Dijkstra)

A fixed-width interval is represented half-open, so the domain ceiling is
**not a value** — a ray is not in the type — and both the decoder and the
encode gate refuse `end == MAX` identically (findings [37] [49] [63]).
The boundary case is gone because the representation does not have a
boundary to special-case.

### 6. Structured inputs are parsed grammars, not indexed bytes

`schema_file` parses over a validated grammar that returns
`TheoryFile::Shape` on malformation instead of indexing out of bounds or
slicing mid-char (findings [11] [12]); a value object with more than one
arm is refused (`object.len() == 1`), not resolved by alphabetical
accident (finding [76]); a restore target with an unknown braid is
refused, not ignored (finding [78]); the argv and the Lambda request are
parsed grammars whose malformed inputs are typed refusals /
domain responses, not flag-swallows or runtime crashes (findings [79]
[125]); and the codec has fuzz/truncation coverage mirroring the Rust
mutation lane (finding [57]).

## The invariant

> **The codec returns a value that is already correct — exact numbers,
> canonical bytes, well-formed strings, fixed-width digests, half-open
> intervals, and vectors that cannot claim more rows than they carry — so
> the interior never re-checks and the two drivers, linking one codec,
> cannot decode one byte string to two values.**

Dissolves: [6] [9] [11] [12] [37] [49] [50] [57] [63] [74] [76] [77] [78]
[79] [97] [104] [105] [106] [107] [108] [113] [125] [130] [137] [138].
The `StoreKey` grammar is shared with [20](20-store-contract.md); the
exact-number `Vector` and `Chain` types are consumed by
[30](30-pending-chain.md) and [40](40-checkpoint-chain.md).
