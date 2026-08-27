import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as path from "node:path"
import { describe, test } from "node:test"
import type {
	Admission,
	LogBatchDecodeKind,
	LogBatchEncodeKind,
	LogCheckpointKind,
	LogManifestKind,
	LogSidecarKind,
	SchemaRelations
} from "@bjornpagen/bumbledb"
import * as errors from "@superbuilders/errors"
import type { LeaseRefusal, RefusalCause } from "#errors.ts"
import { ErrRefused, ErrStore, refusalOf, refuse, wrapStore } from "#errors.ts"
import type { RefreshOutcome, Waited } from "#replica.ts"

/** The generated identity table: the core's one speller per family. */
const TABLE_PATH = path.resolve(import.meta.dirname, "../../crates/bumbledb-log/conformance/v3/identities.json")

/** The bridge marshal's mint table, the same emission checked in twice. */
const MINT_PATH = path.resolve(import.meta.dirname, "../../ts/crate/log-identities.json")

const tableJson: Record<string, unknown> = JSON.parse(fs.readFileSync(TABLE_PATH, "utf8"))

function rowsOf(family: string): readonly string[] {
	const rows = tableJson[family]
	assert.ok(Array.isArray(rows), `identities.json carries the ${family} family`)
	return rows.map(function rowOf(row): string {
		assert.equal(typeof row, "string", `${family} rows are identity strings`)
		return String(row)
	})
}

/**
 * One key per `RefusalCause` arm, in declaration order — the compile
 * side of the lock: an arm without a key here, or a key without an
 * arm, refuses to typecheck, the TS analog of the core's exhaustive
 * witness match.
 */
const REFUSAL_CAUSE_KINDS: Record<RefusalCause["kind"], true> = {
	Truncated: true,
	BadMagic: true,
	Version: true,
	Flags: true,
	FingerprintMismatch: true,
	UnknownBraid: true,
	UnknownOpKind: true,
	UnknownRelation: true,
	ClosedRelation: true,
	OpRelationOutsideBraid: true,
	TagMismatch: true,
	BoolByte: true,
	InvalidUtf8: true,
	EmptyInterval: true,
	IntervalOverflow: true,
	TrailingBytes: true,
	Arity: true,
	Value: true,
	TooManyOps: true,
	TooManyRows: true,
	Malformed: true,
	Overflow: true,
	BraidSet: true,
	Counter: true,
	DigestWidth: true,
	CheckpointDigest: true,
	NoOpSlot: true
}

/**
 * The kinds with no identity-table row: `DigestWidth` is the codec
 * seat's short-digest gate (the corpus pins the name at
 * `batch/r_encode_short_prev`); `CheckpointDigest` and `NoOpSlot` are
 * the replica machine's own refusals — the machines keep two executors
 * and no bridge family.
 */
const HOST_SIDE_KINDS: readonly RefusalCause["kind"][] = ["DigestWidth", "CheckpointDigest", "NoOpSlot"]

/** The bridge families whose rows must all carry a `RefusalCause` arm. */
const BRIDGE_FAMILIES = ["batchDecode", "batchEncode", "manifest", "checkpoint", "sidecar"] as const

/** The `counter` family, arm for arm, core declaration order. */
const LEASE_REFUSAL_KINDS: Record<LeaseRefusal["kind"], true> = {
	Counter: true,
	Exhausted: true,
	OverWidth: true
}

// The bridge's kind unions in `ts/src/native.ts`, one key per literal
// in core declaration order — the same compile lock, so a drifted
// union refuses to typecheck here and a drifted table fails below.
const BATCH_DECODE_KINDS: Record<LogBatchDecodeKind, true> = {
	Truncated: true,
	BadMagic: true,
	Version: true,
	Flags: true,
	FingerprintMismatch: true,
	UnknownBraid: true,
	UnknownOpKind: true,
	UnknownRelation: true,
	ClosedRelation: true,
	OpRelationOutsideBraid: true,
	TagMismatch: true,
	BoolByte: true,
	InvalidUtf8: true,
	EmptyInterval: true,
	IntervalOverflow: true,
	TrailingBytes: true
}

const BATCH_ENCODE_KINDS: Record<LogBatchEncodeKind, true> = {
	FingerprintMismatch: true,
	UnknownBraid: true,
	UnknownRelation: true,
	ClosedRelation: true,
	OpRelationOutsideBraid: true,
	Arity: true,
	Value: true,
	TooManyOps: true,
	TooManyRows: true
}

const MANIFEST_KINDS: Record<LogManifestKind, true> = {
	Malformed: true,
	Version: true
}

const CHECKPOINT_KINDS: Record<LogCheckpointKind, true> = {
	Malformed: true,
	Version: true,
	Overflow: true,
	UnknownBraid: true,
	BraidSet: true
}

const SIDECAR_KINDS: Record<LogSidecarKind, true> = {
	Malformed: true,
	Version: true,
	UnknownBraid: true,
	Overflow: true
}

// The outcome families: the lowercase arm tags the tagged hosts narrow.
const ADMISSION_TAGS: Record<Admission<SchemaRelations, unknown>["tag"], true> = {
	accepted: true,
	rejected: true
}

const WAITED_TAGS: Record<Waited["tag"], true> = {
	reached: true,
	wedged: true,
	refused: true
}

const REFRESH_OUTCOME_TAGS: Record<RefreshOutcome["tag"], true> = {
	advanced: true,
	wedged: true,
	reseed: true,
	refused: true
}

