# Ephemeral AWS buckets for the real-S3 suite

Alchemy 2 stack for one local run of `ts/test-s3` against AWS in us-east-1:

- the log on an S3 Express One Zone directory bucket in `use1-az4`, created
  through Cloud Control (`AWS::S3Express::DirectoryBucket`), since Alchemy has
  no directory bucket resource;
- checkpoints on a versioned S3 Standard bucket.

Every name derives from `ALCHEMY_STAGE`. State is local in `.alchemy/`. No IAM
role is created; the suite runs as the deploying principal. Credentials come
from the standard AWS chain, usually environment variables or `AWS_PROFILE`.

```bash
pnpm install
```

```bash
export ALCHEMY_STAGE=sanity-$(openssl rand -hex 3)
```

```bash
pnpm run deploy && pnpm run test:s3; pnpm run destroy && pnpm run verify-clean
```

`destroy` empties the directory bucket first, because Cloud Control refuses
to delete a non-empty one. `verify-clean` fails if either bucket of the stage
still exists.
