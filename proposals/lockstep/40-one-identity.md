# 40 — One identity

> **Decision.** A digest is **32 bytes, branded, end-to-end, in both
> drivers** — in the wire records, in every parsed value, in every
> in-memory structure. Hex is a *rendering* that exists in exactly two
> places: `duty inspect` output and refusal messages. And a name may not
> spell a deleted grammar: `ckpt_json_key` and its kind are renamed to
> what they address. The boundary between binary and text is written
> down once: **machines write binary; humans write text.**

## The current representation

The cutover made every digest `[u8; 32]` on the wire and in Rust, and
branded bytes (`Digest32`) in the TS chain and codec. But the TS
manifest and checkpoint stopped short:

```ts
interface Manifest {
  readonly fingerprint: string          // 64 hex chars
  readonly checkpoint: string | null    // 64 hex chars, or null
}
```

with `hex32`/`digest32FromHex` conversions at the binary boundary
(`ts-log/src/manifest.ts:107,185,197,199`). One identity, two in-memory
representations, inside one driver — and the wide one is `string`, a
type that admits every value that is *not* a digest. Every consumer of a
manifest either trusts the string's provenance or re-checks its shape:
the exact validation-instead-of-parsing regression the cutover spent 141
findings deleting. It also collides head-on with the standing proof
obligation — the absence grep for "hex-rendered digests in document
paths" hits these lines today.

Second residue, same family: `ckpt_json_key` survives as a **name** in
`manifest.rs` and five `replica.rs` call sites. It correctly produces
`ckpt/{digest}` — the `.json` suffix is dead — but the identifier
spells the deleted grammar, in a repo whose own commits state the rule:
*"the name described the deleted grammar."* Names are representations
read by humans; a lying name is a wide type for the reader.

Third: the binary/text boundary itself is undocumented, so it re-opens
at every file. The theory file is JSON (hand-walked, no serde_json); the
Lambda layer ships `/opt/bin/theory.json`; the conformance
`inventory.json` carries hex dumps. Each is correct — and none of them
says *why* it is exempt from the one-encoding law, so the next reviewer
re-litigates each one.

## The target representation

### 1. `Digest32`, everywhere, both drivers

The TS `Manifest`, `Checkpoint`, and every checkpoint-braid entry carry
branded `Digest32` (a branded 32-byte `Uint8Array`, the same type the
chain and codec already use) for `fingerprint`, `checkpoint`, `hash`,
`catalog`, and `prev`. `digest32FromHex`/`hex32` leave the parse and
render paths of protocol values entirely; the binary reader yields
`Digest32` directly (it already holds the 32 raw bytes — the hex
round-trip is pure loss). Equality is byte equality; a wrong-width or
wrong-alphabet digest is not refused, it is unconstructible. The public
API surface changes type — that is the point, and 0.19.0
([20](20-one-version.md)) is the breaking release that carries it.

### 2. Hex is a rendering, at the human boundary, enumerated

`hex32` survives at exactly four call boundaries, named here and
enforced by the census scope ([50](50-proof-as-gate.md)): (1) `duty
inspect` output, (2) refusal/error message text, (3) the key grammar's
one digest-to-key function (`ckpt/{hex}` is the key's own definition),
(4) test metadata (the inventory's hex dumps). No parsed value holds
hex; nothing parses hex back except the inspect tool and the key
grammar's one function. A fifth caller of `hex32` in either driver is a
census failure, not a review comment.

### 3. Names address objects, not grammars

`ckpt_json_key` → `ckpt_doc_key` (the document of the pair;
`ckpt_mdb_key` already names its sibling honestly). The rename sweeps
the definition and all call sites; the census roster
([50](50-proof-as-gate.md)) adds `_json` as a banned token in
`crates/bumbledb-log/src` and `ts-log/src` identifiers so the class —
not just the instance — is closed.

### 4. The binary/text boundary, written once

One paragraph, landing in `settlement/00-canon.md` §6 and quoted in
RULINGS: **protocol objects — anything a machine writes for a machine to
read (batch, manifest, checkpoint document, sidecar, scratch-lease
body) — are the one binary grammar. Human-authored inputs (the theory
file) and human-facing outputs (`duty inspect`, refusal text, test
inventories) are text.** The theory file staying JSON is not an
exemption from the law; it is the law's other half. With the boundary
written, `/opt/bin/theory.json` and `inventory.json` stop being
re-litigable and start being examples.

## What gets deleted

| Deleted | Because |
| --- | --- |
| `fingerprint: string` / hex-typed digest fields in TS manifest & checkpoint values | `Digest32` end-to-end; hex is unconstructible as an identity |
| `digest32FromHex`/`hex32` on protocol parse/encode paths | the reader already holds the bytes |
| `ckpt_json_key` (the name) | names may not spell deleted grammars; `_json` joins the banned-token roster |
| the unwritten status of the binary/text boundary | one written rule; per-file re-litigation ends |

## The invariant

> **An identity has one in-memory representation per driver — 32 branded
> bytes — and hex exists only where a human is reading.** A digest-typed
> value cannot be the wrong width, the wrong alphabet, or a
> non-digest string, because none of those are values of the type; and
> no identifier in the tree names a grammar the tree no longer speaks.

Dissolves: audit C.6 (the TS hex surface and its E3-grep collision), C.8
(`ckpt_json_key`), and the theory-file ambiguity behind C.7's missing
ruling. The boundary paragraph and the rename's banned token land in
[50](50-proof-as-gate.md)'s roster and rulings.
