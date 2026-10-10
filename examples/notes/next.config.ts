/**
 * The database package stays out of the bundler, and the linux-arm64 platform package (the native
 * addon) ships with the server function, which runs on arm64 Lambda.
 */
import type { NextConfig } from "next"

export default {
	serverExternalPackages: ["@bjornpagen/bumbledb"],
	outputFileTracingIncludes: {
		"/*": ["./node_modules/@bjornpagen/bumbledb-linux-arm64/**/*"]
	}
} satisfies NextConfig
