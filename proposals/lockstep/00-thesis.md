# 00 — Thesis: the audit found duplicated facts, not broken code

## The finding

The pre-release audit ran the whole battery, the absence greps, and a
manifest sweep against the post-cutover tree. The protocol held: 655
driver tests green across `ts/` and `ts-log/`, the root workspace clean,
zero hits on every deleted representation. What failed was never the
protocol. It was the **infrastructure of truth around it**:

- `crates/bumbledb-log` sat at version `0.17.0` while every other
  manifest said `0.18.0` — through an entire release, and no gate
  noticed, because the lockstep gate's roster never contained it.
- The one command the repo calls "test everything" —
  `cargo test --workspace` — does not compile the log crate at all,
  because the log crate is its own workspace. A green root battery
  proves nothing about the component this whole campaign rebuilt.
- The battery exists in three spellings: prose in the endgame doc, YAML
  in the CI workflow, and muscle memory at the shell. The three already
  disagree — the doc says `cargo nextest run`, nextest is not installed
  on the development machine, and the CI installs it from a hardcoded
  `linux-arm` artifact.
- A digest is `[u8; 32]` in Rust and branded bytes in the TS chain, but
  the TS manifest and checkpoint hold it as a **hex string** — one
  identity, two in-memory representations, inside one driver.
- The cutover's grep-for-absence proof ran as a one-time transcript, so
  `ckpt_json_key` — a name that spells the deleted JSON grammar —
  survived in six call sites the day after the grammar died.
- A wall-clock-measuring test lives inside the correctness battery and
  failed under concurrent build load — a bench wearing a test's badge.
- Five spelling decisions the cutover legitimately made (the version
  byte staying 3, the theory file staying text, the counter staying
  decimal ASCII, the hex TS surface, the nextest swap) are on no page:
  `RULINGS.md` has zero E1 entries.

## The diagnosis

Every one of these is the same defect the 141 were, one level up: **a
fact with more than one writer.** The version is one fact written
fourteen times. The battery is one fact written three times. "The set of
crates that must build" is one fact written twice (two workspaces). The
identity of an object is one fact written two ways in one driver. The
deletion table is one fact written once as prose and never again as a
check. Brooks' line does not stop applying at the `src/` boundary — the
build, the release, and the proof are programs too, and their tables are
currently concealed in their flowcharts.

The lineage names the fix exactly:

- **SPOV 1** — the leverage is upstream, in the shape of the data. Do
  not write a better sync procedure between fourteen version fields;
  make thirteen of them derivations of one field.
- **SPOV 2** — the guards guard states a precise representation makes
  impossible. The lockstep gate is a guard; a version that is
  *inherited* cannot skew, and the guard has nothing left to check
  except the npm boundary where inheritance cannot reach.
- **SPOV 3** — the special case belongs to the representation. "Run the
  root battery, and also remember the log crate is special" is a special
  case. Merge the workspaces and the sentence "test everything" has no
  footnote.
- **Parse, don't validate** — a one-time grep transcript is validation:
  it checks and throws the proof away, so the check never happens again.
  A banned-token roster wired into the census is parsing: the proof
  becomes a type the gate carries forever.

## The five duplicated facts, and their one writer

| Fact | Writers today | One writer after |
| --- | --- | --- |
| the crate set that must build | two `[workspace]`s, two lockfiles | one root workspace ([10](10-one-workspace.md)) |
| the version | 14 manifests, hand-edited | `workspace.package.version` + a roster gate over npm ([20](20-one-version.md)) |
| the battery | endgame prose, CI YAML, shell memory | one script all three invoke ([30](30-one-battery.md)) |
| an object's identity | `[u8;32]` / branded bytes / hex strings | one branded 32-byte type per driver; hex is a rendering ([40](40-one-identity.md)) |
| the deletion proof | a transcript run once | a census roster run every gate ([50](50-proof-as-gate.md)) |

## What this is not

Not a re-litigation of the cutover — the six representations stand as
canon states them. Not a process document — nothing here asks anyone to
be more careful. Every item lands as a representation change with a
deletion attached, in the house discipline: state the current shape,
state the target shape, delete the old shape so no one can reach for it,
and prove the absence with a gate that runs forever, not a transcript
that ran once.
