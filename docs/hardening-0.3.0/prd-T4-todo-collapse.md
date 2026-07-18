# PRD-T4 — TODO.md collapsed to reality

Wave T · Repo: bumbledb · depends on: —

## Objective

Root `TODO.md` (last rewritten 2026-07-17, pre-release) is stale nearly
throughout: it calls the published SDK 0.1.0 nominal-brand (0.2.0 structural is
live and tagged), lists Wave 1 as in flight (fully shipped), Wave 2 as parked
(shipped: C1 gravestones landed, C2 closed and recorded), and carries a
resolved republish decision. Rewrite it down to what is genuinely open, in
current tense, so the eventual 1.0.0 release-floor grep ("no unresolved TODO
rows") has nothing stale to trip on.

## Work

1. Verify current reality at HEAD before writing a word: `git log --oneline
   -30`, the v0.2.0 tag, `docs/hardening-0.3.0/` (this packet), fuzz/SESSIONS.md
   tail, the C1 gravestones (`crates/bumbledb/src/storage/env/open_env.rs`,
   `storage/commit/applier.rs`).
2. Rewrite `TODO.md` to at most ~30 lines containing ONLY:
   - What is open: this packet (`docs/hardening-0.3.0/` — the 0.3.0 wave,
     pointer only, no duplication of its contents), and the deferred 1.0.0
     close (R2: crate version + `v1.0.0` tag — owner-gated, explicitly
     deferred by owner ruling 2026-07-18).
   - A one-line "everything else shipped" pointer: structural SDK 0.2.0
     published + tagged `v0.2.0`, primer migrated, engine at zero known open
     semantics.
3. Delete everything else. History lives in git; the packet is the plan of
   record; TODO.md is not an archive.

## Passing criteria

- Every sentence in the new `TODO.md` is true at HEAD (each claim carries a
  pointer — a path, a tag, a commit — that a reviewer can open).
- No reference to any shipped work as pending; no version claim other than
  0.2.0-published / 0.3.0-planned / 1.0.0-deferred.
- ≤ ~30 lines. Commit in the repo's voice; push.
