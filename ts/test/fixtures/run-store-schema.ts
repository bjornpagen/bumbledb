/**
 * Test fixture: an ORDER-EXACT replica of the graph-builder run-store
 * theory (`primer/src/tools/graph-builder/store/schema.ts`, PRD-09) — the
 * 17-newtype, 22-relation, 8-closed-relation schema the driver rides.
 * Declaration order is copied verbatim because order IS fingerprint
 * identity (materialized order pins statement ids): if this fixture admits,
 * fingerprints deterministically across processes, and reopens, the real
 * schema does too. Consumed by `consumer-patterns.test.ts` and the
 * `reopen-child.ts` subprocess.
 */

import type { Statement } from "#index.ts"
import { bytes, closed, contained, exactly, key, none, on, relation, schema, str, u64, window } from "#index.ts"

const toiTypeHandles = [
	"NonComparative",
	"RegularNoun",
	"HigherOrderNoun",
	"Comparative",
	"SingleTransformation",
	"CorrelatedFeatures",
	"CognitiveRoutine",
	"ReviewIntegration",
	"FactSystem"
] as const

type ToiTypeName = (typeof toiTypeHandles)[number]

const ToiType = closed("ToiType", toiTypeHandles)

const programKindHandles = [
	"coordinate_set_program",
	"hierarchy_program",
	"cognitive_routine_program",
	"transformation_program",
	"fact_system_program"
] as const

type ProgramKindName = (typeof programKindHandles)[number]

const ProgramKind = closed("ProgramKind", programKindHandles)

const MemberKind = closed("MemberKind", ["Taught", "ReviewIntegration"])

const TaskKind = closed("TaskKind", ["Enrich", "Cartograph", "Author", "Realize", "ReviewEdge", "Supervise"])

const Pin = closed("Pin", ["FableXhigh", "Gpt56Max", "FableXhighViaIntegrity", "Gpt56MaxViaIntegrity"])

const Outcome = closed("Outcome", ["Accepted", "Rejected", "Refused", "MismatchServed"])

const SteerKind = closed("SteerKind", ["Reissue", "Repartition", "PinBump", "Observe"])

const diagKindHandles = [
	"DuplicateGrade",
	"DuplicateUnit",
	"DuplicateObjectiveRef",
	"ObjectiveReused",
	"DuplicateStrandEdge",
	"DuplicateSpine",
	"DuplicateProgramForGroup",
	"DuplicateCapsuleRef",
	"CapsuleReused",
	"DuplicatePosition",
	"DuplicateReviewedTarget",
	"DuplicateProgramEdge",
	"DuplicateEdge",
	"DuplicateScheduleCapsule",
	"DuplicateScheduleRank",
	"DuplicateTask",
	"DuplicateAttempt",
	"DuplicateAttemptText",
	"DuplicateVerdict",
	"DuplicateReceipt",
	"UnknownSheet",
	"UnknownUnit",
	"UnknownGrp",
	"UnknownObjective",
	"UnknownProgram",
	"UnknownCapsule",
	"UnknownTask",
	"UnknownAttempt",
	"ReviewHolderNotReview",
	"UnknownReviewTarget",
	"OutOfVocabulary",
	"OrphanObjective",
	"OrphanCapsule",
	"CognitiveRoutineFormOutsideRoutineProgram",
	"HigherOrderNounOutsideHierarchyProgram",
	"FactSystemFormOutsideFactSystemProgram",
	"HierarchyMemberType",
	"TransformationMemberType",
	"HierarchyMissingHigherOrderEntry",
	"HierarchyHigherOrderCount",
	"CognitiveRoutineIntegratorCount",
	"CycleDetected",
	"RedundantEdge",
	"SelfLoop",
	"RankOrderViolation",
	"PositionGap",
	"TerminalFormViolation",
	"ToiMismatch",
	"ReviewTargetSameProgram",
	"ReviewTargetNotReviewHolder",
	"ReviewTargetEarlier",
	"ReviewMinTwoTargets",
	"EmptyProgram",
	"TransformationMissingRule",
	"GroupWithoutProgram",
	"CapsuleUnscheduled",
	"StrandEdgeUnrealized",
	"CourseMetaCardinality",
	"source_parse_error",
	"decode_error",
	"program_unit_program_count",
	"ref_namespace",
	"EmptyContractField",
	"mismatch_served"
] as const

const DiagKind = closed("DiagKind", diagKindHandles)

