/**
 * The key layout: generation numbers zero-padded lowercase hex, 16
 * chars; a prefix is a store; a tenant is a prefix under `t/`. A
 * StoreKey is parsed here, once — the verbs take the proof and never
 * re-check. An empty prefix joins as the rest alone, so a leading
 * slash is unrepresentable. Temps and leases live under a reserved
 * `~` first segment no StoreKey can spell, so those namespaces stay
 * disjoint from every honest key. Format characters, line and
 * paragraph separators, and space separators cannot hide a reserved
 * prefix or a `.lock` suffix: a segment containing one is refused
 * outright.
 */
import { internalLogParseCkptScratch, internalLogRenderCkptScratch } from "@bjornpagen/bumbledb"
import { regex } from "arkregex"
import type { Digest32 } from "#bytes.ts"
import { digest32, hex32, U64_MAX } from "#bytes.ts"
import type { Braid } from "#descriptor.ts"
import { LogInputError } from "#errors.ts"

/** A segment wearing this suffix is not a key. */
const LOCK_SUFFIX = ".lock"
/** Reserved first-segment names no StoreKey can spell. Temps and leases live here. */
const TEMP_NAMESPACE = "~tmp"
const LEASE_NAMESPACE = "~lease"
const FORMAT_OR_SEPARATOR = regex("[\\p{Cc}\\p{Cf}\\p{Zl}\\p{Zp}\\p{Zs}]", "u")
const SLASH_TRIM = regex("^/+|/+$", "g")
declare const storeKeyBrand: unique symbol
type StoreKey = string & {
	readonly [storeKeyBrand]: typeof storeKeyBrand
}
declare const generationBrand: unique symbol
type Generation = bigint & {
	readonly [generationBrand]: typeof generationBrand
}
/** One path segment of a key or a tenant id: the same grammar. */
function segmentOk(seg: string): boolean {
	if (seg.length === 0 || seg.includes("/") || seg === "." || seg === "..") {
		return false
	}
	if (FORMAT_OR_SEPARATOR.test(seg)) {
		return false
	}
	return !seg.startsWith("~") && !seg.endsWith(LOCK_SUFFIX)
}
function storeKey(raw: string): StoreKey {
	const wellFormed = raw.length > 0 && !raw.startsWith("/") && !raw.endsWith("/") && raw.split("/").every(segmentOk)
	if (!wellFormed) {
		throw new LogInputError({ message: `store key is not a slash path: ${raw}` })
	}
	return raw as StoreKey
}
/** Empty, or a StoreKey spelling (the same segment grammar, no leading or trailing slash). */
function parsePrefix(raw: string): string {
	if (raw.length === 0) {
		return ""
	}
	return storeKey(raw.replace(SLASH_TRIM, ""))
}
function reservedTemp(pid: number, seq: number): string {
	return `${TEMP_NAMESPACE}/${String(pid)}.${String(seq)}`
}
function reservedLease(key: string, token: bigint): string {
	return `${LEASE_NAMESPACE}/${key}/${String(token)}`
}
/** A slash path whose first segment is `~tmp` or `~lease`. Not a StoreKey. */
function reservedName(raw: string): string {
	const segs = raw.split("/")
	if (segs.length === 0 || segs.some((seg) => seg.length === 0 || seg === "." || seg === "..")) {
		throw new LogInputError({ message: `reserved name is not a slash path: ${raw}` })
	}
	const first = segs[0]
	if (first !== TEMP_NAMESPACE && first !== LEASE_NAMESPACE) {
		throw new LogInputError({ message: `reserved name is not under ${TEMP_NAMESPACE} or ${LEASE_NAMESPACE}: ${raw}` })
	}
	return raw
}
/** Known scratch-lease document under `~lease`. One path, no LIST. */
const CKPT_SCRATCH_LEASE = "ckpt-scratch"
/** The reserved scratch name: `~lease/ckpt-scratch`. */
function scratchCkptName(): string {
	return reservedName(`${LEASE_NAMESPACE}/${CKPT_SCRATCH_LEASE}`)
}
/** The scratch-lease body, spelled by the one grammar (`crates/bumbledb-log`). */
function encodeCkptScratch(digest: Digest32): Uint8Array {
	return internalLogRenderCkptScratch(digest)
}
/** The digest a scratch-lease body names, or null — the one grammar's
 *  parse, branded; the refusal is undifferentiated by law. */
function parseCkptScratch(bytes: Uint8Array): Digest32 | null {
	const named = internalLogParseCkptScratch(bytes)
	if (named === null) {
		return null
	}
	return digest32(named)
}
function generation(raw: bigint): Generation {
	if (raw < 0n || raw > U64_MAX) {
		throw new LogInputError({ message: `generation is a u64: ${raw}` })
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
	return assemble(prefix, "manifest")
}
function logKey(prefix: string, braid: Braid, g: Generation): StoreKey {
	return assemble(prefix, `log/${braid}/${hex16(g)}`)
}
function checkpointMdbKey(prefix: string, digest: Digest32): StoreKey {
	return assemble(prefix, `ckpt/${hex32(digest)}.mdb`)
}
function ckptDocKey(prefix: string, digest: Digest32): StoreKey {
	return assemble(prefix, `ckpt/${hex32(digest)}`)
}
function idsKey(prefix: string, relation: number, field: number): StoreKey {
	return assemble(prefix, `ids/${relation.toString(16).padStart(8, "0")}/${field.toString(16).padStart(4, "0")}`)
}
function tenantPrefix(root: string, tenant: string): string {
	if (!segmentOk(tenant)) {
		throw new LogInputError({ message: `tenant id is not a single path segment: ${tenant}` })
	}
	return assemble(root, `t/${tenant}`)
}

export type { Generation, StoreKey }
export {
	CKPT_SCRATCH_LEASE,
	checkpointMdbKey,
	ckptDocKey,
	encodeCkptScratch,
	generation,
	idsKey,
	LEASE_NAMESPACE,
	logKey,
	manifestKey,
	parseCkptScratch,
	parsePrefix,
	reservedLease,
	reservedName,
	reservedTemp,
	scratchCkptName,
	storeKey,
	TEMP_NAMESPACE,
	tenantPrefix
}
