/**
 * The key layout: generation numbers zero-padded lowercase hex, 16
 * chars; a prefix is a store; a tenant is a prefix under `t/`. A
 * StoreKey is parsed here, once — the verbs take the proof and never
 * re-check. An empty prefix joins as the rest alone, so a leading
 * slash is unrepresentable. Temps and leases live under a reserved
 * tilde-family first segment no StoreKey can spell — ASCII `~` and
 * its lookalikes — so those namespaces stay disjoint from every
 * honest key. Format characters, line and paragraph separators, and
 * space separators cannot hide a reserved prefix or a `.lock` suffix.
 */

import * as errors from "@superbuilders/errors"
import { U64_MAX, utf8Encoder } from "#bytes.ts"
import type { Braid } from "#descriptor.ts"

/** A segment wearing this suffix — after format characters are stripped — is not a key. */
const LOCK_SUFFIX = ".lock"

/** Reserved first-segment names no StoreKey can spell. Temps and leases live here. */
const TEMP_NAMESPACE = "~tmp"
const LEASE_NAMESPACE = "~lease"

/** ASCII tilde and lookalikes that can spell a reserved first segment. */
const TILDE_FAMILY = new Set([
	"~",
	"\u02DC",
	"\u02F7",
	"\u1FC0",
	"\u2053",
	"\u223C",
	"\u223D",
	"\u223F",
	"\u2E1E",
	"\u2E1F",
	"\u301C",
	"\u3030",
	"\uFE4B",
	"\uFE4F",
	"\uFF5E"
])

const FORMAT_OR_SEPARATOR = /[\p{Cc}\p{Cf}\p{Zl}\p{Zp}\p{Zs}]/u
const FORMAT_CHARS = /\p{Cf}/gu

declare const storeKeyBrand: unique symbol
type StoreKey = string & { readonly [storeKeyBrand]: typeof storeKeyBrand }

declare const generationBrand: unique symbol
type Generation = bigint & { readonly [generationBrand]: typeof generationBrand }

function firstCodePoint(seg: string): string | undefined {
	for (const ch of seg) {
		return ch
	}
	return undefined
}

function tildeFamilyPrefix(seg: string): boolean {
	const first = firstCodePoint(seg)
	if (first === undefined) {
		return false
	}
	return TILDE_FAMILY.has(first) || first.normalize("NFKC") === "~"
}

function stripFormat(seg: string): string {
	return seg.replace(FORMAT_CHARS, "")
}

/** One path segment of a key or a tenant id: the same grammar. */
function segmentOk(seg: string): boolean {
	if (seg.length === 0 || seg.includes("/") || seg === "." || seg === "..") {
		return false
	}
	if (FORMAT_OR_SEPARATOR.test(seg)) {
		return false
	}
	const stripped = stripFormat(seg)
	if (stripped.length === 0 || stripped === "." || stripped === "..") {
		return false
	}
	return !tildeFamilyPrefix(stripped) && !stripped.endsWith(LOCK_SUFFIX)
}

function storeKey(raw: string): StoreKey {
	const wellFormed = raw.length > 0 && !raw.startsWith("/") && !raw.endsWith("/") && raw.split("/").every(segmentOk)
	if (!wellFormed) {
		throw errors.new(`store key is not a slash path: ${raw}`)
	}
	return raw as StoreKey
}

/** Empty, or a StoreKey spelling (the same segment grammar, no leading or trailing slash). */
function parsePrefix(raw: string): string {
	if (raw.length === 0) {
		return ""
	}
	return storeKey(raw.replace(/^\/+|\/+$/g, ""))
}

function isReservedName(name: string): boolean {
	return tildeFamilyPrefix(stripFormat(name))
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
		throw errors.new(`reserved name is not a slash path: ${raw}`)
	}
	const first = segs[0]
	if (first !== TEMP_NAMESPACE && first !== LEASE_NAMESPACE) {
		throw errors.new(`reserved name is not under ${TEMP_NAMESPACE} or ${LEASE_NAMESPACE}: ${raw}`)
	}
	return raw
}

/** Known scratch-lease document under `~lease`. One path, no LIST. */
const CKPT_SCRATCH_LEASE = "ckpt-scratch"

/** The reserved scratch name: `~lease/ckpt-scratch`. */
function scratchCkptName(): string {
	return reservedName(`${LEASE_NAMESPACE}/${CKPT_SCRATCH_LEASE}`)
}

/** The scratch-lease body: one version line, then the digest. */
function encodeCkptScratch(digest: string): Uint8Array {
	if (!/^[0-9a-f]{64}$/.test(digest)) {
		throw errors.new(`checkpoint digest is not 64 lowercase hex: ${digest}`)
	}
	return utf8Encoder.encode(`CKPT-SCRATCH/1\n${digest}\n`)
}

/** The digest a scratch-lease body names, or null. */
function scratchCkptDigest(bytes: Uint8Array): string | null {
	const text = new TextDecoder().decode(bytes)
	const match = /^CKPT-SCRATCH\/1\n([0-9a-f]{64})\n$/.exec(text)
	const digest = match?.[1]
	return digest === undefined ? null : digest
}

function parseCkptScratch(bytes: Uint8Array): string | null {
	return scratchCkptDigest(bytes)
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
	if (!segmentOk(tenant)) {
		throw errors.new(`tenant id is not a single path segment: ${tenant}`)
	}
	return assemble(root, `t/${tenant}`)
}

export type { Generation, StoreKey }
export {
	CKPT_SCRATCH_LEASE,
	checkpointJsonKey,
	checkpointMdbKey,
	encodeCkptScratch,
	generation,
	hex16,
	idsKey,
	isReservedName,
	LEASE_NAMESPACE,
	LOCK_SUFFIX,
	logKey,
	manifestKey,
	parseCkptScratch,
	parsePrefix,
	reservedLease,
	reservedName,
	reservedTemp,
	scratchCkptDigest,
	scratchCkptName,
	segmentOk,
	storeKey,
	TEMP_NAMESPACE,
	tenantPrefix
}
