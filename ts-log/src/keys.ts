/**
 * The key layout: generation numbers zero-padded lowercase hex, 16
 * chars; a prefix is a store; a tenant is a prefix under `t/`. A
 * StoreKey is parsed here, once — the verbs take the proof and never
 * re-check. An empty prefix joins as the rest alone, so a leading
 * slash is unrepresentable.
 */

import * as errors from "@superbuilders/errors"
import { U64_MAX } from "#bytes.ts"
import type { Braid } from "#descriptor.ts"

/** Suffix of the per-key pid-lockfile. A segment wearing it cannot be a key. */
const LOCK_SUFFIX = ".lock"

declare const storeKeyBrand: unique symbol
type StoreKey = string & { readonly [storeKeyBrand]: typeof storeKeyBrand }

declare const generationBrand: unique symbol
type Generation = bigint & { readonly [generationBrand]: typeof generationBrand }

function storeKey(raw: string): StoreKey {
	if (raw.length === 0 || raw.startsWith("/") || raw.endsWith("/")) {
		throw errors.new(`store key is not a slash path: ${raw}`)
	}
	for (const segment of raw.split("/")) {
		if (segment.length === 0 || segment === "." || segment === "..") {
			throw errors.new(`store key segment is illegal: ${raw}`)
		}
		if (segment.endsWith(LOCK_SUFFIX)) {
			throw errors.new(`store key collides with the lockfile suffix: ${raw}`)
		}
	}
	return raw as StoreKey
}

function generation(raw: bigint): Generation {
	if (raw < 0n || raw > U64_MAX) {
		throw errors.new(`generation is a u64: ${raw}`)
	}
	return raw as Generation
}

function hex16(g: Generation): string {
	return g.toString(16).padStart(16, "0")
}

function assemble(prefix: string, rest: string): StoreKey {
	return storeKey(prefix.length === 0 ? rest : `${prefix}/${rest}`)
}

function manifestKey(prefix: string): StoreKey {
	return assemble(prefix, "manifest.json")
}

function logKey(prefix: string, braid: Braid, g: Generation): StoreKey {
	return assemble(prefix, `log/${braid}/${hex16(g)}`)
}

function checkpointMdbKey(prefix: string, digest: string): StoreKey {
	return assemble(prefix, `ckpt/${digest}.mdb`)
}

function checkpointJsonKey(prefix: string, digest: string): StoreKey {
	return assemble(prefix, `ckpt/${digest}.json`)
}

function idsKey(prefix: string, relation: number, field: number): StoreKey {
	return assemble(prefix, `ids/${relation.toString(16).padStart(8, "0")}/${field.toString(16).padStart(4, "0")}`)
}

function tenantPrefix(root: string, tenant: string): string {
	return assemble(root, `t/${tenant}`)
}

export type { Generation, StoreKey }
export {
	checkpointJsonKey,
	checkpointMdbKey,
	generation,
	hex16,
	idsKey,
	LOCK_SUFFIX,
	logKey,
	manifestKey,
	storeKey,
	tenantPrefix
}
