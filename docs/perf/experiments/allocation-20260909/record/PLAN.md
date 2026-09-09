# Autoresearch restart — September 8, 2026

Active objective: continue trace-driven general engine improvements; prioritize
hardening, correctness, performance, then simplicity. Ordinary allocation and OS
mmap remain the unified model. No quota subsystem or new fallback architecture.

Baseline: published v1.1.0, b02a641e087364ec09c161a97c67c87cf626e6b2.
The initial worktree was clean, with no benchmark/profile/build running.

1. Build and freeze ordinary release and fully symbolized profiling executables.
2. Run all 13 supported local lanes serially with verification and macOS
   user-interactive QoS. This is a shared host; QoS is not hard P-core affinity.
   Removed steady/delete-heavy churn lanes and the deleted workload stay removed.
3. Latest user direction: **do not run another full trace until the existing
   traces have been exhausted for actionable improvements.** Neither the
   prepared 66-family trace pass nor the full-suite trace pass has started.
   Finish the already-running ordinary benchmark; then run allocation counts
   and the allocation-site census separately from clean timing. Mine the saved
   native captures, preserving their actual source/binary identities.
4. Read a broad cross-family selection of saved traces: self/inclusive costs,
   complete caller paths, CPU/draw, blocking/unrooted time, allocation census
   and live-owner lifetimes. Audit source changes before attributing historical
   costs to the current engine. CPU samples alone are not allocation counts.
5. Rank the highest-impact shared mechanism. Record its trace/source evidence,
   a falsifiable allocation/performance prediction and negative-control test
   before editing. Preserve semantics, lifetime safety and disk compatibility.
6. Implement coherent high-impact changes supported by those traces; prove
   correctness and allocation behavior; compare baseline/baseline and alternating
   baseline/candidate timing across affected and control families. Keep working
   through supported leads instead of collecting another broad trace after each
   edit. A new full trace requires an explicit evidence-backed audit that the
   saved traces' actionable leads have been implemented, disproved or shown
   obsolete. Revisit that gate only after exhausting those improvements.
7. Preserve raw failures, controls, source patches, binaries and hypotheses. Do
   not label external qualification or skipped checks passed. No new release.

This first goal continuation starts a new research round; there is no preceding
goal turn to classify. Launching/verifying the baseline and capturing evidence
constitute progress, not completion of the open-ended objective.
