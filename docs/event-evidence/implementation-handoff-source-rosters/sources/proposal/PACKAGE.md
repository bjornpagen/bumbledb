# Event proposal package

[README](README.md) is the entry point. [proposal.md](proposal.md) is the current
contract; [implementation-plan.md](implementation-plan.md) defines M0–M8;
[the native ledger](../docs/event-implementation.md) states which gates actually
pass. The proposal remains revision 0.9 against public v1.3.1. Packaging it does
not mark the implementation complete or change any release version.

The live documents, all Lean source modules, exact finite reference programs,
laboratory source alternatives, historical text reports and recorded assembly
are versioned at their original paths. [package.json](package.json) is an explicit
path/length/SHA-256 inventory. It excludes disposable `.scratch`/`target` trees,
Python caches, compiled executables and the local paper/download cache. No local
research file is deleted by packaging. Current paper citations use the recorded
upstream URLs; bibliography manifests retain versions and hashes. Frozen archive
documents preserve their original local-cache references and wording.

The laboratory's timing reports are historical observations with their original
source/build metadata. Their binaries are not shipped, and their isolation checks
may correctly refuse the now-modified engine. Preserve the named baseline when
reproducing a historical experiment. Native Event performance still has its own
M8 qualification gate; source packaging supplies no speed claim.

## Verify from a checkout

```sh
python3 scripts/check-event-package.py
python3 proposal/semantics/check.py --label my-proof --lean /path/to/lean-4.32.0
python3 scripts/check-event-semantics.py \
  --lean /path/to/lean-4.32.0 --output docs/event-evidence/my-native-proof
python3 scripts/check-event-readiness.py \
  --proposal-proof proposal/semantics/results/my-proof \
  --native-proof docs/event-evidence/my-native-proof \
  --output docs/event-evidence/my-readiness
```

Use the same installed Lean 4.32.0 executable for both proof runners. New labels
preserve earlier evidence. The readiness checker validates exact source/log
correspondence and proof premises, and checks the retained seven finite reference
suites. It does not compile Rust or rerun timing experiments. Native regressions
have their separate command:

```sh
python3 scripts/check-event-maps.py --relations --queries \
  --output docs/event-evidence/my-native-qualification
```

That command requires the repository's pinned Rust toolchain and the TypeScript
workspace dependencies. No TypeSafe account or provider request is needed.

When source documents or new proof runs change, review the file roster and run
`python3 scripts/check-event-package.py --record` to refresh the current inventory.
After staging its exact paths, `--index` additionally checks every staged blob.
`scripts/check-event-checkout.py --lean /path/to/lean-4.32.0 --output
docs/event-evidence/my-checkout` builds a temporary copy of only staged files,
reruns both proof suites and the readiness audit there, records the input Git
tree and refuses a changing index. Its temporary copy is removed afterward.
Never add the whole laboratory blindly: its disposable build trees are much
larger than the sources and evidence. Immutable run manifests remain separate
from this editable package inventory.

## Proof boundary

Lean proves denotations under explicit premises: admitted legal spaces, exact
readouts, common environments, complete fibres, finite carriers, monotonicity,
and preserved query participation. The [proof matrix](semantics/README.md) maps
each contract to named theorems and the remaining implementation obligations.
Rust must establish those premises. The native differential, ownership, transport,
spill, cancellation and malformed-input tests supply separate evidence. Neither
the package audit nor a theorem count establishes Rust refinement.