const CourseMetaId = u64.newtype("CourseMetaId")
const SheetId = u64.newtype("SheetId")
const UnitId = u64.newtype("UnitId")
const ObjectiveId = u64.newtype("ObjectiveId")
const GrpId = u64.newtype("GrpId")
const StrandEdgeId = u64.newtype("StrandEdgeId")
const ProgramId = u64.newtype("ProgramId")
const CapsuleId = u64.newtype("CapsuleId")
const MemberId = u64.newtype("MemberId")
const ReviewId = u64.newtype("ReviewId")
const ProgramEdgeId = u64.newtype("ProgramEdgeId")
const EdgeId = u64.newtype("EdgeId")
const TaskId = u64.newtype("TaskId")
const AttemptId = u64.newtype("AttemptId")
const DiagnosticId = u64.newtype("DiagnosticId")
const SteerId = u64.newtype("SteerId")
const ReceiptId = u64.newtype("ReceiptId")

const courseMeta = relation("courseMeta", {
	id: CourseMetaId.fresh,
	courseId: bytes(16),
	label: str,
	description: str
})

const sheet = relation("sheet", {
	id: SheetId.fresh,
	name: str,
	grade: str,
	contentHash: bytes(32)
})

const unit = relation("unit", {
	id: UnitId.fresh,
	sheet: SheetId,
	sourceUnitId: str,
	title: str,
	description: str,
	scope: str
})

const objective = relation("objective", {
	id: ObjectiveId.fresh,
	sheet: SheetId,
	unit: UnitId,
	ref: str,
	goal: str
})

const grp = relation("grp", {
	id: GrpId.fresh,
	sheet: SheetId,
	label: str,
	context: str
})

const grpMember = relation("grpMember", {
	grp: GrpId,
	objective: ObjectiveId
})

const strandEdge = relation("strandEdge", {
	id: StrandEdgeId.fresh,
	fromGrp: GrpId,
	toGrp: GrpId
})

const spine = relation("spine", {
	grp: GrpId
})

const program = relation("program", {
	id: ProgramId.fresh,
	grp: GrpId,
	kind: ProgramKind.id
})

const capsule = relation("capsule", {
	id: CapsuleId.fresh,
	program: ProgramId,
	ref: str,
	toi: ToiType.id,
	taughtClaim: str,
	priorAssumption: str,
	exitCondition: str,
	transferRange: str
})

const member = relation("member", {
	id: MemberId.fresh,
	program: ProgramId,
	capsule: CapsuleId,
	pos: u64,
	kind: MemberKind.id,
	toi: ToiType.id
})

const review = relation("review", {
	id: ReviewId.fresh,
	member: MemberId,
	target: MemberId
})

const programEdge = relation("programEdge", {
	id: ProgramEdgeId.fresh,
	fromProgram: ProgramId,
	toProgram: ProgramId
})

const edge = relation("edge", {
	id: EdgeId.fresh,
	fromCapsule: CapsuleId,
	toCapsule: CapsuleId
})

const schedule = relation("schedule", {
	capsule: CapsuleId,
	rank: u64
})

const task = relation("task", {
	id: TaskId.fresh,
	kind: TaskKind.id,
	sheet: SheetId,
	subject: u64
})

const attempt = relation("attempt", {
	id: AttemptId.fresh,
	task: TaskId,
	n: u64,
	pin: Pin.id,
	promptHash: bytes(32)
})

const attemptText = relation("attemptText", {
	attempt: AttemptId,
	prompt: str,
	output: str
})

const verdict = relation("verdict", {
	attempt: AttemptId,
	outcome: Outcome.id
})

const diagnostic = relation("diagnostic", {
	id: DiagnosticId.fresh,
	attempt: AttemptId,
	kind: DiagKind.id,
	path: str,
	message: str
})

const steer = relation("steer", {
	id: SteerId.fresh,
	kind: SteerKind.id,
	task: TaskId,
	note: str
})

const receipt = relation("receipt", {
	id: ReceiptId.fresh,
	sheet: SheetId,
	contentHash: bytes(32),
	courseId: bytes(16)
})

