# PRD 15 — The introspection contract: "stable-ish" becomes a versioned promise

**Depends on:** 07, 08, 14 (every rename that touches EXPLAIN strings
has landed; the goldens pin the FINAL vocabulary once).
**Modules:** `crates/bumbledb/src/exec/explain.rs` (the "stable-ish"
header :9-10) + `exec/explain/{display.rs,into_stats.rs}`,
`api/prepared/introspect.rs`, `api/stats.rs`, new golden test files.
**Authority:** spec P1-3 + audit #11, verified: the module self-
declares its output "OPEN … stable-ish"; ordering is de-facto
deterministic (everything feeds from `Vec`s — verified, no map
iteration reaches the text) but nothing pins it, and there is no
version marker, so a planner change can silently reshape output a
host has learned to parse.
**Representation move:** determinism stops being an accident of `Vec`
iteration and becomes a contract: a version tag, a stated field order,
and goldens.

## Context (decided shape)

1. **The version tag.** The introspection report begins with one
   line: `introspection v1` (and `into_stats`' structured form
   carries `introspection_version: 1`). The language law applies:
   "EXPLAIN" survives only as the colloquial gloss in docs ("plan
   introspection — EXPLAIN, colloquially"); the artifact, its version
   line, and identifiers say introspection. The rule, stated in the module header
   replacing "stable-ish": within one version, byte-identical output
   for identical (schema fingerprint, canonical query, param types,
   feature set); ANY change to content or ordering bumps the version.
   The architecture README's OPEN marker for the explain shape
   resolves to this contract.
2. **The ordering law.** Already true, now stated and pinned: sections
   in fixed order; rules in program order; per-rule nodes in plan
   order; diagnostics (dead/subsumed/unresolved-literal from PRD 14)
   in statement order. One paragraph in the module header; the goldens
   enforce it.
3. **The goldens.** A dedicated golden suite over a fixture schema ×
   representative queries: a join with closed folds, a statically-empty
   program, a key-probe (PRD 08 vocabulary), an aggregate with the
   union regime, a query carrying an unresolved literal (PRD 14's
   line). Byte-exact assertion, one golden per case, values written by
   running the code once and HAND-REVIEWING every line against the
   docs before pinning.
4. **Structured stats parity:** `into_stats` carries the same version
   and the same ordering (its consumers are the bench and future
   tooling); one test asserts display and stats agree on rule/node
   counts.

## Technical direction

Survey first: enumerate every string EXPLAIN can emit (display.rs
arms) and check each against the post-rename vocabulary — this PRD is
the backstop that catches any straggler "chase"/"guard" string the
sweeps missed (grep the emitted corpus, not just the source). Then the
version line, then the goldens. Do NOT reorder anything — the point is
pinning what exists; any ordering change discovered to be needed is a
policy-5 stop.

## Passing criteria

- `[shape]` `grep -n "stable-ish" crates` → zero; the version line
  present in both display and stats.
- `[test]` The golden suite green; re-running the suite twice
  byte-identical (determinism smoke); the display/stats parity test
  green.
- `[shape]` The emitted-string sweep found zero stale vocabulary (or
  fixed them, listed).
- `[gate]` Fingerprint pin untouched; full suite green; clippy; fmt.

## Doc amendments (rule 6)

`70-api.md`: the EXPLAIN contract paragraph (version, determinism law,
what a version bump means); architecture README's OPEN item resolved.

## Execution reconciliation

The tree sorted unresolved-literal labels lexically, contradicting the stated
statement-order law. This was ledger-missed but mechanically resolvable under the
conflict protocol: diagnostics now deduplicate stably at first occurrence, and the
reverse-lexical hostile golden pins statement order.
