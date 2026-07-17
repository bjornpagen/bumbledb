# PRD-E — Doc reconciliation

Wave 1 · Repo: bumbledb (`TODO.md`, `docs/`) · depends on: — · parallel with everything

## Objective

Leave exactly one coherent, current-tense plan of record and retire the drift. As
of now there are THREE overlapping planning artifacts: `TODO.md` (the original
road map), `docs/road-to-1.0.0/` (the nominal-hardening packet, now superseded),
and `docs/structural-1.0.0/` (this packet). Reconcile them so nothing contradicts
and the superseded material is gone.

## Work

1. **Delete `docs/road-to-1.0.0/`** — it is superseded by this packet (its 02/03/09
   shipped; its 04–07 are replaced by S1–S4; its 01/08 are carried as A/R). Record
   the supersession in this packet's `00-README` (already done) so nothing is lost.
2. **Update `TODO.md`** to current-tense reality and point it at
   `docs/structural-1.0.0/` as the execution detail:
   - Done: v0.1.0 published + tagged (`v0.1.0`); the SDK relocated to `ts/`,
     arch-split-packaged; primer cut over to the registry (dev-dep) + Vercel fixed;
     the engine W-ledger, self-describing stores, EINVAL fix, and unconditional
     fresh law.
   - In flight (this packet): the engine panic-gap (A); the structural-B SDK
     refactor (S1–S4); the SDK cookbook (S5).
   - Parked: Wave 2 (heed flags C1, fuzz hunt C2) — idle machine; Wave 3 (bench
     re-true R1, `1.0.0` tag R2, republish R3) — idle machine + owner ceremony.
   - Rulings from this session: structural-B (the eight ratified points); no
     release until owner approval; the open republish-version decision (0.2.0 vs
     hold); Fable-only fanout.
   - Keep `TODO.md`'s existing owner-law framing (push discipline, measurement law,
     no worktrees) — those are unchanged.
3. **Amend `docs/architecture/70-api.md`'s bindings/SDK section** if it describes
   the SDK surface: the SDK is now structural (bare values, schema-level domains),
   not nominal-branded. Keep the engine-side facts (fingerprint parity, the
   two-boundary split) — only the SDK-skin description changes. Do not touch the
   normative engine semantics.

## Technical direction

- Docs describe the present tense (README rule 6: no history in the architecture
  docs; the packet and `TODO.md` may carry status). The Lean spec and the
  architecture chapters remain the normative authorities; this reconcile only
  aligns the PLANNING docs and the SDK-skin description.
- This PRD may run first or last in Wave 1 — it does not depend on S1–S4 landing
  (it describes the plan, not the code). If it runs before the refactor lands,
  phrase the structural sections as "in flight (this packet)."

## Passing criteria

- `docs/road-to-1.0.0/` is deleted; nothing references it.
- `TODO.md` is coherent, current-tense, and points at `docs/structural-1.0.0/`;
  no contradictory status between it and this packet.
- `70-api.md`'s SDK-skin description (if any) matches the structural surface; the
  normative engine semantics are untouched; `scripts/spec-census.sh` stays green
  (no citation orphaned).
- `cargo fmt --all --check` unaffected (docs-only); commit deferred to the Land
  phase (bundled with the docs commit).
