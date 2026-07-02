# PRD 00 — Workspace Skeleton

Authority: `docs/architecture/00-product.md` (64-bit only, Apple Silicon target,
toolchain posture), `README.md` rules.

## Purpose

Create the Cargo workspace every later PRD builds inside. Nothing functional.

## Technical direction

- Workspace members: `crates/bumbledb` (the library; everything lives here as modules
  until a PRD says otherwise) and `crates/bumbledb-macros` (empty proc-macro crate
  stub, filled by PRD 27 — proc-macros require a separate crate).
- `bumbledb` initial module tree (empty `mod` files with one-line doc comments only):
  `encoding`, `schema`, `storage`, `image`, `ir`, `plan`, `exec`, `api`, `error`.
- `rust-toolchain.toml`: pinned stable toolchain.
- Workspace lints in the root `Cargo.toml` (`[workspace.lints]`): `rust.unsafe_code =
  "deny"` at workspace level, overridden to `allow` only in the modules later PRDs
  sanction; clippy `all` + `pedantic` as warn, promoted to deny via the global command.
- 64-bit enforcement: `compile_error!` behind `#[cfg(target_pointer_width = "32")]` in
  `lib.rs`.
- Dependencies added now, exactly: `heed`, `blake3`. Nothing else (no serde, no
  thiserror — the error enum is hand-written in PRD 04; no anyhow ever).
- No CI files, no scripts directory, no line-count anything.

## Non-goals

Any functional code. Any test beyond `compiles`.

## Passing criteria

- Global commands green on an empty-module workspace.
- `cargo tree` shows exactly heed, blake3, and their transitive deps.
- Building for a 32-bit target fails with the explicit compile error (verified by
  comment/documentation of the cfg, not by installing a 32-bit toolchain).
