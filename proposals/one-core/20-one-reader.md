# 20 — One reader

> **Decision.** The protocol grammar has **one implementation**:
> `crates/bumbledb-log`. The TypeScript package stops being a second
> reader and becomes what the engine SDK already is — typed payload
> construction and IO glue over a dumb napi bridge. The sealed byte
> grammar (batch codec, braid derivation, manifest/checkpoint/sidecar/
> scratch parse and render) crosses the FFI as a `LogCodec` handle in
> `ts/crate`, following the query builder's recipe with its documented
> warts fixed. The machines, the stores, the Vector hot math, and the
> key assembly stay TypeScript — that boundary is Brooks' essential
> line, not a compromise.

## The current representation

Two interpreters of one grammar, hand-locked (audit/20): 6,482 TS lines
mirroring ~10,685 Rust lines. The mirror's costs are no longer
hypothetical — the bug bash's grammar criticals (BOM, interval ceiling,
hex-vs-base64) were all two-readers-disagreeing bugs, and the audit
found fresh divergence on every unpinned surface. Meanwhile the
supposed benefit of pure TS — no native dependency — is **already
zero**: `internalDescriptor` is a value-import on the import path of
`codec.ts`, `braids.ts`, `manifest.ts`, `vector.ts`, and `chain.ts`, so
the "pure" codec cannot even be *imported* without the platform binary,
and every store verb mints etags through `internalBlake3` (audit/70).
The mirrored pair has paid native's costs without collecting native's
benefit for two releases.

The engine's bridge shows the target shape working at production scale
(audit/10): a boundary vocabulary of plain tagged payloads (not JSON
strings, not ad-hoc bytes), hand-written marshal walkers, opaque
`External` handles, a closed error taxonomy — and drift locks with
teeth: `wire_tags!` tables that break the compile when the core grows a
variant, a three-way `tags.json` golden, a cross-host fingerprint pin.

## The target representation

### 1. The `LogCodec` handle

`ts/crate` (the existing napi crate; one new path dependency on
`bumbledb-log`) exports a sealed per-theory handle mirroring
`Codec::new`:

- `logCodec(descriptor)` → opaque handle (sealed once per theory,
  cached — the same lifecycle as the engine's prepared queries).
- `encodeBatch(handle, header, ops)` → bytes; `decodeBatch(handle,
  bytes)` → the decoded batch in **engine types** (after
  [10](10-one-vocabulary.md) the rows *are* `FactValue` — the same
  declaration, no alias — so the decode marshal is the same `ValueOut`
  walk the engine already ships).
- `braidsOf` rides the existing `DescriptorWire`, once per theory,
  cached beside the handle.
- Document functions — manifest/checkpoint/sidecar/scratch parse and
  render — as plain bytes-in/values-out calls. All cold-path or
  fsync-adjacent: per-commit, per-applied-slot, per-refresh — noise
  against the conditional PUTs and fsyncs they precede (audit/70).
- The sidecar's **fs half stays TS**: the grammar crosses, the file IO
  does not.

### 2. The two blockers, paid up front

- **B1 — the feature split.** `bumbledb-log` today unconditionally
  pulls `object_store` + tokio, which would drag ~146 crates into the
  cdylib (audit/50). The crate splits a default-on `store` feature; the
  grammar core (`codec`, `braids`, `vector`, document modules) compiles
  dependency-lean, and `ts/crate` depends on
  `bumbledb-log --no-default-features`. The store feature is a *cargo
  feature*, not a code fork — one source tree, one grammar.
- **B2 — the error mint-table.** napi carries `{kind, message}`; the
  log's refusal identities must survive the crossing byte-for-byte. The
  boundary mints ts-log's sentinel errors from the **generated identity
  table** ([40](40-the-oracle.md)) — `Malformed`, `Version`,
  `UnknownBraid`, `Overflow`, and the rest cross as table rows, so an
  identity that exists on one side and not the other is a build error.

### 3. What stays TypeScript, and why it is a law

The essential line from audit/70, adopted verbatim as the boundary:
**anything whose signature contains a Promise, a `Db`, an fd, a clock,
or a process identity stays per-language.** Concretely: the replica and
writer steppers (every transition awaits a store verb), all three
stores and the fence/lease IO, the tenants LRU, the Lambda glue, the
Vector algebra (hot bigint math inside poll predicates), and key-string
assembly (hot, allocation-shaped). This is also the standing
`temporal-gate` law's shape — the pure/sync boundary the repo already
enforces — extended to the FFI: **nothing async crosses; nothing that
crosses is async.** audit/80's async-poisoning objection is thereby a
boundary condition of the design, not an argument against it.

### 4. The bridge's warts, fixed not copied

From audit/10's non-replication list, three become rules of the new
surface: no dead head data on the wire (payloads carry exactly what the
core reads); **payload keys join the goldens** (the three-way lock pins
keys, not just tags); and the generation/ABI tripwire is enforced by
the census roster, not a comment. The four hand-copies of the
lexical-capability pointer pattern stay an engine affair — the log
handle is sealed and immutable, so it needs none of it.

### 5. The gates reach the bridge

`ts/crate` is workspace-excluded (a foreign build system), which is the
lockstep defect shape (audit/80 F). Ruled: the battery gains the bridge
lane — `cargo fmt --check` + `clippy -D warnings` on `ts/crate` via its
manifest, plus the `.node` build, inside `scripts/battery.sh` — so
protocol law living in the bridge is inside the definition of green,
not beside it.

## What gets deleted

| Deleted | Because |
| --- | --- |
| `ts-log/src/codec.ts` + the codec half of `bytes.ts` (~975 lines, audit/20's rank-1) | one reader |
| the TS document/sidecar/scratch grammar halves of `manifest.ts`, `chain.ts`, `keys.ts` scratch codec | one reader |
| the TS braid union-find and its descriptor cache twin | rides `DescriptorWire` |
| the TS `Vector` *wire* encoder (grade-D: no golden, no caller — audit/20) | dead on arrival |
| `writeCanonicalLiteral` (~45 lines, pre-`internalDescriptor` residue) | dead since the seal crossed |
| `parity.test.ts`'s corpus walker, `conformance-v3` TS walker, `codec.test.ts` (~1,700 test lines) | replaced by [40](40-the-oracle.md)'s lanes |

Net: ~1,200 src + ~1,700 test lines of TypeScript die; the TS package
keeps payload types, machines, stores, tenants, and glue. The diff is
net-negative by thousands of lines or something was implemented instead
of deleted.

## The invariant

> **One implementation reads and writes the protocol's bytes.** A
> grammar divergence between the drivers is not caught by a lane or
> reviewed in a diff — it is unconstructible, because there is no
> second grammar to diverge. The bridge is dumb by law: shape checks
> only, identities from the generated table, nothing async across the
> boundary, and the battery's definition of green includes the bridge.

Dissolves: audit/20's share-ranked list items 1–4 and its grade-D
surfaces; audit/50's MOVE-NOW families with both blockers; audit/70's
cost ledger (each cost either paid here or shown pre-paid); audit/10's
recipe (followed) and warts (fixed). The apply-path prize
(`logApplySlot`) is deliberately **not** here — it is
[50](50-deferred-with-triggers.md)'s first trigger.
