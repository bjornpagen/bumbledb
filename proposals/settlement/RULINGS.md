# Rulings

Precedence decisions that flexed a decision-doc spelling to keep its
invariant. One entry per ruling; the owning doc's invariant block stays
binding.

## 40 §3 — orphans are addressable without a LIST complement (17, 10, 128)

40 §3 spelled collection as "walk the reachable set once and delete the
complement." `gc.rs` walks with GETs alone (no LIST), so that complement
is not a store verb.

1. **Loser self-deletes on Kept and every refused publish.** The
   candidate knows its own digest. `Ran::Kept { incumbent }` and every
   `Refused` arm delete that digest's `ckpt/{digest}.json` and
   `ckpt/{digest}.mdb`. Finding 17 is dissolved: a kept or refused
   loser is not gc fodder and is not uncollectable.

2. **Crash orphans: candidate digests live in a scratch lease; the
   successor sweeps at open.** Before the upload-before-decision
   window, the publisher writes its digest under the reserved
   `~lease` namespace at the known document `ckpt-scratch`. The
   successor GETs that document at open and deletes the named objects
   when they are not the live head. GET-only GC does not LIST-delete
   the complement; that spelling is flexed to this invariant.

The binding contract is unchanged: losers and crashes leave
known-orphan, collectable objects, never live objects with clobbered
links.

## Parent 10 — complement prose caught up to 40 §3

`proposals/10-protocol.md` still spelled orphan collection as the
reachable complement — the 40 §3 form this file already flexed. Parent
10 now states the flexed form (loser self-deletes; crash orphans named
in `ckpt-scratch`). No new flex.

The parent PRD set and the cutover folder retired into
proposals/settlement/ (00-canon.md); doc references above are historical.

## 20-one-encoding — the version byte stays 3 (b10ec531)

20's new grammar could have taken a new version byte. It does not:
documents begin with byte 3. The JSON v:3 interlude never shipped, so
there is no phantom v:4. The parser refuses anything that is not the
binary magic.

## 40-one-identity — the theory file stays text (6c17e3d9)

40's one-encoding law covers protocol objects — machines write binary.
The theory file is hand-walked JSON: humans write text. That boundary
is the law's other half, not an exemption.

## 20-one-encoding — the lease counter is canonical decimal ASCII (baebdd85)

20's binary grammar covers protocol documents. A lease is CAS'd
metadata, not a protocol document: the counter body is a canonical
decimal ASCII u64.

## 40-one-identity — a digest in memory is bytes (73a5c542)

40 tried hex strings as the TS in-memory digest. That surface is
deleted. A digest is branded 32 bytes in both drivers; hex is a
rendering.

## 30-one-battery — the battery runs nextest (46b0412b, 650a2875)

30's green is one script. The test lane is `cargo nextest run
--workspace` — one process pool. The config is `.config/nextest.toml`
at the workspace root.

## 20 — the async boundary is a runtime constructor refuse (29)

20 spells the async/sync boundary as a type: misuse will not compile.
Rust cannot inhabit a token that is unobtainable inside an async
context without a second public capability on `ObjectStore`. The
trait stays five sync verbs. `S3Store::new` and every verb return
`Err` when `Handle::try_current` is `Ok`, and never `block_on` from
that context. The compile-time spelling is this runtime constructor
refuse — the logged substitute.

The binding contract is unchanged: an async caller cannot
`block_on`-panic the dedicated runtime.
