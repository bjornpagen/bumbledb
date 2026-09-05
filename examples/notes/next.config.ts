/**
 * Native packaging (chapter 33): serverExternalPackages keeps the
 * database packages out of the bundler; outputFileTracingIncludes ships
 * the SELECTED target's platform package with the server unit. The target
 * is an explicit build decision (BUMBLEDB_TARGET), never an import-time
 * guess — an AWS arm64 build traces linux-arm64, a Vercel x64 deployment
 * traces linux-x64. Externalization alone does not prove the .node file
 * shipped: inspect the emitted server unit and execute it on the target
 * (APP-04).
 */
import type { NextConfig } from "next"

const target = process.env.BUMBLEDB_TARGET ?? "linux-arm64"
if (target !== "linux-arm64" && target !== "linux-x64" && target !== "darwin-arm64") {
	throw new Error(`BUMBLEDB_TARGET must be one of linux-arm64 | linux-x64 | darwin-arm64, got ${target}`)
}

export default {
	serverExternalPackages: ["@bjornpagen/bumbledb", "@bjornpagen/bumbledb-log"],
	outputFileTracingIncludes: {
		"/*": [`./node_modules/@bjornpagen/bumbledb-${target}/**/*`]
	}
} satisfies NextConfig
