# Event evidence

The [implementation ledger](../event-implementation.md) records passing feature
slices and the remaining M0–M8 gates. The current typed-relation query checkpoint is
[native-query-relation-qualification](native-query-relation-qualification/check.json),
with exact source hashes and all fourteen native qualification commands.
[native-query-relation-semantics](native-query-relation-semantics/check.json) contains
266 Lean reports; the proposal adds 246 reports over its separate source suite.

[portable-package-check](portable-package-check/check.json) reruns both Lean suites
and the readiness audit in a temporary checkout containing only staged Git files.
Its input tree, package hash, exact commands, manifests and logs are retained.
At its recorded input tree this rules out a dependency on untracked research sources for those checks; it
does not qualify Rust, solver behavior or native performance.

Earlier passing directories remain immutable evidence for their named source
hashes. They are not assertions about every later commit. `implementation-handoff-*`
audits distinguish document/proof readiness from implemented feature acceptance.

The [archive manifest](archive/manifest.json) records eight superseded, partial or
failed local runs moved under `archive/` without changing their bytes. Their
original logs and outcomes are preserved. Archive status never promotes a failed
or incomplete run into accepted evidence. Current checks use the explicit passing
paths above, not a wildcard over every `check.json` in this directory.
