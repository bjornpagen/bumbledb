/**
 * @bjornpagen/bumbledb — the type-theoretic TypeScript SDK for the
 * bumbledb embedded relational engine. Public surface: the structural type
 * kernel (fields with schema-level domain labels, `relation()`,
 * `closed()`), the statement
 * algebra with `schema()` and `SchemaSpec` lowering (PRD-06), the `Db`
 * runtime (path-cached stores, transactions, typed violations, scoped
 * snapshot reads, the witnessed write loop with `abandon` — PRD-07, zero
 * closables), the query surface (Datalog as values, kysely-shaped:
 * `query(S).rule(r => r.match(...).where(...).select(...))` with
 * string-named domain-typed vars, params typed by use, negation,
 * conditions, aggregates, and stratified recursion via `program()`/`rec` —
 * `db.prepare` as a plain value), and the exhume surface
 * (`Db.exhume` — the one schema-independent read path: the store's
 * self-described shapes and raw facts by name, typed at bare structural
 * values, deliberately schema-free). The raw native bridge is not exported.
 */

export type {
	AnyClosed,
	AxiomRow,
	Axioms,
	Closed,
	ClosedColumn,
	ClosedCore,
	ClosedData,
	ClosedRow,
	PayloadField
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
export { abandon, Db } from "#db.ts"
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
	FaceDomainMismatch,
	FaceDomains,
	FaceFields,
	FaceOwner,
	FaceSource,
	OneOf,
	SameArity,
	SameDomains
} from "#face.ts"
export { on, oneOf } from "#face.ts"
export type {
	AnyField,
	BoolField,
	BytesCtor,
	BytesField,
	ClosedIdField,
	ClosedRoster,
	FreshU64Field,
	I64Ctor,
	I64Field,
	Infer,
	IntervalCtor,
	IntervalField,
	IntervalValue,
	StrField,
	U64Ctor,
	U64Field
} from "#fields.ts"
export { bool, bytes, i64, interval, span, str, u64 } from "#fields.ts"
export { lower, lowerClosed, lowerRelation } from "#lower.ts"
export type { KeyFact, Minted } from "#marshal.ts"
export type {
	FactValue,
	OccurrenceDrift,
	ProgramIr,
	Staleness,
	StatementKindTag
} from "#native.ts"

export type {
	AnyCond,
	BindingInput,
	Cmp,
	MatchShape,
	NotAtom,
	RecData,
	RuleData,
	SelectColumn,
	Tree
} from "#query/atom.ts"
export { ALLEN } from "#query/atom.ts"
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
export type { Duration, MaskParam, Param, ParamEntry, ParamsRecord, SetParam, Var } from "#query/scope.ts"
export type { Agg, SelectEntry } from "#query/select.ts"
export type {
	AnyRelation,
	AnySelected,
	Fact,
	FieldRef,
	FieldRefs,
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
export type { KeyData, KeyStatement, Statement, StatementData } from "#statements.ts"
export { contained, key, mirrors, renderStatement, window } from "#statements.ts"
