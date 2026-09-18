# Shelved participation-index follow-up: unvalidated draft

The latest validated result remains [relational Pack](RELATIONAL-PACK.md) and
its [frozen final audit](results/relation-pack-final-audit.json).

`participation-src/` is a separate, unvalidated draft. It proposes comparing the
complete and staged schedules with a right-first participation index and an
index that chooses its first branch using input-row counts. Proposed workloads
include balanced, skewed and unmatched branches. It is not a selected policy,
a measured improvement or part of the accepted production design.

The [initial source record](results/participation-unbuilt.json) identifies the
revision before compilation. A release build was subsequently started, then
cancelled when the user ended the research goal. The compiler and builder have
exited, and their disposable engine copy was removed. The
[closeout record](results/participation-shelved.json) retains source and helper
hashes and the cancellation log. No successful build, correctness run or
benchmark exists for this experiment. The new `participation_sweep.py` and
`participation_analysis.py` harnesses are also unvalidated.

This optional experiment is shelved at research closeout. It is not part of the
accepted evidence or a prerequisite for production planning. Resuming it would
require compilation, differential correctness, then isolated timings. See the
[research handoff](../../RESEARCH-HANDOFF.md) for the frozen conclusions and
remaining implementation work.
