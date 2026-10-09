import { existsSync } from "node:fs"
import { createRequire } from "node:module"
import { fileURLToPath } from "node:url"
import { NativeLoadError } from "../errors.ts"

const require = createRequire(import.meta.url)

/** The platforms that ship a prebuilt addon package. */
export const SHIPPED_PLATFORMS = ["darwin-arm64", "linux-arm64", "linux-x64"] as const

/**
 * Loads the addon for `platform-arch`: a development build at the package root
 * (`bdb.<platform>-<arch>.node`, written by `scripts/build.ts dev`) wins over the
 * installed `@bjornpagen/bumbledb-<platform>-<arch>` package.
 */
export function loadAddon<T>(platform: string, arch: string): T {
	const target = `${platform}-${arch}`
	const dev = fileURLToPath(new URL(`../../bdb.${target}.node`, import.meta.url))
	const source = existsSync(dev) ? dev : `@bjornpagen/bumbledb-${target}`
	try {
		return require(source) as T
	} catch (cause) {
		throw new NativeLoadError({
			target,
			message: SHIPPED_PLATFORMS.some((shipped) => shipped === target)
				? `load the bumbledb addon from ${source}`
				: `no bumbledb addon for ${target}; prebuilt addons ship for ${SHIPPED_PLATFORMS.join(", ")}`,
			cause
		})
	}
}
