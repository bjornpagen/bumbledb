# Installed-package syntax fixtures (chapter 34)

These are the API-12 / PKG-03 consumer fixtures: the chapter 34
Rust / core-TypeScript / log-TypeScript spellings as standalone sources
that compile against the ACTUAL shipped surfaces — never against
workspace path aliases, `#`-import conditions or repo compiler settings.

- `core-ts/consumer.ts`, `log-ts/consumer.ts` — copied by
  `scripts/packed-import.sh` into the isolated tarball consumer, where
  they must typecheck under a strict downstream `tsc` against the STAGED
  `@bjornpagen/bumbledb` / `@bjornpagen/bumbledb-log` tarballs and the
  exact Effect `4.0.0-rc.112` peer. They construct programs and export
  them; nothing runs at import (laziness is part of the pin). The
  runtime smoke lives in `scripts/packed-consumer.ts`.
- `rust/` — a standalone Cargo package OUTSIDE the workspace consuming
  the `bumbledb` core crate as a downstream path dependency (crate
  publication is not authorized; `publish = false` — a path consumer is
  the installed-consumer stand-in for Rust). It compiles AND runs the
  chapter 34 schema/query/change flow:
  `cargo run --manifest-path examples/consumers/rust/Cargo.toml`.

All three spell the same `Learning` schema, so the cross-language
schema-identity gates (API-08, F-*) can compare one canonical
fingerprint across every public surface.

Execution happens only in F3 (the final verification campaign); these
sources are staged during implementation and must not be "fixed" by
weakening them to whatever happens to compile — a drift here is a
public-surface defect.
