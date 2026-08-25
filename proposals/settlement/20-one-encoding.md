# 20 — One encoding, one coordinate

> **Decision.** The protocol has **one encoding**: the length-delimited
> binary grammar the batch codec already speaks. The JSON document format
> — manifest, checkpoint, sidecar — is deleted, and with it every refusal
> arm, spelling law, and dependency that existed only because the
> protocol had a second, textual grammar. And the protocol has **one
> coordinate object**: `Vector`, the per-braid generation map, owning its
> algebra — sum, domination, order, floor — defined once, in one place,
> per driver.

This is the last unification. The cutover built one codec, one store,
one chain, one checkpoint spine, one floor, one machine — and left the
protocol speaking two languages: binary on the wire, JSON in the
documents. Two languages is two grammars, two parsers, two spellings of
every number and digest, and a standing invitation for the meta-cause
(one thing, implemented twice) to move back in.

## The current representation

- **Two grammars for one protocol.** A batch is binary: `u64le` is eight
  bytes, a digest is 32 bytes, a count is bounded by the bytes behind
  it. A manifest, checkpoint, or sidecar is JSON: the same `u64` becomes
  a *quoted decimal string* (because a JSON number cannot hold it), the
  same digest becomes *64 lowercase hex characters*, and the same value
  crosses a text encoding on every read and write.
- **The text grammar demands its own law book.** Across
  `manifest.rs`, `sidecar.rs`, `document.ts`, `manifest.ts`, and
  `chain.ts` there are ~79 sites of BOM handling, whitespace law,
  leading-zero refusal, duplicate-key refusal, quoted-bigint parsing,
  and hex-width checking — refusal arms that defend against states the
  binary grammar *cannot express*. A `DigestWidth` refusal exists
  because hex can be the wrong length; 32 raw bytes cannot. A
  quoted-decimal law exists because JSON numbers lose precision; `u64le`
  cannot. Canonical-form enforcement exists because content-addressing
  hashes bytes and JSON has infinitely many spellings of one value; a
  binary encoder has exactly one.
- **A dependency exists to serve the second grammar** (`serde_json` in
  `bumbledb-log`), and the checkpoint digest law carries a
  canonicalization clause only because text made "the bytes" ambiguous.
- **The coordinate is an alias, not an object.** `pub type Vector =
  BTreeMap<BraidId, u64>` — an alias with no algebra. So the sum
  (wholeness), the domination test (`wait_for`, gc), the checkpoint
  order, and the floor comparison are hand-rolled loops at each site
  (`gc.rs`, `replica.rs`, and the TS parse sites sprinkling
  `checkedAddU64`), and the Overflow refusal is re-implemented wherever
  a sum happens to be computed.

## The target representation

### 1. Every protocol object is the one binary grammar

The manifest, checkpoint document, and sidecar are binary records built
from the primitives both drivers already own (`u64le`, `u32le`,
length-delimited vectors, raw 32-byte digests), in the batch codec's
style: a version byte, fixed field rosters, vectors as
`count + (braid, g)` pairs bounded by their bytes.

- A `u64` is eight bytes. Precision loss is not refused; it is
  inexpressible.
- A digest is `[u8; 32]` on disk exactly as it is in memory. Width is
  not checked; it is the field's size.
- The content address is `blake3(bytes)` with no canonicalization
  clause, because one encoder produces one byte string.
- The version is the leading byte, and it says **3**: v:3 is the binary
  format. The JSON v:3 interlude never shipped and never existed
  publicly; the parser refuses anything that is not the binary magic,
  which subsumes the v:2 refusal.
- Keys drop the lie in their name: `manifest.json` → `manifest`,
  `ckpt/{digest}.json` → `ckpt/{digest}`, the local `chain.json` →
  `chain`. The `.mdb` sibling keeps its suffix; it names a different
  artifact, not a different grammar.
- Human inspection is a *tool*, not a wire format: `duty inspect <key>`
  renders any document to text. The protocol does not pay a second
  grammar so that `cat` is pretty.

### 2. `Vector` owns its algebra

`Vector` becomes a first-class type in both drivers, and the four
operations the protocol actually performs on it are defined exactly
once:

```
Vector.sum()        -> u64 | Overflow      // the wholeness arithmetic; checked in ONE place
Vector.dominates(o) -> bool                // wait_for, gc target, catch-up goal
Vector.order(o)     -> CheckpointOrder     // the total order the manifest CAS installs
Vector.at(braid) / Vector.advance(braid)   // apply's one mutation
```

The floor test, the sweep goal, the `wait_for` predicate, and the
checkpoint order become calls, not loops. The Overflow refusal lives
inside `sum()` and nowhere else. Encoding a `Vector` is one function in
the one grammar, so the quoted-decimal and hex spellings of its numbers
die with the format that demanded them.

## What gets deleted

| Deleted | Because |
| --- | --- |
| `serde_json` from `bumbledb-log`; every `json!`/`from_str` site | the second grammar is gone |
| `document.ts` (the JSON document walker) | the one grammar walks bytes |
| the JSON halves of `manifest.rs`, `sidecar.rs`, `manifest.ts`, `chain.ts` | fixed field rosters over the shared primitives |
| BOM, whitespace, leading-zero, duplicate-key, quoted-bigint, hex-width arms (~79 sites) | they guard states the binary grammar cannot express |
| the canonical-JSON clause in the checkpoint digest law | one encoder, one byte string, `blake3(bytes)` |
| `.json` in `StoreKey` spellings and the local sidecar name | the name described the deleted grammar |
| the ad-hoc sum/domination loops at every consumer site | `Vector` owns the algebra |

The v:3 golden corpus re-renders: document goldens become binary (hex
dumps may live in `inventory.json` as *test metadata* — the inventory is
a test artifact, not a protocol object). Refusal fixtures shrink to what
the binary grammar can actually refuse: truncation, bad magic, trailing
bytes, unknown braid, overflow.

## The invariant

> **One protocol, one grammar, one coordinate.** Every protocol object —
> batch, manifest, checkpoint, sidecar — is a sentence of the same
> binary language, so a number cannot lose precision, a digest cannot
> have a width, and a document cannot have two spellings. Every
> per-braid arithmetic — wholeness, domination, order, floor — is a
> method of `Vector`, so the protocol's semantics are written once per
> driver and the conformance inventory proves the two drivers speak the
> same sentences.

Dissolves the entire text-format residue class (the descendants of
findings 6, 9, 74, 77, 97, 106, 107, 108, 113, 130, 138 in
[90-traceability.md](90-traceability.md) — every one was JSON's
accidental complexity wearing a finding number), and the
scattered-arithmetic risk behind the 42/68/72 family. Consumes the codec
primitives of [00-canon.md](00-canon.md) §6; feeds the checkpoint digest
law (§4) and the generation function (§3).
