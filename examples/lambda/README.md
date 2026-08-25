# Lambda example

This directory is **non-normative**. It is not on the conformance path, not
wired into the engine battery, and not a second source of protocol law. It
exists so a real deploy is an afternoon of reading, not archaeology.

It installs from the **registry**, not this tree: `@bjornpagen/bumbledb@0.18.0`,
`@bjornpagen/bumbledb-linux-arm64@0.18.0`, `@bjornpagen/bumbledb-log@0.18.0`.
The owner's publish ceremony lands those versions before a stranger can
`pnpm install` this example the way this file describes.

Infra is Alchemy **v2 Effect** (`alchemy@2.0.0-beta.74` + `effect@rc`, which
is Effect 4 — `effect@latest` is 3.22.1 and wrong). The program is
`Alchemy.Stack` + `Effect.gen`. The handler is plain async — the smallest
honest shape for a Function URL plus a Scheduler raw invoke.

`alchemy aws bootstrap` is owner-later. This example does not run it. Swap
`AWS.state()` for `Alchemy.localState()` if you want disk-only iteration
before that bootstrap exists.

## The six resources

`alchemy.run.ts` provisions exactly these:

1. **One S3 bucket** — the log. Standard class. Versioning omitted (do not
   set Suspended). No lifecycle rules; gc is the retention law, not bucket
   config.
2. **One IAM role, as intent** — inline policy `Prefix`: `s3:GetObject`,
   `s3:PutObject`, `s3:DeleteObject` on `arn:aws:s3:::BUCKET/log/*`. No
   `s3:ListBucket`. Alchemy 2.0.0-beta.74 `AWS.Lambda.Function` always mints
   its own role and has no `roleArn`. This role is the intended document.
   It is not the function's execution role. See [IAM](#iam).
3. **One Lambda function** — `nodejs24.x`, `arm64`, 512 MB, 60 s timeout
   (the 3 s default is too short for `duty --once`). Bundle installs the
   three registry packages above. Node 26 is in preview; Alchemy's runtime
   union is only `nodejs22.x | nodejs24.x` and cannot type 26.
4. **One function URL** — `functionUrl: true` on that same function. The
   Vercel app calls this URL. Nothing bumbledb-shaped installs on Vercel.
5. **One EventBridge Scheduler** — `every("5 minutes")` invokes the same
   function with `{ "duty": true }`. Scheduler synthesizes a **second
   invoke role** so it can call Lambda. That role is AWS-required plumbing,
   not the function execution role.
6. **One duty Layer** — `layer/duty` extracts into `/opt`. Rolldown has no
   `extraFiles`; a LayerVersion is the representation that makes "binary in
   the zip" possible. Place the linux-arm64 `bumbledb-log-duty` artifact at
   `layer/duty/bin/bumbledb-log-duty` (mode `+x`) before deploy. The handler
   execs `/opt/bin/bumbledb-log-duty`.

No VPC, no alarms, no second function.

## Handler

One default export. The event is a parsed grammar (`src/request.ts`):

- Function URL HTTP — `GET` reads `note` rows; `POST` commits one note
  `{ "id": "<decimal u64>", "body": "..." }`. `id` is a canonical decimal
  string (no leading zeros, range `[0, 2^64)`). A malformed body or id is
  a 400 domain refusal.
- Scheduler `{ "duty": true }` — `execFile`s the bundled binary with
  `--once`.

The replica is a value on the execution environment (`src/handle.ts`),
opened over `s3Store` with `/tmp/store` as the disposable cache. A
failed open leaves the value absent and the invoke answers 503; the next
invoke retries. Cold start is checkpoint pull plus tail replay; the
handler prints `open <ms>` on first open and `commit <ms>` on each write
so the owner sees the budget.

Credentials are static from process env (`AWS_ACCESS_KEY_ID`,
`AWS_SECRET_ACCESS_KEY`, optional `AWS_SESSION_TOKEN`). Lambda injects
those from the **execution** role. The child constructs `S3Store` itself,
so construct-outside-async holds without the handler's help. The TS store
is constructed at module scope, also outside async.

## Duty argv

`--theory PATH` is required. The file is the crate corpus schema object
`{relations, statements}` and its fingerprint must match the handler's
`Notes` theory. The layer ships that file at
`layer/duty/bin/theory.json` → `/opt/bin/theory.json`.

The handler invokes:

```
/opt/bin/bumbledb-log-duty \
  --once \
  --store s3 \
  --bucket $BUCKET \
  --dir /tmp/duty \
  --theory /opt/bin/theory.json \
  --region $AWS_REGION \
  --s3-prefix $PREFIX
```

Optional flags the binary also accepts, unused here: `--prefix P`
(protocol prefix; omitted, defaults to empty — the store's `--s3-prefix`
already scopes the bucket), `--endpoint E`, `--writer N` (defaults to 0).

`PREFIX` is `log`. The replica prefix is empty because `s3Store` already
scopes keys under that S3 prefix. Duty gets the same split: `--s3-prefix`
is the store prefix, `--prefix` stays omitted.

## Honesty

Per-commit latency includes one conditional PUT to standard-class S3
(tens of ms) — published-ack is the only mode here, and that is the
durability being paid for.

Concurrent executions are concurrent writers and resolve by the loser
algebra. That is the feature.

A braid-hot workload shows up as re-judgments. Remedies are braid design
first, Express class second (its recorded trigger).

## Latencies (owner smoke)

Cold start (checkpoint pull + tail replay): `(owner smoke)`

Commit (one conditional PUT, standard-class S3): `(owner smoke)`

## IAM

Alchemy 2.0.0-beta.74 `AWS.Lambda.Function` always mints its own role. There
is no `roleArn`. Yielding `S3.GetObject` / Put / Delete "just for IAM"
always adds `s3:ListBucket` on the **whole bucket** and cannot
prefix-scope. This program does not yield those bindings. The handler
talks through `s3Store`. The `Fn` role above is the intended document
only.

Owner chooses later (not this example, not deploy):

1. Accept the derived Function role and the ListBucket leak (yield the S3
   bindings, or attach an equivalent after the fact).
2. Wait for Alchemy to accept `roleArn` (or an attach) so the intended
   prefix-only role can be the execution role.
3. Inject a separate IAM user key via env (`AWS_ACCESS_KEY_ID` /
   `AWS_SECRET_ACCESS_KEY`), bypassing the derived role for S3.

Until one of those three, a deploy with the derived role and no extra
policy will get `AccessDenied` on every store verb.

## Vercel

The stack output is one URL (`fn.functionUrl`). The Next.js app calls it
with `fetch`. Env on the Vercel side carries the URL and whatever
app-layer auth the app owns. Vercel never loads a native module.
