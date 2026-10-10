/** Runs ts/test-s3 against this stage's buckets under a fresh prefix. */
import { execFileSync } from "node:child_process"
import { randomUUID } from "node:crypto"
import { fileURLToPath } from "node:url"
import { buckets, region, stage } from "../buckets.ts"

const names = buckets(stage())
execFileSync("pnpm", ["--dir", fileURLToPath(new URL("../../../ts", import.meta.url)), "run", "test:s3"], {
	stdio: "inherit",
	env: {
		...process.env,
		BUMBLEDB_S3_TARGET: "aws",
		BUMBLEDB_S3_REGION: region,
		BUMBLEDB_S3_LOG_BUCKET: names.log,
		BUMBLEDB_S3_CKPT_BUCKET: names.checkpoints,
		BUMBLEDB_S3_PREFIX: `ci/${randomUUID()}/`
	}
})
