# TODO — the plan of record

## Open

- **The hardening-0.3.0 wave** — the plan of record is
  `docs/hardening-0.3.0/` (the PRD packet, ratified 2026-07-18). Truth
  repinned, domains derived from the laws, ψ shipped in the SDK, every
  consumer cut over; ships as 0.3.0. Read the packet; nothing is
  duplicated here.
- **The 1.0.0 close (owner-gated, explicitly deferred 2026-07-18)** —
  R2 of `docs/structural-1.0.0/`: crate version `1.0.0` + the annotated
  `v1.0.0` tag. Owner ceremony only; no agent bumps, tags, or publishes
  (`docs/hardening-0.3.0/00-README.md`, ruling 10).

## Everything else: shipped

The structural SDK is published as `@bjornpagen/bumbledb@0.2.0`
(+ `-darwin-arm64@0.2.0`), tagged `v0.2.0`; primer is migrated to it
(`../primer/package.json`); the engine has zero known open semantics —
the C1 heed flags are gravestoned
(`crates/bumbledb/src/storage/env/open_env.rs`,
`crates/bumbledb/src/storage/commit/applier.rs`) and the C2 fuzz hunt
is closed at the owner's call (`fuzz/SESSIONS.md`, commit `712abe57`).
History lives in git; this document is not an archive.
