# 00 — The brutal scope

The product is TWO use cases, and nothing else:

1. **The embedded library** — bumbledb + bumbledb-log in-process on one
   machine: dev, tests, desktop, and the local fleet (cases 2 and 5,
   already shipped). Gains one small part here: `memStore`, an
   in-memory five-verb ObjectStore, so library tests and ephemeral dev
   need no disk.
2. **AWS Lambda on arm64** — the serverless deployment: a Node Lambda
   (arm64, AL2023 runtime) embeds the replica+writer, `/tmp` is the
   disposable local cache reused across warm invocations, one S3 bucket
   is the log, commits race slots exactly as designed, and a Next.js
   app on Vercel calls the function URL. Vercel itself never loads a
   native module. Checkpoint and gc duty is a SECOND, tiny Lambda —
   the Rust duty binary on provided.al2023 — fired by an EventBridge
   schedule.

Amazon Linux is law: every linux artifact is built in an
`amazonlinux:2023` arm64 userspace and its glibc (2.34) is the
compatibility floor. Never Ubuntu userspace, on pain of a `.node` that
does not load where it deploys.

## IN (each names its consumer)

1. **The beauty pass** (10) — every future reader; the one free week
   for breaking renames. Runs first.
2. **linux-arm64 platform package** + roster widening + SDK **0.17.2**
   with `internalDescriptor` (20). Consumer: the app Lambda.
3. **The Rust `S3Store`** (20/30) — box B closes because its consumer
   now exists: the duty Lambda. Single storage class.
4. **The TS aws4fetch store** (30) — box C closes. Consumer: the app
   Lambda's writer. Plus `memStore` (use case 1's test tier).
5. **The duty binary with `--once`** (20) — one cadence check per
   invocation; the scheduled Lambda's whole body. Consumer: bounded
   logs, fast cold opens, enforced retention with zero residents.
6. **The CI lane** (40) — arm runner, amazonlinux:2023 build container,
   linux battery, artifacts for owner publish.
7. **The Alchemy example** (50) — bucket + IAM + two Lambdas + schedule
   + function URL, non-normative, smoke-run once.
8. **Publish prep only** — SDK 0.17.2 (three packages) and ts-log
   **0.18.0** (renamed surface), one command away; owner publishes.

## OUT (each with its reason and reopen trigger)

- **Any resident/long-lived AWS shape** (Fargate, ECS, EC2 services,
  `ack=local` in the cloud): the two use cases don't contain one.
  Trigger: a workload that measures Lambda's per-commit PUT latency as
  its bottleneck and needs 1 ms acks.
- **linux-x64**: neither use case loads it (Lambda is arm64 by
  ruling; the library case is the owner's Mac). Trigger: a named
  x64 consumer.
- **S3 Express One Zone dual-class**: configuration once the store
  exists. Trigger: a measured commit-latency need standard-class S3
  misses.
- **TS-native checkpoint duty**: the duty Lambda IS the recorded
  Rust-sidecar deviation, serverless-shaped. Trigger: a deployment
  that cannot run a second Lambda.
- **Vercel-side replicas** (true case 1 — functions embedding the
  library): would need Vercel-platform binaries and buys nothing while
  reads can ride the function URL. Trigger: a measured read-latency
  need that Lambda round-trips miss.
- **Drain packing, dual-PUT zone availability, the quantitative
  algebra**: standing triggers, unmoved.
- **Any engine change beyond the one napi export**: the engine is
  done; this pass consumes it.
