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
