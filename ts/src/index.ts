/**
 * @bjornpagen/bumbledb — the type-theoretic TypeScript SDK for the
 * bumbledb embedded relational engine. Public surface: the structural type
 * kernel (fields as pure structure, `relation()`, `closed()` — domains are
 * never declared: THE LAWS TYPE THE COLUMNS, `schema()` computing every
 * field's equivalence class FROM the statement list at both tiers), the
 * statement algebra with `schema()` and `SchemaSpec` lowering (PRD-06), the `Db`
 * runtime (path-cached stores, transactions, typed violations, scoped
 * snapshot reads, the witnessed write loop with `abandon` — PRD-07, zero
 * closables), the query surface (Datalog as values, kysely-shaped:
 * `query(S).rule(r => { const { id, name } = v(Holder); return r.match(Holder, { id, name }).find({ name }) })` —
 * variables minted by `v()` and joined by OBJECT REFERENCE (reuse is the
 * join), the head a `find` RECORD whose keys name the answer columns
 * (renames are real), params still STRING-named, plus negation,
 * conditions, aggregates, and stratified recursion via `program()`/`rec` —
 * `db.prepare` as a plain value; the comparison/connective builders are
 * also free exports, and the free names `eq`/`not`/`and`/`or` collide with
 * common host identifiers — import aliasing is the answer; the SDK does
 * not rename for collision-avoidance), the exhume surface
 * (`Db.exhume` — the one schema-independent read path: the store's
 * self-described shapes and raw facts by name, typed at bare structural
 * values, deliberately schema-free), and the answer-ordering helpers
 * (`by`/`desc` — sort keys as data for the language's own `.sort`; answers
 * are sets, the engine never orders, and limit is the language's own
 * `.slice`). The raw native bridge is not exported.
 */

export type {
	AnyClosed,
	AnySelectedClosed,
	AxiomRow,
	Axioms,
	Closed,
	ClosedColumn,
	ClosedCore,
	ClosedData,
	ClosedRow,
	ClosedSelectionInput,
	PayloadField,
	SelectedClosed
} from "#closed.ts"
export { closed } from "#closed.ts"
export type { Count } from "#count.ts"
export { atLeast, atMost, between, exactly, none } from "#count.ts"
export type {
	Abandon,
	DeclaredKeyFact,
	DeltaBuild,
	MemberRelation,
	OffendingFact,
	Prepared,
	ReadScope,
	Tx,
	Violation,
	WitnessedWriteResult,
	WriteResult
} from "#db.ts"
export { abandon, Db, ErrNewtypeMismatch, ErrWitnessedLivelock, WITNESSED_ATTEMPT_CAP } from "#db.ts"
export type {
	Exhumed,
	ExhumedAxiom,
	ExhumedDescriptor,
	ExhumedFact,
	ExhumedField,
	ExhumedRelation
} from "#exhume.ts"
export {
	ErrExhumeCorruption,
	ErrExhumeFormatMismatch,
	ErrExhumeNoDescriptor
} from "#exhume.ts"
export type {
	AnyFace,
	Arity,
	Face,
	FaceArityMismatch,
	FaceData,
	FaceFields,
	FaceOwner,
	FaceShapeMismatch,
	FaceShapes,
	FaceSource,
	OwnerOf,
	SameArity,
	SameShapes
} from "#face.ts"
export { on } from "#face.ts"
export type {
	AnyField,
	BoolField,
	BytesField,
	ClosedIdField,
	ClosedRoster,
	FreshU64Field,
	I64Field,
	Infer,
	IntervalField,
	IntervalValue,
	StrField,
	U64Field
} from "#fields.ts"
export { bool, bytes, i64, interval, span, str, u64 } from "#fields.ts"
export type { ClassesOf, ClassWall, LawfulStatements, RelationClasses, SchemaClasses } from "#law.ts"
export { lower, lowerClosed, lowerRelation } from "#lower.ts"
export type { KeyFact, Minted } from "#marshal.ts"
export type {
	FactValue,
	OccurrenceDrift,
	ProgramIr,
	Staleness,
	StatementKindTag
} from "#native.ts"
export type { Desc, SortKey } from "#order.ts"
export { by, desc } from "#order.ts"

export type {
	AnyCond,
	BindingInput,
	Cmp,
	FindColumn,
	MatchShape,
	NotAtom,
	RecData,
	RuleData,
	Tree
} from "#query/atom.ts"
export { ALLEN, allen, and, eq, ge, gt, le, lt, ne, not, or, pointIn } from "#query/atom.ts"
export type { Agg, FindEntry } from "#query/find.ts"
export type {
	AnyQuery,
	AnyRuleValue,
	OutputRuleChain,
	OutputRuleScope,
	Query,
	QueryData,
	QueryParams,
	QueryRelation,
	QueryRow,
	QueryRuleChain,
	QueryRuleScope,
	QueryStart,
	RecRef,
	RecRuleChain,
	RecRuleScope,
	RuleValue,
	TermOps
} from "#query/lower.ts"
export { lowerQuery, query } from "#query/lower.ts"
export type { ProgramScope, Rec } from "#query/predicate.ts"
export { program } from "#query/predicate.ts"
export type {
	ClassedField,
	Duration,
	MaskParam,
	MatchFields,
	MatchOwner,
	Param,
	ParamEntry,
	ParamsRecord,
	SetParam,
	Var,
	VarsOf
} from "#query/scope.ts"
export { v } from "#query/scope.ts"
export type {
	AnyRelation,
	AnySelected,
	Fact,
	FieldsShape,
	FreshKeys,
	InsertFact,
	Relation,
	RelationData,
	RelationField,
	RelationFields,
	Selected,
	SelectionBinding,
	SelectionInput
} from "#relation.ts"
export { relation } from "#relation.ts"
export type { AnySchema, Schema, SchemaRelation, SchemaRelations } from "#schema.ts"
export { schema } from "#schema.ts"
export type {
	FieldSpec,
	LiteralSetSpec,
	LiteralSpec,
	RelationSpec,
	RowSpec,
	SchemaSpec,
	SideSpec,
	StatementSpec,
	ValueSpec,
	ValueTypeSpec,
	WindowSpec
} from "#spec.ts"
export { renderLiteral, renderLiteralSet, renderWindow } from "#spec.ts"
export type {
	ContainedStatement,
	ContainmentData,
	KeyData,
	KeyStatement,
	Statement,
	StatementData,
	WindowData,
	WindowStatement
} from "#statements.ts"
export { contained, key, mirrors, renderStatement, window } from "#statements.ts"
