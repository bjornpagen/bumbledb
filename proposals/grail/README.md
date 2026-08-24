# grail — two use cases, the beauty pass, and the Lambda deployment

The PRD set for the pass that hard-scopes the product to TWO use cases
— **the embedded library** and **AWS Lambda on arm64** — and spends the
one free week bumbledb-log will ever have for breaking renames: the
package has been published for a day and has zero consumers, so the
aggressive refactor costs nothing now and something forever after.

The architecture rulings that shrink everything: **Vercel never loads
the native module** (the Next.js app calls a Lambda function URL; the
function embeds the replica+writer with `/tmp` as its disposable cache
and one S3 bucket as the log); **there is no second function** (an
EventBridge schedule invokes the same Lambda with a duty event whose
handler arm runs the bundled Rust duty binary); and **Amazon Linux is
law** (every linux artifact builds in an `amazonlinux:2023` arm64
userspace — glibc 2.34, Lambda's own floor — never Ubuntu).
Consequences: one new binary target, three stores (two cloud, one
in-memory for use case 1's tests), one CI lane, one example, zero
servers.

These documents are normative for the pass; where they and a numbered
proposals doc disagree, this set wins for its duration and Lane X
amends the numbered docs. The pass deletes this directory as its final
act, receipts in 90.

| Doc | Contract |
| --- | --- |
| [00-scope.md](00-scope.md) | The two use cases; every OUT with its reason and trigger |
| [10-beauty.md](10-beauty.md) | The deep read and the quality pass: one descriptor authority, the naming law, parse-don't-validate closures, module splits |
| [20-linux.md](20-linux.md) | linux-arm64 on AL2023, the roster widening, the duty binary, the Rust S3Store ruling, SDK 0.17.2 |
| [30-s3.md](30-s3.md) | The aws4fetch store and memStore; the verb-to-header mapping; the gated smokes |
| [40-ci.md](40-ci.md) | The lane: amazonlinux:2023 container on the arm runner; the law applied to ci.yml's linux legs too; battery; artifacts for owner publish |
| [50-deploy.md](50-deploy.md) | The Alchemy example: bucket + IAM + one Lambda + schedule + function URL; non-normative |
| [90-rollout.md](90-rollout.md) | Lanes, order, gates, receipts, self-deletion |

House laws stand whole. The beauty pass runs FIRST because every later
lane writes against the renamed surface.
