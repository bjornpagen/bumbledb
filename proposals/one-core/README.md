# one-core — one grammar, one reader, one vocabulary

The campaign after lockstep. Eight investigators swept the engine/log
seam (`audit/10`–`80`) and found the last big duplicated fact in the
repository: **the protocol grammar has two readers.** `ts-log` is 6,482
lines of TypeScript hand-mirroring ~10,685 lines of Rust, module for
module, with no lock but tests — while the engine solved this exact
problem years ago: one core crate, dumb marshal-only bridges, and drift
locks that break the compile. The audit also caught the mirrored pair
*actively diverged in production surfaces today*: the two drivers'
fs lock files are mutually unreadable, `waitFor` on a wedged braid polls
forever in TS, and one error identity means two different things.

This campaign ends the second reader, unifies the type vocabulary so
`ts-log` speaks the engine's types verbatim, pins every dark surface the
corpus never covered, and replaces the two-readers oracle with a
stronger one — deliberately, as a ruling, not as an accident. **It ships
as 0.20.0, a bridge-burning release: zero backwards compat in the
package API, the exports, or the on-disk lease spelling — no
compatibility arm exists anywhere, by design.**

| Doc | Decision | Dissolves |
| --- | --- | --- |
| [00-thesis.md](00-thesis.md) | The audit findings are shadows of four representational decisions | — |
| [10-one-vocabulary.md](10-one-vocabulary.md) | One type vocabulary: `ts-log` speaks the engine SDK's types verbatim; `Batch` ⊂ `WriteTx`; `Commit` composes `Admission`; one algebra per fact | audit/30, audit/40 |
| [20-one-reader.md](20-one-reader.md) | One grammar reader: the sealed codec, braids, and documents move behind `ts/crate`; TS becomes typed payloads + IO glue, like the query builder | audit/10, audit/20, audit/50, audit/70 |
| [30-pin-the-dark.md](30-pin-the-dark.md) | Every unpinned surface gets a golden; the five live drift bugs are fixed against ruled spellings | audit/20, audit/40, audit/50 |
| [40-the-oracle.md](40-the-oracle.md) | The two-readers witness is retired on purpose and replaced: independent golden generation + the engine's compile-breaking drift locks | audit/70, audit/80 |
| [50-deferred-with-triggers.md](50-deferred-with-triggers.md) | What this campaign refuses to do, with the reopen trigger written for each | audit/50, audit/60, audit/80 |
| [90-traceability.md](90-traceability.md) | Every audit finding → the decision that dissolves it | all |
| [DISPATCH.md](DISPATCH.md) | The fanout prompt: recon → vocabulary ∥ pins → bridge → oracle → green → receipt | — |

**The law returns.** The prior orchestrator retired the entire
`proposals/` tree at `49d45b5c` ("settlement and lockstep live in the
code and the gates") — which deleted the canon itself, leaving the
written law with zero writers. This campaign's receipt reinstates it:
the canon is recovered from `49d45b5c^`, amended with this campaign's
landed representations, and lives at `proposals/CANON.md` — one law file
at the proposals root, campaigns as subfolders beside it, retired when
closed. The law is a fact; facts have one writer.

Reading order: [00-thesis.md](00-thesis.md) first;
[10](10-one-vocabulary.md) and [30](30-pin-the-dark.md) land
concurrently and first; [20](20-one-reader.md) is the centerpiece;
[40](40-the-oracle.md) is the ruling that makes 20 honest;
[DISPATCH.md](DISPATCH.md) runs it end to end.
