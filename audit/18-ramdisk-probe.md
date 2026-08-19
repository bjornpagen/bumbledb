# 18 — The ramdisk devhonesty lane reads as red on machines without device attach

- **Status:** OPEN (verified 2026-08-19 — `cargo test --workspace` fails
  exactly this one test under a sandbox that blocks `hdiutil attach`).
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
