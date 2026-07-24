## db.write silently commits when the callback returns abandon(payload)

category: bug | severity: medium | verdict: CONFIRMED | finder: ts:core

### Summary

`runDelta` — the body of `db.write` — inspects the delta callback's return value for exactly one hazard: a thenable (the async-callback refusal). It never probes for the `Abandon` sentinel, even though `abandon(payload)` is the documented callback-return protocol of the sibling verb `writeWitnessed`, whose docstring promises "nothing is committed, not even an empty commit." Because `DeltaBuild` is typed `(tx: Tx<Rels>) => void` and TypeScript's void-return assignability rule accepts any returned value, `db.write(tx => { ...; return abandon(reason) })` typechecks cleanly — and the delta commits anyway, returning `{ ok: true }`. The caller's explicit decline-to-commit is silently discarded.

### Evidence

All citations verified against the working tree.

- `ts/src/db.ts:1139-1154` — `runDelta` checks only `if (isThenable(built.data))` before proceeding to `native.txCommit(txHandle)` at 1155-1158. Its own comment (1140-1147) states the probe exists because an async callback "TYPECHECKS ... but" carries different semantics — precisely the hazard class the abandon sentinel also falls in.
- `ts/src/db.ts:1304-1308` — `isAbandon(built.data)` is consulted only in `witnessedAttempt`; nowhere in the `write` path.
- `ts/src/db.ts:144` — `type DeltaBuild<Rels> = (tx: Tx<Rels>) => void`. Verified with the repo's own `tsc --strict` that a callback ending in `return abandon({reason})` after `tx.insert(...)` is assignable to a void-returning signature (TypeScript void-return rule). It compiles with no diagnostic.
- `ts/src/db.ts:154-172` — the `Abandon`/`abandon` docstrings: "returning one ... aborts the attempt WITHOUT committing (no empty commit is ever issued)" and "aborts the delta (nothing is committed, not even an empty commit)". The sentinel's contract is unconditional in its own prose; `write` violates it silently.
- Test coverage gap: `ts/test/db.test.ts:447` and `:469` cover abandon only through `writeWitnessed`. No test passes an abandon-returning callback to plain `db.write`.

Design-doctrine angle (docs/design/representation-first.md lens): the sentinel reifies "decline to commit" as data — parse-don't-validate done right — but `write` then ignores the reified value, which is worse than never offering it: an illegal state (commit despite abandon) is not only representable, it is the silent default.

### Failure scenario

A host refactors a `writeWitnessed` callback into `db.write` (both take a tx-receiving callback; the shapes invite it) and keeps `return abandon(payload)` on the decline branch. The refactor typechecks with zero diagnostics. At runtime, every `tx.insert`/`tx.delete` the callback issued before reaching the abandon branch is durably committed, `runDelta` returns `{ ok: true, generation }`, and the decline path — the branch the author wrote specifically to prevent the commit — has no effect and produces no error. Data the caller explicitly declined to commit lands in the store with no diagnostic anywhere.

### Suggested fix

In `runDelta` (ts/src/db.ts, beside the thenable probe at line 1139), add an `isAbandon(built.data)` probe. Two consistent options:

1. **Refuse typed** (matches the thenable precedent and keeps abandon a `writeWitnessed`-only protocol): abort the tx and throw, e.g. "bumbledb write callback returned the abandon sentinel — abandon is the writeWitnessed protocol; nothing was committed."
2. **Honor it** (matches the sentinel's own unconditional docstring): abort and surface an abandoned arm — but this would require widening `WriteResult`, so the typed refusal is the smaller, precedent-consistent change.

Either way, commit becomes unreachable for a sentinel result; `isAbandon` and the abort machinery already exist at this site (`native.txAbort` is called two lines above for the thenable case).
