# 50 — The Alchemy example

Non-normative by declaration: an `examples/lambda/` directory that is a
working deployment, smoke-run once during the pass, never wired into
conformance, and owned by whoever deploys it. It exists so the owner's
deployment is an afternoon of reading, not archaeology.

## The resources (one Alchemy program)

- **One S3 bucket** — the log. Standard class; versioning off (the
  protocol's objects are immutable or CAS'd; the log IS the history);
  no lifecycle rules (gc is the retention law, not bucket config).
- **One IAM role** for the function, minimal: GetObject, PutObject
  (conditional headers ride ordinary PutObject), DeleteObject, on the
  bucket prefix. No ListBucket — the protocol never lists, and the
  policy proves it.
- **One Lambda function** — Node 22, arm64, memory sized to the working
  set (512 MB start), `/tmp` at default unless the checkpoint budget
  says otherwise. The bundle carries the handler, `@bjornpagen/bumbledb`
  + `-linux-arm64` + `@bjornpagen/bumbledb-log` from the registry, and
  the `bumbledb-log-duty` binary as a packaged executable file.
- **One function URL** (or the owner's API Gateway, their call) — what
  the Vercel app calls, with whatever auth the app layer owns.
- **One EventBridge schedule** invoking the same function with
  `{"duty": true}` on the owner's cadence (start: every 5 minutes).

## The handler (~40 lines, in the example)

Module scope: `openReplica({ store: s3Store(env), prefix, dir: "/tmp/store",
theory })` + `openWriter` — built once per execution environment, reused
across warm invocations (Lambda's environment reuse IS the module-scope
singleton pattern; `/tmp` is the disposable cache the design assumes).
Two event arms: the duty event `execFile`s the bundled binary with
`--once` and the bucket args; everything else is the owner's API (the
example ships one read route and one commit route as the demonstration,
nothing more). Cold start = checkpoint pull + tail replay; the example
prints both durations so the owner sees their budget.

## Vercel wiring

The example's output is one URL. The Next.js app calls it with fetch;
env vars on the Vercel side carry the URL and the app-layer auth
secret. Nothing bumbledb-shaped installs on Vercel.

## Honesty recorded in the example's README

Per-commit latency includes one conditional PUT to standard-class S3
(tens of ms) — the published-ack mode is the only mode here, and that
is the durability being paid for; concurrent executions are concurrent
writers and resolve by the loser algebra (that is the feature); a
braid-hot workload shows up as re-judgments and its remedies are braid
design first, Express class second (its recorded trigger).
