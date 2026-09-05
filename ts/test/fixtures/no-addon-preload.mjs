/**
 * D22 / TS-010: make the platform addon unresolvable before any package
 * import. A successful schema import with the addon still installed is not
 * this discriminator.
 */
import Module from "node:module"

const original = Module._resolveFilename
Module._resolveFilename = function resolveWithoutAddon(request, parent, isMain, options) {
	if (typeof request === "string" && /^@bjornpagen\/bumbledb-(darwin|linux|win32)-/.test(request)) {
		const error = new Error(`native unavailable: ${request}`)
		error.code = "MODULE_NOT_FOUND"
		throw error
	}
	return original.call(this, request, parent, isMain, options)
}
