/** Only frozen plain data graphs are stable cache keys. Typed-array contents
 * remain mutable even behind a frozen parent, so they never qualify. */
const immutable = new WeakSet<object>()

function isImmutable(input: unknown, visiting = new WeakSet<object>()): boolean {
	if (typeof input !== "object" || input === null) return typeof input !== "function"
	if (immutable.has(input)) return true
	const prototype = Object.getPrototypeOf(input)
	if (
		(prototype !== Object.prototype && prototype !== null && prototype !== Array.prototype) ||
		!Object.isFrozen(input) ||
		visiting.has(input)
	)
		return false
	visiting.add(input)
	for (const name of Reflect.ownKeys(input)) {
		const property = Object.getOwnPropertyDescriptor(input, name)
		if (property === undefined || !("value" in property) || !isImmutable(property.value, visiting)) {
			visiting.delete(input)
			return false
		}
	}
	visiting.delete(input)
	immutable.add(input)
	return true
}

/** Validation is never skipped for a mutable structural input. Validated
 * immutable outputs can be reused without requiring constructor identity. */
function descriptorCache<A extends object>(parse: (input: unknown) => A): (input: unknown) => A {
	const cache = new WeakMap<object, A>()
	return (input) => {
		if (typeof input === "object" && input !== null) {
			const found = cache.get(input)
			if (found !== undefined) return found
		}
		const result = parse(input)
		if (isImmutable(result)) {
			cache.set(result, result)
			if (typeof input === "object" && input !== null && isImmutable(input)) cache.set(input, result)
		}
		return result
	}
}

/** Detached inspection data: share proven immutable branches, copy byte
 * buffers and their parents, and preserve references within one snapshot.
 * This is for already checked internal data, not an input validator. */
function snapshotData<A>(input: A, onCopy?: (source: object, snapshot: object) => void): A
function snapshotData(input: unknown, onCopy?: (source: object, snapshot: object) => void): unknown {
	const copied = new WeakMap<object, object>()
	function visit(value: unknown): unknown {
		if (typeof value !== "object" || value === null || isImmutable(value)) return value
		const existing = copied.get(value)
		if (existing !== undefined) return existing
		if (value instanceof Uint8Array) {
			const bytes = Uint8Array.from(value)
			copied.set(value, bytes)
			return bytes
		}
		const result = Array.isArray(value) ? [] : Object.create(Object.getPrototypeOf(value))
		copied.set(value, result)
		for (const name of Reflect.ownKeys(value)) {
			if (Array.isArray(value) && name === "length") continue
			const property = Object.getOwnPropertyDescriptor(value, name)
			if (property === undefined || !("value" in property)) throw new TypeError("snapshotData requires checked data")
			Object.defineProperty(result, name, { value: visit(property.value), enumerable: property.enumerable === true })
		}
		Object.freeze(result)
		onCopy?.(value, result)
		return result
	}
	return visit(input)
}

export { descriptorCache, isImmutable, snapshotData }