const laws = Object.freeze({
	sheetGradeKey: key(sheet, ["grade"]),
	unitSourceKey: key(unit, ["sheet", "sourceUnitId"]),
	objectiveRefKey: key(objective, ["ref"]),
	grpMemberObjectiveKey: key(grpMember, ["objective"]),
	strandEdgePairKey: key(strandEdge, ["fromGrp", "toGrp"]),
	spineGrpKey: key(spine, ["grp"]),
	programGrpKey: key(program, ["grp"]),
	capsuleRefKey: key(capsule, ["ref"]),
	memberCapsuleKey: key(member, ["capsule"]),
	memberPositionKey: key(member, ["program", "pos"]),
	reviewTargetPairKey: key(review, ["member", "target"]),
	programEdgePairKey: key(programEdge, ["fromProgram", "toProgram"]),
	edgePairKey: key(edge, ["fromCapsule", "toCapsule"]),
	scheduleCapsuleKey: key(schedule, ["capsule"]),
	scheduleRankKey: key(schedule, ["rank"]),
	taskIdentityKey: key(task, ["kind", "subject"]),
	attemptSequenceKey: key(attempt, ["task", "n"]),
	attemptTextKey: key(attemptText, ["attempt"]),
	verdictKey: key(verdict, ["attempt"]),
	receiptSheetKey: key(receipt, ["sheet"]),

	unitSheetRef: contained(on(unit, "sheet"), on(sheet, "id")),
	objectiveSheetRef: contained(on(objective, "sheet"), on(sheet, "id")),
	objectiveUnitRef: contained(on(objective, "unit"), on(unit, "id")),
	grpSheetRef: contained(on(grp, "sheet"), on(sheet, "id")),
	grpMemberGrpRef: contained(on(grpMember, "grp"), on(grp, "id")),
	grpMemberObjectiveRef: contained(on(grpMember, "objective"), on(objective, "id")),
	strandEdgeFromRef: contained(on(strandEdge, "fromGrp"), on(grp, "id")),
	strandEdgeToRef: contained(on(strandEdge, "toGrp"), on(grp, "id")),
	spineGrpRef: contained(on(spine, "grp"), on(grp, "id")),
	programGrpRef: contained(on(program, "grp"), on(grp, "id")),
	capsuleProgramRef: contained(on(capsule, "program"), on(program, "id")),
	memberProgramRef: contained(on(member, "program"), on(program, "id")),
	memberCapsuleRef: contained(on(member, "capsule"), on(capsule, "id")),
	programEdgeFromRef: contained(on(programEdge, "fromProgram"), on(program, "id")),
	programEdgeToRef: contained(on(programEdge, "toProgram"), on(program, "id")),
	edgeFromRef: contained(on(edge, "fromCapsule"), on(capsule, "id")),
	edgeToRef: contained(on(edge, "toCapsule"), on(capsule, "id")),
	scheduleCapsuleRef: contained(on(schedule, "capsule"), on(capsule, "id")),
	taskSheetRef: contained(on(task, "sheet"), on(sheet, "id")),
	attemptTaskRef: contained(on(attempt, "task"), on(task, "id")),
	attemptTextAttemptRef: contained(on(attemptText, "attempt"), on(attempt, "id")),
	verdictAttemptRef: contained(on(verdict, "attempt"), on(attempt, "id")),
	diagnosticAttemptRef: contained(on(diagnostic, "attempt"), on(attempt, "id")),
	steerTaskRef: contained(on(steer, "task"), on(task, "id")),
	receiptSheetRef: contained(on(receipt, "sheet"), on(sheet, "id")),

	programKindVocab: contained(on(program, "kind"), on(ProgramKind, "id")),
	capsuleToiVocab: contained(on(capsule, "toi"), on(ToiType, "id")),
	memberKindVocab: contained(on(member, "kind"), on(MemberKind, "id")),
	memberToiVocab: contained(on(member, "toi"), on(ToiType, "id")),
	taskKindVocab: contained(on(task, "kind"), on(TaskKind, "id")),
	attemptPinVocab: contained(on(attempt, "pin"), on(Pin, "id")),
	verdictOutcomeVocab: contained(on(verdict, "outcome"), on(Outcome, "id")),
	diagnosticKindVocab: contained(on(diagnostic, "kind"), on(DiagKind, "id")),
	steerKindVocab: contained(on(steer, "kind"), on(SteerKind, "id")),

	partitionTotality: window(on(objective, "id"), exactly(1n), on(grpMember, "objective")),
	capsuleTotality: window(on(capsule, "id"), exactly(1n), on(member, "capsule")),

	hierarchyParentCount: window(
		on(program.where({ kind: ProgramKind.hierarchy_program }), "id"),
		exactly(1n),
		on(member.where({ toi: ToiType.HigherOrderNoun }), "program")
	),
	routineIntegratorCount: window(
		on(program.where({ kind: ProgramKind.cognitive_routine_program }), "id"),
		exactly(1n),
		on(member.where({ toi: ToiType.CognitiveRoutine }), "program")
	),

	reviewHolderIsReviewIntegration: contained(
		on(review, "member"),
		on(member.where({ kind: MemberKind.ReviewIntegration }), "id")
	),
	reviewTargetIsMember: contained(on(review, "target"), on(member, "id"))
})

