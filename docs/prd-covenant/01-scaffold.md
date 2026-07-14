# PRD 01 — The scaffold: a living lake project with a pinned toolchain

**Depends on:** nothing. Everything depends on it.
**Modules:** new `lean/` at repo root (lake project), `.github/
workflows/ci.yml` (the lean lane), `scripts/lean.sh`, `docs/formal/`
(marked superseded, retired by PRD 14).
**Authority:** the covenant README; the crucible toolchain-pin
precedent (a dated, deliberate pin with recorded selection checks).
**Representation move:** the formal model stops being a frozen
artifact and becomes a buildable, CI-checked project the docs can cite
by theorem name.

## Context (decided shape)

1. **Toolchain.** `lean/lean-toolchain` pins the LATEST stable Lean 4
   release at execution time (elan has v4.28/v4.31 installed and
   resolves 4.32.0 today — `elan self update && elan toolchain install
   <latest stable>`, then pin that exact version string). The pin file
   carries no comments (Lean toolchain files are bare), so the
   selection record goes in `lean/README.md`: the version, the date,
   and the two checks (lake builds the tree; `lean --version` matches
   on aarch64-apple-darwin). The pin moves deliberately, never
   implicitly — same law as rust-toolchain.toml.
2. **Project layout** (lakefile in TOML or Lean form, executor's
   choice, recorded):

   ```
   lean/
     lean-toolchain
     lakefile.toml
     README.md            — scope, refinement chain, what Lean does NOT own,
                            the toolchain record, the gate law
     Bumbledb.lean        — the root import file (imports everything)
     Bumbledb/
       Values.lean          (02)
       Schema.lean          (03)
       Dependencies.lean    (03)
       Query/Syntax.lean    (04)
       Query/Denotation.lean(04)
       Query/Aggregates.lean(05)
       Exec/Sweep.lean      (06)
       Exec/Dedup.lean      (07)
       Exec/Rewrites.lean   (08)
       Txn.lean             (09)
       Bridge.lean          (10)
       Countermodels.lean   (02+, grows all campaign)
   ```

   This PRD creates every file as a compiling stub (module doc + the
   imports spine only — NO placeholder theorems, no `sorry`; an empty
   module is honest, a sorried one is not).
3. **`scripts/lean.sh`** — the gate: `cd lean && lake build` plus the
   placeholder battery (`grep -rn "sorry\|admit\|axiom " lean/Bumbledb`
   → must be empty; `axiom` matched as a declaration keyword, with the
   grep shaped to avoid comment false-positives, recorded in the
   script) and an `#eval`-free-of-`partial`-escape check is NOT
   required (partial defs are allowed only in PRD 13's driver, noted).
4. **CI lane.** `ci.yml` gains a `lean` job: install elan (cached),
   `scripts/lean.sh`. Per-push — the build is seconds at this size;
   if it exceeds 2 minutes by campaign end, PRD 14 moves it to the
   Miri cron and records the number.
5. **`docs/formal/` superseded marker.** Its README gains one line:
   the artifact is the statement inventory this tree was built from;
   the living spec is `lean/`; PRD 14 deletes the directory. (The
   SHA-pinned artifact remains reachable in git history forever.)
6. **`lean/README.md`** carries, verbatim-adapted from the covenant
   README: the refinement chain, the zero-duplication law, the gate
   law, the mechanism fence, the mathlib refusal, and the statement
   "this tree is the ONLY normative home of bumbledb's semantics; the
   architecture docs cite it and never restate it."

## Technical direction

`elan self update`, install latest stable, pin. `lake new` the
project, then reshape to the layout above (lake's default layout
differs; the module tree is the decided shape). Verify `lake build`
green with the stub tree. Wire CI by copying the existing job
conventions in `ci.yml` (caching style, runner). Do NOT port any
theorem in this PRD — the scaffold's whole job is to make PRDs 02–10
pure content work.

## Passing criteria

- `[shape]` `lean/` exists with the exact module tree; every file
  compiles; `grep -rn "sorry\|admit" lean/` → zero.
- `[shape]` `lean/lean-toolchain` pins an exact version; the selection
  record with both checks is in `lean/README.md`.
- `[gate]` `scripts/lean.sh` exit 0 locally; the CI `lean` job present
  and YAML-parses (first-run verification on push recorded as pending,
  the PRD-16-of-crucible precedent).
- `[shape]` `docs/formal/README.md` carries the superseded marker.
- `[shape]` `lean/README.md` carries the laws; zero-duplication law
  stated verbatim.

## Doc amendments (rule: docs cite, never restate)

`docs/architecture/README.md`: the one-paragraph pointer — the formal
spec lives in `lean/`, the docs' surviving duties are mechanism,
measurement, decisions, operations.
