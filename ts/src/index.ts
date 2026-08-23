/**
 * @bjornpagen/bumbledb — the type-theoretic TypeScript SDK for the
 * bumbledb embedded relational engine. Public surface: the structural type
 * kernel (fields as pure structure, `relation`, `closed` — domains are
 * never declared: THE LAWS TYPE THE COLUMNS, `schema` computing every
 * field's equivalence class FROM the statement list at both tiers), the
 * statement algebra with `schema` and `SchemaSpec` lowering (PRD-06), the `Db`
 * runtime (exclusive-lock stores, transactions, typed violations, callback
 * instance reads, one-shot `write`/`writeFrom` with `abandon` — PRD-07), the query surface (kysely-shaped:
 * `query(S).rule(r => { const { id, name } = v(Holder); return r.match(Holder, { id, name }).find({ name }) })` —
 * variables minted by `v` and joined by OBJECT REFERENCE (reuse is the
 * join), the head a `find` RECORD whose keys name the answer columns
 * (renames are real), params still STRING-named, plus negation,
 * conditions, aggregates, and interiors / one linear rec via
 * `q.interior` / `q.reach` —
 * `db.prepare` as a plain value; comparisons and connectives live on
 * the rule scope). The raw native bridge is not exported.
 */

export type {
	BoundsOnTarget,
	CapacityWeight,
	CapacityWindow,
	DurationRef,
	FieldRef,
	UnitWindowBan,
	WeightOnSource
} from "#capacity.ts"
export { duration, ref, weigh, within } from "#capacity.ts"
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
export type {
	Abandon,
	AbandonedArm,
	Admission,
	CapacityViolation,
	Committed,
	ContainmentViolation,
	DeclaredKeyFact,
	DeclaredKeyViolation,
	DeltaBuild,
	FreshRange,
	ImpliedKeyViolation,
	MemberRelation,
	MirrorViolation,
	MutationReport,
	OffendingFact,
	OwnedInstance,
	Prepared,
	ReadInstance,
	SyncResult,
	Violation,
	Witness,
	WriteFromOutcome,
	WriteOutcome,
	WriteTx
} from "#db.ts"
export {
	abandon,
	Db,
	ErrAsyncCallback,
	ErrFingerprintMismatch,
	ErrForeignPrepared,
	ErrForeignWitness,
	ErrIrError,
	ErrNewtypeMismatch,
	ErrSchemaError,
	ErrSpentHandle,
	ErrUseAfterScope,
	InstanceBuilder
} from "#db.ts"
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
	ProjectedShape,
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
export { lower } from "#lower.ts"
export type { KeyFact } from "#marshal.ts"
export type { FactValue, ParsedQuery, QueryIr, StatementKindTag } from "#native.ts"
export { internalBlake3 } from "#native.ts"

export type {
	AnyCond,
	BindingInput,
	Cmp,
	FindColumn,
	InteriorData,
	MatchShape,
	NotAtom,
	RecData,
	RuleData,
	Tree
} from "#query/atom.ts"
export { ALLEN } from "#query/atom.ts"
export type { Agg, FindEntry } from "#query/find.ts"
export type {
	AnyQuery,
	AnyRuleValue,
	Query,
	QueryData,
	QueryParams,
	QueryReachStart,
	QueryRelation,
	QueryRow,
	QueryRuleChain,
	QueryRuleScope,
	QueryStart,
	RecRuleChain,
	RecRuleScope,
	RuleValue,
	TermOps
} from "#query/lower.ts"
export { lowerQuery, query } from "#query/lower.ts"
export type {
	ClassedField,
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
	CapacityBoundSpec,
	CapacityWindowSpec,
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
	WeightSpec
} from "#spec.ts"
export type {
	CapacityData,
	CapacityStatement,
	ContainedStatement,
	ContainmentData,
	KeyData,
	KeyStatement,
	Statement,
	StatementData
} from "#statements.ts"
export { capacity, contained, key, mirrors, renderStatement } from "#statements.ts"
