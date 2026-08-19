# 18 — The ramdisk devhonesty lane reads as red on machines without device attach

- **Status:** **fixed this pass** — attach is `RamDiskProbe::{Attached,
  Unavailable}` (a sum, never a panic); the live lock is `#[ignore]`d
  like the other capability lanes. Test:
  `timed_families_refuse_a_live_ram_disk`.
- **Severity:** environmental / later.
- **Supersedes:** VER-04.

## Principle

An environment capability spelled as a test failure trains readers to ignore
red. The suite already has the right representation for "cannot run here" —
seven principled ignores; this lane predates the pattern.

## Evidence

- `crates/bumbledb-bench/src/devhonesty/tests.rs:76` —
  `timed_families_refuse_a_live_ram_disk` panics on
  `ramdisk.sh create failed: hdiutil: attach failed — Device not
  configured`. 408 other tests pass on the same machine.

## The fix

Probe for ramdisk capability first (attempt the attach in a helper that
returns a sum, not a panic); when absent, mark the test skipped-with-reason
like the existing ignores. The live lane still runs — and must run — on bare
metal before a release; record that in the release checklist next to the
Primer lane.

## Acceptance

- `cargo test --workspace` is green in a sandbox with no device attach, with
  the lane reported skipped-with-reason.
- The lane passes on bare metal (release-checklist row).

## Adjudication

**Checklist row — collision, not edited.** Lane J owns `docs/**`. No
release checklist with a Primer lane exists outside that set. The
closest Primer-lane release gates live in
`proposals/instance-lifetime.md` (Allocation and performance gates),
which this lane does not own. Suggested one-liner for whoever owns
that list, beside the Primer lane:

`The live ramdisk lock (\`timed_families_refuse_a_live_ram_disk\`)
passes on bare metal (\`cargo test -p bumbledb-bench
timed_families_refuse_a_live_ram_disk -- --ignored\`).`

The probe/skip gate does not depend on that row.