describe("the identity-table lock", function suite() {
	test("the bridge mint table is the conformance table, byte for byte", function twinTables() {
		assert.ok(
			fs.readFileSync(MINT_PATH).equals(fs.readFileSync(TABLE_PATH)),
			"ts/crate/log-identities.json and conformance/v3/identities.json are one emission"
		)
	})

	test("every RefusalCause kind is a table row or a pinned host-side kind", function everyArmIsARow() {
		const rows = new Set<string>()
		for (const family of [...BRIDGE_FAMILIES, "counter"]) {
			for (const row of rowsOf(family)) {
				rows.add(row)
			}
		}
		const hostSide = new Set<string>(HOST_SIDE_KINDS)
		for (const kind of Object.keys(REFUSAL_CAUSE_KINDS)) {
			assert.ok(
				rows.has(kind) || hostSide.has(kind),
				`RefusalCause kind ${kind} is a table row or a pinned host-side kind`
			)
		}
		for (const kind of hostSide) {
			assert.ok(!rows.has(kind), `host-side kind ${kind} stays outside the table`)
		}
	})

	test("every bridge-family row has a RefusalCause arm", function everyRowHasAnArm() {
		const causeKinds = new Set<string>(Object.keys(REFUSAL_CAUSE_KINDS))
		for (const family of BRIDGE_FAMILIES) {
			for (const row of rowsOf(family)) {
				assert.ok(causeKinds.has(row), `${family} row ${row} carries a RefusalCause arm`)
			}
		}
	})

	test("the counter family is LeaseRefusal, arm for arm, with the thrown Counter arm", function counterFamily() {
		assert.deepEqual(Object.keys(LEASE_REFUSAL_KINDS), [...rowsOf("counter")])
		assert.ok("Counter" in REFUSAL_CAUSE_KINDS, "the counter parse's thrown identity is a RefusalCause arm")
	})

	test("the bridge's kind unions match the table, row for row", function nativeUnions() {
		assert.deepEqual(Object.keys(BATCH_DECODE_KINDS), [...rowsOf("batchDecode")])
		assert.deepEqual(Object.keys(BATCH_ENCODE_KINDS), [...rowsOf("batchEncode")])
		assert.deepEqual(Object.keys(MANIFEST_KINDS), [...rowsOf("manifest")])
		assert.deepEqual(Object.keys(CHECKPOINT_KINDS), [...rowsOf("checkpoint")])
		assert.deepEqual(Object.keys(SIDECAR_KINDS), [...rowsOf("sidecar")])
	})

	test("the outcome families match the tagged hosts", function outcomeFamilies() {
		assert.deepEqual(Object.keys(ADMISSION_TAGS), [...rowsOf("admission")])
		assert.deepEqual(Object.keys(WAITED_TAGS), [...rowsOf("waited")])
		const refreshRows = rowsOf("refreshOutcome")
		const refreshTags = Object.keys(REFRESH_OUTCOME_TAGS)
		for (const row of refreshRows) {
			assert.ok(refreshTags.includes(row), `refreshOutcome row ${row} is a RefreshOutcome arm`)
		}
		const hostLocal = refreshTags.filter(function outsideTable(tag) {
			return !refreshRows.includes(tag)
		})
		assert.deepEqual(hostLocal.sort(), ["reseed", "wedged"], "the replica machine's own refresh arms, pinned")
	})
})

describe("the refusal identity", function suite() {
	test("refuse mints ErrRefused carrying its typed cause", function typedCause() {
		const caught = errors.trySync(function refuseIt() {
			return refuse({ kind: "BraidSet" }, "checkpoint braid set drifts from the derived braids")
		})
		assert.ok(caught.error, "refuse throws")
		assert.ok(errors.is(caught.error, ErrRefused), "the thrown chain names ErrRefused")
		assert.equal(refusalOf(caught.error)?.kind, "BraidSet")
	})

	test("a bare bridge kind is a complete cause; the detail rides the message", function bareKind() {
		const caught = errors.trySync(function refuseIt() {
			return refuse({ kind: "TooManyRows" }, "op 0 relation Booking carries 70000 rows")
		})
		assert.ok(caught.error, "refuse throws")
		const cause = refusalOf(caught.error)
		assert.ok(cause !== undefined, "the refusal carries its cause")
		assert.deepEqual(cause, { kind: "TooManyRows" })
		assert.ok(caught.error.message.includes("op 0 relation Booking carries 70000 rows"))
	})
})

describe("the store failure identity", function suite() {
	test("wrapStore puts the exported sentinel into the cause chain, matched by identity", function identity() {
		const vendor = new Error("EACCES: permission denied, open '/bucket/x'")
		const wrapped = wrapStore(vendor, "putCreate prod/main/log/c00000000/0000000000000001")
		assert.ok(errors.is(wrapped, ErrStore), "errors.is matches the exported sentinel by identity")
		assert.equal(errors.cause(wrapped), ErrStore, "the sentinel is the chain's root")
	})

	test("the vendor error's message rides the detail verbatim", function vendorMessage() {
		const vendor = new Error("ENOSPC: no space left on device")
		const wrapped = wrapStore(vendor, "putSwap prod/main/manifest")
		assert.ok(wrapped.message.includes("ENOSPC: no space left on device"))
		assert.ok(wrapped.message.includes("putSwap prod/main/manifest"))
		assert.ok(String(wrapped).includes(ErrStore.message), "the rendered chain names the store channel")
	})

	test("an unrelated error never matches the sentinel", function unrelated() {
		assert.equal(errors.is(errors.new("not a store failure"), ErrStore), false)
	})
})
