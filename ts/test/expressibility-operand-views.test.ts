import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import type { Db as DbValue } from "#index.ts"
import { closed, contained, Db, key, on, query, relation, renderStatement, schema, str, u64, v } from "#index.ts"
import { accepted } from "#test/accepted.ts"
import { put } from "#test/put.ts"

function byPos<T extends { readonly pos: bigint }>(left: T, right: T): number {
	if (left.pos < right.pos) {
		return -1
	}
	if (left.pos > right.pos) {
		return 1
	}
	return 0
}

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-operand-"))
const storeDir = path.join(tmpRoot, "store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

function must<T>(value: T | undefined): T {
	assert.ok(value !== undefined, "expected a present value")
	return value
}

const Kind = closed("Kind", ["Taught", "Reviewed", "Enrichment"])
const Grp = relation("Grp", { id: u64.fresh, name: str })
const Prog = relation("Prog", { id: u64.fresh, grp: u64 })
const Capsule = relation("Capsule", { id: u64.fresh, title: str })
const Capability = relation("Capability", { id: u64.fresh, text: str })
const Member = relation("Member", { id: u64.fresh, program: u64, capsule: u64, pos: u64, kind: Kind.id })
const Teaches = relation("Teaches", { capsule: u64, capability: u64 })
const TransferRange = relation("TransferRange", { capsule: u64, floor: str, ceiling: str })
const ExitCondition = relation("ExitCondition", { capsule: u64, condition: str })
const NonExampleBoundary = relation("NonExampleBoundary", { capsule: u64, nearMiss: u64 })

const taughtHasTeaches = contained(on(Member.where({ kind: "Taught" }), "capsule"), on(Teaches, "capsule"))
const taughtHasTransferRange = contained(on(Member.where({ kind: "Taught" }), "capsule"), on(TransferRange, "capsule"))
const taughtHasExitCondition = contained(on(Member.where({ kind: "Taught" }), "capsule"), on(ExitCondition, "capsule"))
const taughtHasNonExampleBoundary = contained(
	on(Member.where({ kind: "Taught" }), "capsule"),
	on(NonExampleBoundary, "capsule")
)

const OperandViews = schema(
	"OperandViews",
	{ Kind, Grp, Prog, Capsule, Capability, Member, Teaches, TransferRange, ExitCondition, NonExampleBoundary },
	[
		contained(on(Prog, "grp"), on(Grp, "id")),
		contained(on(Member, "program"), on(Prog, "id")),
		contained(on(Member, "capsule"), on(Capsule, "id")),
		contained(on(Member, "kind"), on(Kind, "id")),

		key(Member, ["program", "pos"]),

		key(Teaches, ["capsule"]),
		contained(on(Teaches, "capsule"), on(Capsule, "id")),
		contained(on(Teaches, "capability"), on(Capability, "id")),
		key(TransferRange, ["capsule"]),
		contained(on(TransferRange, "capsule"), on(Capsule, "id")),
		key(ExitCondition, ["capsule"]),
		contained(on(ExitCondition, "capsule"), on(Capsule, "id")),
		key(NonExampleBoundary, ["capsule"]),
		contained(on(NonExampleBoundary, "capsule"), on(Capsule, "id")),
		contained(on(NonExampleBoundary, "nearMiss"), on(Capability, "id")),

		taughtHasTeaches,
		taughtHasTransferRange,
		taughtHasExitCondition,
		taughtHasNonExampleBoundary
	]
)

const taughtContract = query(OperandViews).rule(function taughtArm(r) {
	const { id: m, capsule: c, pos } = v(Member)
	const { title } = v(Capsule)
	const { capability: taught } = v(Teaches)
	const { text: taughtText } = v(Capability)
	const { floor, ceiling } = v(TransferRange)
	const { condition } = v(ExitCondition)
	const { nearMiss } = v(NonExampleBoundary)
	const { text: nearMissText } = v(Capability)
	return r
		.match(Member, { id: m, program: r.param("program"), capsule: c, pos, kind: "Taught" })
		.match(Capsule, { id: c, title })
		.match(Teaches, { capsule: c, capability: taught })
		.match(Capability, { id: taught, text: taughtText })
		.match(TransferRange, { capsule: c, floor, ceiling })
		.match(ExitCondition, { capsule: c, condition })
		.match(NonExampleBoundary, { capsule: c, nearMiss })
		.match(Capability, { id: nearMiss, text: nearMissText })
		.find({ m, c, pos, title, taught, taughtText, floor, ceiling, condition, nearMiss, nearMissText })
})

const restMembers = query(OperandViews).rule(function restArm(r) {
	const { id: m, capsule: c, pos, kind } = v(Member)
	return r
		.match(Member, { id: m, program: r.param("program"), capsule: c, pos, kind })
		.where(r.ne(kind, "Taught"))
		.find({ m, c, pos, kind })
})

const restExplicit = query(OperandViews).rule(function restByArray(r) {
	const { id: m, capsule: c, pos } = v(Member)
	return r
		.match(Member, { id: m, program: r.param("program"), capsule: c, pos, kind: ["Reviewed", "Enrichment"] })
		.find({ m, c, pos })
})

const restUnion = query(OperandViews)
	.rule(function reviewedArm(r) {
		const { id: m, capsule: c, pos } = v(Member)
		return r
			.match(Member, { id: m, program: r.param("program"), capsule: c, pos, kind: "Reviewed" })
			.find({ m, c, pos })
	})
	.rule(function enrichmentArm(r) {
		const { id: m, capsule: c, pos } = v(Member)
		return r
			.match(Member, { id: m, program: r.param("program"), capsule: c, pos, kind: "Enrichment" })
			.find({ m, c, pos })
	})

const CAPABILITIES = 40
const CAPSULES = 60
const MEMBERS_PER_PROGRAM = 150

function kindOf(i: number): "Taught" | "Reviewed" | "Enrichment" {
	if (i % 3 === 0) {
		return "Taught"
	}
	return i % 3 === 1 ? "Reviewed" : "Enrichment"
}

describe("expressibility: the primer's prompt-operand view as rules and laws", function suite() {
	let db: DbValue<(typeof OperandViews)["relations"]>
	let progA = 0n
	let progB = 0n

	let capsuleIds: readonly bigint[] = []
	let capabilityIds: readonly bigint[] = []

	test("the theory admits, and one commit seeds ~560 rows (contracts stitched same-commit)", async function seed() {
		db = accepted(await Db.create(storeDir, OperandViews))
		assert.equal(
			renderStatement(taughtHasTransferRange),
			"Member(capsule | kind == Taught) <= TransferRange(capsule)",
			"the kind-conditional inclusion is one canonical statement"
		)
		const landed = db.write(function seedAll(tx) {
			const caps: bigint[] = []
			for (let k = 0; k < CAPABILITIES; k++) {
				caps.push(put(tx, Capability, { text: `cap-text-${k}` }).id)
			}
			const capsules: bigint[] = []
			for (let j = 0; j < CAPSULES; j++) {
				capsules.push(put(tx, Capsule, { title: `capsule-${j}` }).id)
			}

			for (let j = 0; j < CAPABILITIES; j++) {
				const capsule = must(capsules[j])
				put(tx, Teaches, { capsule, capability: must(caps[j]) })
				put(tx, TransferRange, { capsule, floor: `floor-${j}`, ceiling: `ceiling-${j}` })
				put(tx, ExitCondition, { capsule, condition: `exit-${j}` })
				put(tx, NonExampleBoundary, { capsule, nearMiss: must(caps[(j + 1) % CAPABILITIES]) })
			}
			const grp = put(tx, Grp, { name: "estate" })
			const a = put(tx, Prog, { grp: grp.id })
			const b = put(tx, Prog, { grp: grp.id })
			for (const program of [a.id, b.id]) {
				for (let i = 0; i < MEMBERS_PER_PROGRAM; i++) {
					const kind = kindOf(i)

					const capsule = kind === "Taught" ? must(capsules[i % CAPABILITIES]) : must(capsules[i % CAPSULES])
					put(tx, Member, { program, capsule, pos: BigInt(i), kind })
				}
			}
			progA = a.id
			progB = b.id
			capsuleIds = capsules
			capabilityIds = caps
		})
		assert.equal(
			landed.tag,
			"accepted",
			"the seed commit satisfies all four kind-conditional inclusions at final state"
		)
	})

	test("Q1: the 8-way conjunctive rule prepares and answers correctly", function multiWay() {
		const prepared = db.prepare(taughtContract)
		const rows = db.read((i) => i.execute(prepared, { program: progA }))
		assert.equal(rows.length, MEMBERS_PER_PROGRAM / 3, "one contract row per Taught member of program A")

		const first = must(
			rows.find(function atPosZero(row) {
				return row.pos === 0n
			})
		)
		assert.equal(first.c, must(capsuleIds[0]))
		assert.equal(first.title, "capsule-0")
		assert.equal(first.taught, must(capabilityIds[0]))
		assert.equal(first.taughtText, "cap-text-0")
		assert.equal(first.floor, "floor-0")
		assert.equal(first.ceiling, "ceiling-0")
		assert.equal(first.condition, "exit-0")
		assert.equal(first.nearMiss, must(capabilityIds[1]))
		assert.equal(first.nearMissText, "cap-text-1", "Capability joined TWICE decodes both texts")

		assert.equal(db.read((i) => i.execute(prepared, { program: progB })).length, MEMBERS_PER_PROGRAM / 3)
	})

	test("Q1: the planning cliff is the occurrence cap — 21 atoms refuse typed at prepare, 8 has headroom", function cliff() {
		const wide = query(OperandViews).rule(function twentyOneAtoms(r) {
			const { id: c, title: t0 } = v(Capsule)
			let chain = r.match(Capsule, { id: c, title: t0 })
			for (let i = 1; i <= 20; i++) {
				const { title } = v(Capsule)
				chain = chain.match(Capsule, { id: c, title })
			}
			return chain.find({ c })
		})
		assert.throws(function overCap() {
			db.prepare(wide)
		}, /21 atom occurrences exceed the planner cap/)
	})

	test("Q2 candidate A (SANCTIONED): one prepared query per kind-arm, host-concatenated", function armPerKind() {
		const taught = db.read((i) => i.execute(db.prepare(taughtContract), { program: progA }))
		const rest = db.read((i) => i.execute(db.prepare(restMembers), { program: progA }))
		assert.equal(taught.length, 50)
		assert.equal(rest.length, 100)
		for (const row of rest) {
			assert.notEqual(row.kind, "Taught", "ne(kind, Taught) really excludes the Taught arm")
		}

		const ids = new Set<bigint>([...taught.map((row) => row.m), ...rest.map((row) => row.m)])
		assert.equal(ids.size, MEMBERS_PER_PROGRAM, "concatenation covers program A member for member")

		const project = function project(rows: readonly { m: bigint; c: bigint; pos: bigint }[]) {
			return rows.map((row) => ({ m: row.m, c: row.c, pos: row.pos })).sort(byPos)
		}
		assert.deepEqual(project(db.read((i) => i.execute(db.prepare(restExplicit), { program: progA }))), project(rest))
		assert.deepEqual(project(db.read((i) => i.execute(db.prepare(restUnion), { program: progA }))), project(rest))
	})

	test("Q2 candidate B (REFUSED): a union head cannot drop the contract columns — absence is unrepresentable", function unionWall() {
		assert.throws(function mixedHeads() {
			query(OperandViews)
				.rule(function taughtHead(r) {
					const { id: m, capsule: c, pos } = v(Member)
					const { capability: taught } = v(Teaches)
					return r
						.match(Member, { id: m, program: r.param("program"), capsule: c, pos, kind: "Taught" })
						.match(Teaches, { capsule: c, capability: taught })
						.find({ m, pos, taught })
				})
				.rule(function bareHead(r) {
					const { id: m, capsule: c, pos, kind } = v(Member)
					return r
						.match(Member, { id: m, program: r.param("program"), capsule: c, pos, kind })
						.where(r.ne(kind, "Taught"))
						.find({ m, pos })
				})
		}, /every rule of a query derives the same head/)
	})

	test("Q3: a Taught member whose capsule lacks a sidecar REFUSES at commit — the law replaces the host throw", function totalityRefusal() {
		const rejected = db.write(function missingTransferRange(tx) {
			const capsule = put(tx, Capsule, { title: "capsule-partial" })

			put(tx, Teaches, { capsule: capsule.id, capability: must(capabilityIds[0]) })
			put(tx, ExitCondition, { capsule: capsule.id, condition: "exit-partial" })
			put(tx, NonExampleBoundary, { capsule: capsule.id, nearMiss: must(capabilityIds[1]) })
			put(tx, Member, { program: progA, capsule: capsule.id, pos: 1000n, kind: "Taught" })
		})
		assert.equal(rejected.tag, "rejected", "the kind-conditional inclusion judges the commit")
		const violation = must(
			rejected.violations.find(function byStatement(entry) {
				return entry.statement === taughtHasTransferRange
			})
		)
		assert.equal(violation.kind, "containment")
		assert.strictEqual(violation.statement, taughtHasTransferRange, "the violation names the law by identity")
	})

	test("Q3: deleting a sidecar out from under a surviving Taught member refuses (targetRequired)", function targetSide() {
		const rejected = db.write(function stripContract(tx) {
			assert.equal(
				tx.delete(TransferRange, [{ capsule: must(capsuleIds[0]), floor: "floor-0", ceiling: "ceiling-0" }]).changed,
				1n
			)
		})
		assert.equal(rejected.tag, "rejected", "capsule-0 backs live Taught members — its contract cannot be stripped")
		const violation = must(
			rejected.violations.find(function byStatement(entry) {
				return entry.statement === taughtHasTransferRange
			})
		)
		assert.equal(violation.kind, "containment")
		assert.equal(violation.direction, "targetRequired")
	})

	test("Q3: the compliant same-commit stitch lands, and a non-Taught member on a bare capsule stays legal", function compliant() {
		const stitched = db.write(function fullContract(tx) {
			const capsule = put(tx, Capsule, { title: "capsule-whole" })
			put(tx, Teaches, { capsule: capsule.id, capability: must(capabilityIds[2]) })
			put(tx, TransferRange, { capsule: capsule.id, floor: "floor-whole", ceiling: "ceiling-whole" })
			put(tx, ExitCondition, { capsule: capsule.id, condition: "exit-whole" })
			put(tx, NonExampleBoundary, { capsule: capsule.id, nearMiss: must(capabilityIds[3]) })
			put(tx, Member, { program: progA, capsule: capsule.id, pos: 1001n, kind: "Taught" })
		})
		assert.equal(stitched.tag, "accepted", "member and contract commit together — totality holds at final state")

		const bare = db.write(function bareCapsuleMember(tx) {
			put(tx, Member, { program: progA, capsule: must(capsuleIds[CAPSULES - 1]), pos: 1002n, kind: "Reviewed" })
		})
		assert.equal(bare.tag, "accepted", "the ψ selection scopes the law to Taught rows only")
	})

	test("Q4: answers are sets — the host sorts; two executions agree", function ordering() {
		const prepared = db.prepare(restMembers)
		const first = [...db.read((i) => i.execute(prepared, { program: progB }))].sort(byPos)
		const second = [...db.read((i) => i.execute(prepared, { program: progB }))].sort(byPos)
		assert.equal(first.length, 100)
		for (let i = 1; i < first.length; i++) {
			assert.ok(
				must(first[i - 1]).pos < must(first[i]).pos,
				"pos strictly ascends — key(Member, [program, pos]) makes the comparator total"
			)
		}
		assert.deepEqual(second, first, "two executions sort to one sequence — deterministic")
	})
})

describe("primer cycle detector: rec reach(x,x) on a DAG is empty", function primerCycle() {
	const State = closed("State", ["Upheld", "Broken"])
	const Node = relation("Grp", { id: u64.fresh })
	const Produces = relation("Produces", { grp: u64, capability: u64 })
	const Requires = relation("Requires", { consumer: u64, capability: u64, state: State.id })
	const Primer = schema("Primer", { State, Grp: Node, Produces, Requires }, [
		contained(on(Produces, "grp"), on(Node, "id")),
		contained(on(Requires, "consumer"), on(Node, "id")),
		contained(on(Requires, "state"), on(State, "id"))
	])

	const requiresCycleQuery = query(Primer)
		.reach("reach", {
			base: [
				function edge(r) {
					const { grp: from, capability: cap } = v(Produces)
					const { consumer: to } = v(Requires)
					return r
						.match(Produces, { grp: from, capability: cap })
						.match(Requires, { consumer: to, capability: cap, state: "Upheld" })
						.where(r.ne(from, to))
						.find({ from, to })
				}
			],
			rec: [
				function step(r) {
					const { grp: from, capability: cap } = v(Produces)
					const midReq = v(Requires)
					const { consumer: to } = v(Requires)
					return r
						.match(Produces, { grp: from, capability: cap })
						.match(Requires, { consumer: midReq.consumer, capability: cap, state: "Upheld" })
						.match(Requires, { consumer: to, state: "Upheld" })
						.where(r.ne(from, midReq.consumer))
						.interior("reach", { from: midReq.consumer, to })
						.find({ from, to })
				}
			]
		})
		.rule(function diagonal(r) {
			const { id: node } = v(Node)
			return r.match(Node, { id: node }).interior("reach", { from: node, to: node }).find({ node })
		})

	test("empty answers on a DAG — the lattice has no cycle", async function dagIsEmpty() {
		const primerDir = path.join(tmpRoot, "primer-store")
		const db = accepted(await Db.create(primerDir, Primer))
		const seeded = db.write(function seedDag(tx) {
			const a = put(tx, Node, {})
			const b = put(tx, Node, {})
			put(tx, Produces, { grp: a.id, capability: 1n })
			put(tx, Requires, { consumer: b.id, capability: 1n, state: "Upheld" })
		})
		assert.equal(seeded.tag, "accepted", "a one-edge DAG lands")
		const rows = db.read((i) => i.execute(db.prepare(requiresCycleQuery), {}))
		assert.deepEqual(rows, [], "empty answers = DAG")
	})
})