interface MisplacedFormBan {
	readonly programKind: ProgramKindName
	readonly toi: ToiTypeName
	readonly statement: Statement
}

function misplacedFormBan(programKind: ProgramKindName, toi: ToiTypeName): MisplacedFormBan {
	return Object.freeze({
		programKind,
		toi,
		statement: window(
			on(program.where({ kind: ProgramKind[programKind] }), "id"),
			none,
			on(member.where({ toi: ToiType[toi] }), "program")
		)
	})
}

const misplacedFormBans: readonly MisplacedFormBan[] = Object.freeze([
	misplacedFormBan("coordinate_set_program", "CognitiveRoutine"),
	misplacedFormBan("hierarchy_program", "CognitiveRoutine"),
	misplacedFormBan("transformation_program", "CognitiveRoutine"),
	misplacedFormBan("fact_system_program", "CognitiveRoutine"),
	misplacedFormBan("coordinate_set_program", "HigherOrderNoun"),
	misplacedFormBan("cognitive_routine_program", "HigherOrderNoun"),
	misplacedFormBan("transformation_program", "HigherOrderNoun"),
	misplacedFormBan("fact_system_program", "HigherOrderNoun"),
	misplacedFormBan("coordinate_set_program", "FactSystem"),
	misplacedFormBan("hierarchy_program", "FactSystem"),
	misplacedFormBan("cognitive_routine_program", "FactSystem"),
	misplacedFormBan("transformation_program", "FactSystem"),
	misplacedFormBan("hierarchy_program", "NonComparative"),
	misplacedFormBan("hierarchy_program", "Comparative"),
	misplacedFormBan("hierarchy_program", "SingleTransformation"),
	misplacedFormBan("hierarchy_program", "CorrelatedFeatures"),
	misplacedFormBan("transformation_program", "NonComparative"),
	misplacedFormBan("transformation_program", "RegularNoun"),
	misplacedFormBan("transformation_program", "Comparative"),
	misplacedFormBan("transformation_program", "CorrelatedFeatures")
])

interface EntryFormBan {
	readonly toi: Exclude<ToiTypeName, "HigherOrderNoun">
	readonly statement: Statement
}

const entryFormBans: readonly EntryFormBan[] = Object.freeze(
	toiTypeHandles
		.filter(function notHigherOrder(handle): handle is Exclude<ToiTypeName, "HigherOrderNoun"> {
			return handle !== "HigherOrderNoun"
		})
		.map(function banAtEntry(toi): EntryFormBan {
			return Object.freeze({
				toi,
				statement: window(
					on(program.where({ kind: ProgramKind.hierarchy_program }), "id"),
					none,
					on(member.where({ pos: 1n, toi: ToiType[toi] }), "program")
				)
			})
		})
)

const runStoreStatements: readonly Statement[] = Object.freeze([
	...Object.values(laws),
	...misplacedFormBans.map(function banStatement(ban) {
		return ban.statement
	}),
	...entryFormBans.map(function banStatement(ban) {
		return ban.statement
	})
])

const runStoreSchema = schema(
	"graphBuilderRun",
	{
		ToiType,
		ProgramKind,
		MemberKind,
		TaskKind,
		Pin,
		Outcome,
		SteerKind,
		DiagKind,
		courseMeta,
		sheet,
		unit,
		objective,
		grp,
		grpMember,
		strandEdge,
		spine,
		program,
		capsule,
		member,
		review,
		programEdge,
		edge,
		schedule,
		task,
		attempt,
		attemptText,
		verdict,
		diagnostic,
		steer,
		receipt
	},
	runStoreStatements
)

type RunStoreSchema = typeof runStoreSchema

export type { RunStoreSchema }
export {
	attempt,
	attemptText,
	capsule,
	courseMeta,
	DiagKind,
	diagnostic,
	edge,
	entryFormBans,
	grp,
	grpMember,
	laws,
	MemberKind,
	member,
	misplacedFormBans,
	Outcome,
	objective,
	Pin,
	ProgramKind,
	program,
	programEdge,
	receipt,
	review,
	runStoreSchema,
	SteerKind,
	schedule,
	sheet,
	spine,
	steer,
	strandEdge,
	TaskKind,
	ToiType,
	task,
	unit,
	verdict
}
