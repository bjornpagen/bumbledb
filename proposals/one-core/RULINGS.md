# Rulings — the one-core pass

Precedence decisions that flexed a decision-doc spelling to keep its
invariant, one entry per ruling. The owning doc's invariant block stays
binding.

## S1 — vocabulary and pins

- **`Commit` dies rather than aliasing.** Rust call sites say
  `Admission<Slotted<R>>` with log-owned
  `Slotted { value, braid, slot, durability }`; TS composes the same
  shape under its tagged-sum house style. The no-alias law applied to
  the outcome shell itself.
- **`BraidOutcome` stays a named struct** `{ braid, admission }` in both
  languages so the rejected arm keeps its braid tag; the container of
  `commit_split` stays language-idiomatic (Rust `(R, Vec<BraidOutcome>)`
  tuple, TS tagged `split` arm) — the per-braid *element* is the one
  shared shape, the container is host idiom.
- **`WAIT_FOR_POLL_MS` fact ruled 10 ms** (the Rust value); the
  machine-constants table (`conformance/v3/machine-constants.json`) is
  the writer; TS's 20 ms was a pinned divergence and conforms.
- **`reserve_capacity` deleted, not twinned**: zero non-test callers
  repo-wide; `Error::ReservationShape` and the reservations derivation
  died with it. The verb-parity ruling for doc 10 §5 is "dies in Rust."
- **The empty-commit refusal stays and is typed** in both drivers (law
  6: the empty commit is not a commit); TS spells it as a named arm, not
  a bare throw.
- **A lease-cache refill is not an exhaustion**: `ErrRefillNeeded` is a
  driver-internal operational signal, not a `LeaseRefusal` arm, because
  Rust's `LeaseRefusal` has no such arm — the sum mirrors the shared
  algebra; the signal is host-local.
- **`Waited` is surfaced in TS as a lowercase-tagged sum**
  (`reached | wedged | refused`) per the file's house style; `wedged`
  carries the cause string TS has (Rust carries braid only) — a
  superset payload, same arms.
- **The fs lease protocol is the Rust spelling**: `LEASE/1` body,
  `~lease/{key}/{n}` tokens + `~head` pointer, 5 s TTL, break by expiry
  of the lease's own bytes only; release rewrites `expires=0` rather
  than deleting. TS `Liveness`/probe machinery deleted — the mutation
  lock never probes a foreign process.
- **TS temp sweep uses mtime age** (> stale window) instead of Rust's
  own-pid ledger — the pid ledger does not survive the TS process
  model; the honest analog never touches a live sibling's fresh temp.
- **The tilde table is the wider set** (15 code points; NFKC-closed at
  generation, so the runtime NFKC fallback died). Rust's 10-point set
  is the pinned drift and conforms in S2.
- **Key-grammar refusal posture pins the TS spelling**: Cc/Cf/Zl/Zp/Zs
  anywhere in a segment refuses; Rust's strip-Cf and Zs-blind paths are
  the pinned drift and conform in S2.
- **The counter body pins Rust's canonical decimal** (no leading zeros,
  refuse at parse); TS's `^\d+$` acceptance conforms in S2.
- **The Vector wire form is deleted, not pinned**: `encode`/`parse`
  had zero non-test callers in BOTH drivers; a wire form nothing
  produces is not a surface. TS side deleted in S1; Rust side dies in
  S2; the `vector-wire` surface left the manifest.
- **`surfaces.json` is its own roster**, separate from `inventory.json`
  (case roster) — two rosters, two jobs, no entanglement. `fuzz/` is
  not a surface; its subfamilies attribute to the surfaces they mutate.
- **The ts-log `Generation` brand keeps its name**: it brands the
  per-braid coordinate (the count), not an outcome slot; the
  one-name-per-coordinate rename applied to outcome fields that carried
  slots.
- **`LeaseRefusal` in ts-log stays despite zero external consumers**:
  it mirrors the shared algebra and is an identity-table pin candidate
  for S3, not dead code.
- **Refusal sidecars for lease/scratch carry no refusal name**: both
  drivers yield an undifferentiated `None`/`null` from those parsers;
  naming variants would over-pin.
- **Banned-token roster gains `conformance/` allowance** for the
  `.json` store-key line in the two driver scopes: corpus tables read
  by drivers are data references, not key spellings.

## S1 — engine-side

- **`catalogDigest` required a new napi binding**, not a type export:
  the bridge never exposed the core's `Db::catalog_digest`; the binding,
  the `native.ts` declaration, and the index export landed together.
- **`ManifestField` widened with `fresh`/`newtype`** so the descriptor
  spec re-join could die; the bridge stays dumb — shape only.
