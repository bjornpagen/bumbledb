# PRD 25 — Vocabulary sweep

**Depends on:** everything (last PRD; the tree compiles again before this one
completes — this is also the PRD where residual compile breakage from the whole
plan is driven to zero).
**Modules:** the entire workspace; `README.md` (root, code-example section only).
**Authority:** `docs/architecture/00-product.md` (§ deleted vocabulary), `docs/prd/README.md` policy rule 5.

## Goal

The deleted vocabulary is gone from the codebase as *identifiers and concepts*;
the tree builds green; the root README's code example speaks the new language.

## Technical direction

1. **Sweep targets**, workspace-wide (`crates/`, `scripts/`):
   `unique`, `fk`, `foreign`, `primary_key`/`pkey`, `constraint`, `cascade`,
   `restrict`, `replace` (the operation — the string API `str::replace` is
   obviously fine; judgment required, which is why this is last and explicit).
   For each hit: identifiers/type names/function names → renamed into the
   statement vocabulary; comments and doc comments → rewritten to the new
   concepts (a comment explaining *history* may keep the old word if it cites an
   architecture doc's tombstone); test names → renamed to what they now test.
2. **Systematic procedure** (do it this way, not ad hoc): for each target word,
   run `rg -i --pretty <word> crates scripts`, classify every hit into
   {identifier, comment, false-positive}, and fix the first two classes. Zero
   identifier hits is the bar; a comment may keep a deleted word only when it
   states the current refusal (e.g. "no cascade exists; delete the cluster in one
   delta"), never to narrate what used to be.
3. **Root README:** the code example (`schema!` block and the surrounding prose
   that names `fk`/serial-unique behavior) is rewritten in statement notation to
   compile against the new macro. Do not otherwise rewrite the README (owner's
   document).
4. **Compile-and-gate closure:** this PRD ends with
   `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   and `cargo test --workspace` green — every `[test]` criterion from PRDs 01–24
   is now live and must pass. Fix residual breakage here; if a fix requires a
   *semantic* decision, stop and file it as a `## Conflict` per the README policy
   rather than improvising.
5. `scripts/check-asm.sh` and `scripts/check.sh`: update invocations/module paths
   that moved; the asm gates' *properties* (no calls/`bcmp` in probe loops) are
   re-asserted, and any gate that no longer matches a renamed symbol is re-pointed,
   never deleted (deleting a gate is an owner decision).

## Out of scope

New behavior of any kind. Architecture-doc changes (only if a Conflict demands an
owner ruling).

## Passing criteria

- `[shape]` `rg -i 'unique|foreign|fkey|\bfk\b|constraint|cascade|restrict' crates scripts`
  yields only current-refusal comments and string-API false positives.
- `[shape]` The root README `schema!` example uses statement notation and the
  seven-type vocabulary.
- `[gate]` `cargo fmt --all --check` clean; `clippy --workspace --all-targets
  -D warnings` clean; `cargo test --workspace` green — with every PRD's `[test]`
  criteria included.
- `[gate]` `scripts/check.sh` runs to completion on the new tree.
