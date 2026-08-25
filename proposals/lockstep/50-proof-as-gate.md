# 50 — The proof becomes a gate

> **Decision.** The deletion proof stops being a transcript and becomes
> a **standing census roster** the battery runs on every gate, forever.
> The five unwritten rulings get written. The 141-row adversarial audit
> runs to verdicts. The receipt closes the pass with every transcript,
> the tally, and the one line the owner runs.

## The current representation

The cutover's proof obligation was executed as a **one-time grep
transcript**: run the absence list, publish the output, done. King's
distinction names the defect exactly — that is *validation*: it checked
the condition and threw the proof away, so nothing re-checks tomorrow.
The audit caught the consequence within hours of the proof's own era:
`ckpt_json_key` — six sites spelling the deleted grammar — alive in a
tree whose transcript would have read clean, because the transcript
tested spellings, ran once, and vanished.

Meanwhile the rulings ledger is silent where it must speak.
`RULINGS.md` — the log every flexed spelling is required to appear in —
has **zero entries from the one-encoding change**, though at least five
decisions of exactly the kind it exists for were made:

1. the version byte stays **3** (the JSON v:3 interlude never shipped;
   no phantom v:4);
2. the **theory file stays text** (hand-walked JSON; the machines-write-
   binary / humans-write-text boundary of [40](40-one-identity.md));
3. the **lease counter body is canonical decimal ASCII**, not a binary
   record (a lease is CAS'd metadata, not a protocol document — say so);
4. the **TS digest surface** (resolved by [40](40-one-identity.md) to
   branded bytes — the ruling records that hex-in-memory was tried and
   deleted, so it is not re-tried);
5. the **battery runs nextest** (one process pool; the swap and its
   config location).

And the 141-row adversarial audit — the acceptance test of the entire
campaign — has not run: no verifier citations exist for any row.

## The target representation

### 1. The absence list is data the census executes

`spec-census.sh` gains a checked-in **banned-token roster**: one file,
one line per `(token, scope)` pair, carrying the full absence list the
cutover earned —

`pid_alive` · `pidAlive` · `applied_pending` · `kill(0)` · `kill -0` ·
`refresh_braid` · `upsert` · `Ok(status.success())` · `ESRCH` ·
`gc fodder` · `serde_json` · `document.ts` · base64-pending ·
JSON-`number` u64 · `manifest.json` · `chain.json` · `.json"` store
keys · BOM/whitespace/leading-zero/duplicate-key arms · quoted-decimal
u64 · `hex32`/`digest32FromHex` on protocol parse/encode paths
([40](40-one-identity.md)) · `_json` in identifiers ([40](40-one-identity.md)) ·
raw regex literals in the TS surfaces (E1b's guarantee, kept forever)

— scoped to `crates/bumbledb-log/src`, `ts-log/src`, and
`examples/lambda/src`. The census fails on any hit and **prints the
roster line that fired**, so a violation names its own law. Adding a
deletion to the tree means adding its token to the roster in the same
commit — the deletion table of every future cutover has a place to live
that runs on every battery ([30](30-one-battery.md)), not a proposal doc
that retires. A parsed proof is carried forward; a validated one was
already lost once.

### 2. The rulings get written

The five entries above land in `settlement/RULINGS.md` in its existing
form — what was flexed, what invariant held, one entry each. The
binary/text boundary paragraph ([40](40-one-identity.md)) lands in
`settlement/00-canon.md` §6. The ledger's completeness becomes part of
the receipt's definition of done: a receipt that cannot cite a ruling
for every flexed spelling is not a receipt.

### 3. The 141-row audit runs to verdicts

As specified since the cutover, now executed: every row of
`settlement/90-traceability.md`, no sampling; verifiers briefed to
**refute** closure; a row passes only when the refuter must cite the
type or invariant that stopped them, `file:line`; two independent
verifiers per critical (rows 0–9), one per major/minor; any refutation
reopens the owning work before the receipt can exist. Rows are batched
by owning decision so each verifier reads one canon section deeply. The
verdict table (row → verdict → citation) is a receipt artifact.

### 4. The docs are cut back with the code

The pass eats its own residue the way every pass here has. Canon absorbs
the landed one-encoding facts — `settlement/00-canon.md` §3/§4/§6 are
amended to state binary v:3 documents, the `manifest`/`ckpt/{digest}`/
`chain` keys, and the `Vector` algebra as facts of the present tree, and
the "remaining delta" closing paragraph is rewritten to point at this
pass's receipt. Then the superseded pages die: `settlement/10-endgame.md`
(absorbed here), `settlement/20-one-encoding.md` (landed; canon speaks
it), and `settlement/DISPATCH.md` (superseded by
[DISPATCH.md](DISPATCH.md)) are deleted, and `settlement/README.md`
shrinks to what settlement now is — the law (`00-canon.md`) and the
proof artifacts (`90-traceability.md`, `RULINGS.md`). One law, one open
campaign, zero stale dispatches: the proposals directory obeys the same
one-writer invariant as the code.

### 5. The receipt

One commit, closing the pass and the campaign:

- the battery transcript — one script, one exit code
  ([30](30-one-battery.md)), including the lean lane's 0 disagreements;
- the census roster run, clean, named by commit;
- the 141 verdicts with citations;
- the rulings ledger, complete;
- the version: `0.19.0`, one writer, roster proven complete
  ([20](20-one-version.md));
- the deletion tally of this pass, in the house style;
- the one line the owner runs: `git push origin HEAD`.

After the receipt, `settlement/` and `lockstep/` are both eligible for
the retirement their predecessors got — the canon (amended with the
boundary paragraph) is the one surviving law, the roster and the battery
script survive as *gates*, and the proposals that built them go the way
of the grail.

## The invariant

> **A proof that ran once is an anecdote; a proof that runs on every
> gate is a representation.** Every deletion this repository has earned
> is a roster line the battery enforces forever; every flexed spelling
> is a ruling on the page; and no receipt exists without the verdicts,
> the transcripts, and the one version number.

Dissolves: audit C.7 (the empty rulings ledger), D.9–D.11 (the unrun
audit, the unformalized transcript, the missing receipt), and — as a
class — the recurrence of every absence the cutover paid for.
