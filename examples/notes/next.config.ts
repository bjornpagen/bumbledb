/**
 * The database package stays out of the bundler, and the selected target's platform package (the
 * native addon) ships with the server function. The target is an explicit build input
 * (BUMBLEDB_TARGET), never guessed at import time.
 */
import type { NextConfig } from "next"

const target = process.env.BUMBLEDB_TARGET ?? "linux-arm64"
if (target !== "linux-arm64" && target !== "linux-x64" && target !== "darwin-arm64") {
	throw new Error(`BUMBLEDB_TARGET must be one of linux-arm64 | linux-x64 | darwin-arm64, got ${target}`)
}

export default {
	serverExternalPackages: ["@bjornpagen/bumbledb"],
	outputFileTracingIncludes: {
		"/*": [`./node_modules/@bjornpagen/bumbledb-${target}/**/*`]
	}
} satisfies NextConfig
