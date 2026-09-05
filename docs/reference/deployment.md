# Deployment contract and runbook

Status: permanent doc. Support floors and procedures below are the
selected contract. **Every measured envelope number stays NotRun until
the post-retirement qualification campaign records it — no latency,
size or concurrency figure here is evidence.**

## Supported targets and floors

| Target | Policy |
| --- | --- |
| Apple Silicon macOS arm64 | Canonical development/performance target. macOS 14 minimum plus current supported macOS. |
| AWS Graviton Linux arm64 | Supported portable correctness target. glibc 2.34 / Amazon Linux 2023 baseline (`@bjornpagen/bumbledb-linux-arm64` is built on amazonlinux:2023). |
| Vercel Node Linux x86-64 | Supported inside a measured envelope (below). The emitted x64 artifact, observed Node/libc/CPU, writable temp space and warm concurrency are qualified per deployment — Node compatibility alone proves none of this. |
| Node versions | Node 24 is the common deployment baseline (`engines: >=24`). Node 26 may be separately qualified on hosts that offer it; do not claim Vercel Node 26 unless Vercel's supported-version list includes it at qualification time. |

Baseline CPU paths: x86-64/SSE2 and ARMv8-A per platform ABI. Shipped
binaries are never compiled for the release machine's native CPU.

## Explicitly unsupported

Edge/Worker runtimes, browsers, React Native/Expo, musl/Alpine, Windows,
macOS x64 and 32-bit targets. Client/Edge imports of the packages must
fail usefully (no native artifact resolves; the loader refuses with the
shipped-platform roster). The Expo/Drizzle-style generated-migration
analogy is a workflow inspiration only — no mobile runtime exists.

## The Vercel envelope — NotRun

Facts recorded from official docs at proposal time (2026-09-04): full
Node API surface; Node 24.x default; standard function bundle 250 MB
uncompressed; 1,024 shared file descriptors including runtime usage;
memory/duration/payload limits are plan/runtime dependent deployment
inputs. These are *provider documentation facts*, not Bumbledb evidence.

The following are measured ONLY in authorized G15/APP-04/05/06 cells
and are recorded here after the runs; until then every cell is NotRun:

| Measurement | Value |
| --- | --- |
| Cold materialization (checkpoint + tail download/import/validate) | NotRun |
| Warm request latency distribution (p50/p95/p99) | NotRun |
| Concurrent-tenant FD/memory/disk budget per instance | NotRun |
| Writable temp-disk capacity observed on the deployed runtime | NotRun |
| Emitted server-unit size with the traced x64 native package | NotRun |

HostedHistory's local directory on such hosts is DISPOSABLE
materialization — S3 is the authority; a cold instance re-downloads the
selected checkpoint plus retained tail before serving. LocalHistory is
NOT durable on ephemeral function storage and must not be advertised as
such; large tenants use an ordinary Node host with adequate owned disk.
`InsufficientLocalDisk` is a typed refusal, not a crash.

## Native packaging checklist (Next.js)

1. `serverExternalPackages: ["@bjornpagen/bumbledb", "@bjornpagen/bumbledb-log"]`.
2. `outputFileTracingIncludes` names the SELECTED target's platform
   package explicitly (see `examples/notes/next.config.ts`); the target is
   a build decision, never an import-time guess.
3. Inspect the emitted server unit for the `.node` file and execute it in
   the target environment; never copy a macOS binary into Linux.
4. Test optional-dependency installation and pnpm workspace tracing.
5. No native library, database file, secret or admin plan enters a
   client/public bundle (APP-01).

## Credentials and IAM

- The server function's role carries exactly the data-writer scope:
  `s3:GetObject`/`s3:PutObject` on the log prefix (see
  `examples/notes/alchemy.run.ts` — the policy is ATTACHED via
  `server.bind`, not merely returned).
- Admin/GC/backup identities are separate, prefix-constrained roles with
  only their additional list/delete/source-target permissions.
- Credentials refresh through the supported provider chain during native
  operations; static keys are never committed. Vercel-to-AWS
  authentication is an explicitly configured credential source — the
  Alchemy deployment does not automatically supply it.
- The log bucket keeps a protected, never-deleted HEAD; bucket policy,
  conditional writes, TLS/encryption and region/account/prefix
  configuration are qualified in G08/APP-05/`S3-*` cells. Missing IAM
remains NotRun.

## Migration cutover runbook

Executed with `examples/notes/scripts/migrate.ts` or the app's own admin
job. Every operation carries a stable operator-minted operation ID
persisted BEFORE the first attempt; a lost response is resolved by
`status` with the same ID, never by assuming failure.

1. `status` — verify manifest/prefix/schemas and feasibility. Rehearse
   the plan against a verified backup first (operations-runbook.md).
2. `migrate` — durably freezes source admission, executes the checked
   plan chain natively into isolated staging, validates target laws and
   history, publishes ONE final target in Frozen/AwaitingCutover, returns
   `ready-to-switch { deploymentBinding, activation }`. It does NOT
   activate. `completed(paused)` reports a frozen source honestly; fix
   and rerun with the SAME operation ID.
3. Deploy the application with the new binding while the target stays
   frozen; run authorized read-only checks.
4. `activate` — the explicit one-time cutover using the saved activation
   ref; repeated activation returns the recorded outcome. Then re-enable
   traffic. The old source stays frozen.
5. Abort (before activation only): durably fences the target under its
   publication authority FIRST, then thaws the matching frozen source.
   Uncertain cancellation leaves the source frozen; a cancelled operation
   cannot resume. After activation there is no automatic rollback —
   receipts and external effects require an explicit decision/effect
   audit and a forward repair plan or documented loss acceptance.

Migrations run from an appropriately provisioned Node admin job with
disk/CPU/deadline budgets — never a request hook, Next build import or
worker startup path.
