# PRD 23 — Census close: the constitution is counted

**Depends on:** 01–22 all landed. Terminal, always last.
**Modules:** read-mostly across `crates/`, `fuzz/`, `scripts/`,
`docs/`; write access to this packet's ledger files and the
architecture amendment stragglers.
**Authority:** the campaign audit discipline (crucible PRD 09's
precedent): batteries at the END so mid-campaign regressions cannot
hide; the ledger converts "we aligned the vocabulary" into counted,
dated evidence.
**Representation move:** none.

## The batteries (results recorded IN THIS FILE: command, count, date)

1. **Vocabulary battery** — all dead tokens grep-zero across `crates/
   fuzz/ scripts/ docs/` (packet ledger files exempt): `CmpOp::Contains`,
   `ContainsVarVar`, `ContainsVarPoint`, `chase` (case-insensitive),
   `chase-off`, `guard` (domain sense — idiom survivors re-verified
   against PRD 08's list), `pitch`, `DistinctCounter`,
   `OverflowKind::Origins`, `Term::Duration`, `AggregateDuration`,
   `closed_member`, `coverage: bool`, `Enforcement::Probe`,
   `stable-ish`, `"tiling"` in cookbook outside the recorded survivors,
   `StatementDescriptor::Functionality`, `DuplicateFunctionality`.
2. **Representation battery** — `debug_assert` in encoding/encode.rs =
   0; `DisjointGuardProof` construction sites = 1; `DistinctWitness`
   mint sites = 1; `MemberSet` the only `[u64;4]` in schema.rs;
   `-> bool` absent from provably_distinct.rs; generation `u64`
   battery per PRD 05.
3. **Contract battery** — the theorem↔evidence table has zero
   "delivered by PRD NN" placeholder cells left (every cell now cites
   landed machinery); the eleven-row minimum holds; EXPLAIN version
   line present; the four PRD-11 locks, the PRD-12 reverse-key locks,
   the PRD-14 negated-binder locks all exist by name (grep the test
   names).
4. **Defensive-check census** — the standing counts (`unreachable!`,
   `assert!`, `debug_assert!`, `.expect(`) per file vs the crucible
   PRD-09 floor (121 non-test `unreachable!`); every delta attributed
   to a PRD by mechanism; any RISE explained or fixed.
5. **Doc-amendment checklist** — every amendment promised by 01–22
   verified present by grep (one row each).
6. **Refusal-ledger verification** — each README refusal still holds
   (no `partitions` sugar, no ScalarValue, no Rule→Clause, flat error
   family, no DetMap sweep) — grep-proven absences.

## The terminal gate

`scripts/check.sh` exit 0 (including the renamed `closed-fold-off`
matrix line); `scripts/check-asm.sh` exit 0 on a fresh release build;
`cargo test` in fuzz/ exit 0 (replay + sweep suites); the corpus digest
pin and the fingerprint pin byte-untouched across the whole campaign
(`git log -p` over the two test files shows zero edits since campaign
start — assert it); a 10k-run smoke on ops and rewrites, finding-free
or trophied.

## Passing criteria

- `[shape]` All six batteries green, recorded here with commands and
  dates.
- `[gate]` The terminal gate cashed in full; both pins provably
  untouched since campaign start.
- `[shape]` The reconciliation ledger in this packet's README marked
  CLOSED with the final commit hash.

## Doc amendments (rule 6)

The verification checklist IS the amendment duty; no new prose.
