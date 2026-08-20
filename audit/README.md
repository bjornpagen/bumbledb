Every audit in this directory was produced under
[REQUIRED-READING.md](REQUIRED-READING.md). Findings prefer changing
data/types/invariants over adding branches; representation defects outrank
control-flow patches. There is always a single way to do a thing; a second
way is a finding.

Start at [INDEX.md](INDEX.md).

**The convention:** one numbered file per open issue. Each file carries a
`Status` line at the top (`OPEN` / `fixed this pass` / `keep` /
`CONTESTED`), the principle it rests on, verified `file:line` evidence, the
fully fleshed fix, and acceptance gates. Fixers update the `Status` line
with test names when they land a fix; the auditor appends an
**Adjudication** block when a `keep` ruling is accepted, narrowed, or
contested — nobody deletes the other side's words. Standing do-not-fix
rulings live in [kept.md](kept.md).

The tree is hot: agents fix issues while audits run. Re-verify a file's
evidence against the working tree before starting work on it, and update
the `Status` line when you finish. Earlier pass documents (the 0.13 area
files and the 0.15 second-pass area files) are deleted; their record is git
history, and every still-open row from them was carried into a numbered
file here.
