# V5 Algorithm Deletion Decisions

## Mixed Hash/LFTJ

Decision: deleted.

Evidence:

```text
Artifact: /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-no-mixed-nonjob.json
Artifact: /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-no-mixed-job-10k.json
```

Results:

- `cargo fmt --all --check`: pass
- `cargo check --workspace --all-targets --all-features`: pass
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass
- `cargo test --workspace --all-features`: pass
- non-JOB gates: pass, zero failures
- JOB 10k gates: pass, zero failures
- active LMDB Rust code has no `Mixed`, `Hybrid`, `NodeImpl::Hybrid`, `PlanFamily::Mixed`, or `QueryRuntimeKind::Mixed` references

Runtime changes:

- Partial hash-probe shapes now fall back to pure LFTJ.
- Full benchmark baseline already had no Mixed runtime users.

Accepted performance result:

- Non-JOB and JOB gate thresholds remain satisfied.
- No measured benchmark query required Mixed runtime.

## Hash Probe

Decision: deleted.

Evidence:

```text
Artifact: /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-no-hash-probe-nonjob.json
Artifact: /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-no-hash-probe-job-10k.json
```

Results:

- `cargo fmt --all --check`: pass
- `cargo check --workspace --all-targets --all-features`: pass
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass
- `cargo test --workspace --all-features`: pass
- non-JOB gates: pass, zero failures
- JOB 10k gates: pass, zero failures
- active Rust code has no `HashProbe`, `hash_probe`, `NodeImpl::HashProbe`, `PlanFamily::HashProbe`, or `QueryRuntimeKind::HashProbe` references

Runtime changes:

- Hash-probe runtime and planner family were removed.
- A formerly partial probe-shaped test now asserts pure LFTJ fallback correctness.
- Hash trie data structures remain because direct kernels still use hash tries internally.

Accepted performance result:

- Non-JOB and JOB gate thresholds remain satisfied.
- The v5 baseline already had no full-suite benchmark query selecting HashProbe.
- Pure LFTJ remains the protected general Free Join backbone.

## Tiny Project Sink

Decision: deleted.

Evidence:

```text
Artifact: /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-no-tiny-project-nonjob.json
Artifact: /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-no-tiny-project-job-10k.json
```

Results:

- `cargo fmt --all --check`: pass
- `cargo check --workspace --all-targets --all-features`: pass
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass
- `cargo test --workspace --all-features`: pass
- non-JOB gates: pass, zero failures
- JOB 10k gates: pass, zero failures
- active LMDB Rust code has no `TinyProject`, `TINY_PROJECT_THRESHOLD`, `is_tiny_project_candidate`, or `OutputSink::TinyProject` references

Runtime changes:

- All materialized projections now use the generic encoded project sink.
- Projection dedup and final-boundary decode behavior remain covered by tests.

Accepted performance result:

- Non-JOB and JOB gate thresholds remain satisfied.
- Removing the specialized tiny sink did not create benchmark gate failures.
