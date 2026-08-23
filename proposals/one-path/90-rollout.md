# 90 — Rollout: the deletion campaign

Self-contained dispatch. The normative truth for the pass is this
directory; where it and a numbered proposals doc disagree, THIS SET wins
(it is the owner's ruling on the disagreement) and the numbered doc gets
amended by Lane DOC. This file is the only one-path file lanes edit
(receipts in the checklist).

## Ground rules (binding on every lane)

1. one-path/00 is the operating law: question, delete, simplify; the
   representation question first at every fix; unbounded repair + scream;
   never weaken a red test — delete it WITH its dead requirement or fix
   the code.
2. House laws stand: one way per question; zero dyn; sums for outcomes;
   parse-all-first; comment law (census-enforced, banned tokens, no
   proposals/ paths in code comments); commit style (house voice, one
   deliverable per commit, explicit paths, index.lock retry).
3. Work on the main tree. Never push. Never publish. Lanes own disjoint
   files except where a lane order below says otherwise.
4. Every deletion is counted and named in its commit message. The final
   receipt sums them.

## Lanes and order

```
Lane R (Rust one-path + writer repairs) ──┐
Lane T (TS one-path + ErrStore + sweep)  ─┼──► Lane P (store protocol, both languages,
Lane L (Lean trim + ledger 104)          ─┘        + interop + TS multi-process lanes)
                                                      │
                                   Lane DOC (all amendments, once) ──► Lane X (cutover:
                                                      packaging prep, S3-if-network,
                                                      battery, receipts, tally,
                                                      DELETE proposals/one-path/)
```

R, T, L run in parallel from minute one (disjoint files). P follows R and
T only because both languages' store files are being rewritten against the
one protocol and the interop lane needs both sides' final loss path; L
joins before DOC (the docs cite the ledger count). DOC writes every
amendment once against final shapes. X closes.

- **Lane R** — crates/bumbledb-log: 10's cut (writer loser region, delete
  intersect.rs, footprint.rs, the wire section, W arithmetic, counters)
  plus 40's writer/replica repairs (unbounded+scream, duty off the lock,
  orphan prev proven-by-CAS, knob deletions, deposition sentence, sidecar
  canonical-fixpoint, dual-corruption loudness) plus 60's Rust test
  consequences (deletions, reshapes, corpus regeneration). Gates: fmt,
  clippy -D warnings, full crate tests.
- **Lane T** — ts-log: the same cut twinned (delete intersect.ts,
  footprint.ts, wire section, counters; one loss path; violation-sourced
  contention payload) plus the ErrStore chain fix with identity test,
  the rotated-dir sweep, the named waitFor constant, README runbook line
  updates deferred to DOC where they are doc, done here where they are
  code. Gates: tsc, biome, full suite.
- **Lane L** — lean/: 20 whole (delete the footprint model, L6–L8, the
  relaxed family, both countermodels; restate L9 over the statement
  graph; ledger 107 → 104; census token roster updated). Gate:
  scripts/lean.sh whole.
- **Lane P** — 30 whole: the unified on-disk protocol in both store
  implementations (link create, computed etags, pid-lockfile CAS,
  flock/libc/unsafe/.etag/random-token deletions), the interop lane green
  from both sides, the TS multi-process lane green. Gates: both language
  suites plus the two new lanes.
- **Lane DOC** — 50 whole: every numbered doc amended once; 15 deleted
  with its disposition map executed; proposals/README.md table updated
  (15's row out; no new rows — this directory is not product spec).
- **Lane X** — 70 whole (packaging prep, S3-if-network) plus 60's battery
  run WHOLE, proposals/90-rollout.md receipts re-issued with the new
  landing hashes and every deviation, the deletion tally computed and
  reported, and the final act: **git rm -r proposals/one-path/** in the
  closing commit, whose message is the handoff note (70).

## Acceptance checklist (receipts land here)

- [ ] R: one loss path shipped; intersect/footprint/W-arithmetic/wire
      section deleted; repairs landed; Rust suite green
- [ ] T: TS twin of the cut; ErrStore identity fixed and pinned; sweep +
      constants; TS suite green
- [x] L: Footprint model and L6–L8 gone; L9 restated; L10 untouched;
      ledger 104; lean.sh green. Landed at c6a359dc (both countermodels
      plus their four fixtures out of Countermodels.lean) and f2a65db1
      (the model, the W arithmetic, L6/L7/L8 with the relaxed family and
      the stability engine — 44 named declarations; L9 restated over the
      statement graph and the judgment via three congruences;
      Txn/Footprint.lean renamed Txn/Braids.lean, namespace following).
      One recorded deviation: the file rename moves L10's qualified name
      to Bumbledb.Txn.Braids.L10, so the L10 ledger row's two citation
      tokens follow it — premise, mechanism, and instrument are
      byte-identical. One recorded strengthening: restated L9 quantifies
      the braid's own batch freely (only the foreign history's LocalTo
      is spent), so the old hd hypothesis carries no consumer and is
      gone. lean.sh green twice, the second run concurrent with lanes
      R/T mid-flight: census 104 rows / 332 tokens, corpus 277 cases at
      0 disagreements, three-way comparator green.
- [ ] P: one on-disk protocol, two conforming implementations; interop
      lane green both directions; TS multi-process lane green;
      flock/libc/unsafe/.etag/random-etags deleted
- [ ] DOC: 15 deleted, disposition executed; 00/10/20/30/40/50/60/70/80
      amended once each; no doc-vs-code disagreement stands in either
      direction
- [ ] X: battery whole and green; 0.17.1 prepared unpublished; S3 boxes
      resolved honestly (built, or reason restated); deletion tally
      reported; proposals/one-path/ deleted; handoff written
