/**
 * @bjornpagen/bumbledb — the Effect-native TypeScript SDK for the
 * bumbledb embedded relational engine. Pure
 * schema/query/scalar construction is synchronous metadata; all work is
 * lazy, scoped and bounded on the one native runtime. No Promise, sync,
 * or disposal twin. The raw native bridge is not exported from this barrel.
 */

export type {
	BoundsOnTarget,
	CapacityWeight,
	CapacityWindow,
	DurationRef,
	FieldRef,
	UnitDimensionBan,
	WeightOnSource
} from "#capacity.ts"
export { duration, ref, weigh, within } from "#capacity.ts"
export type { ChangeDraft } from "#changes.ts"
export { ChangeSet } from "#changes.ts"
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
export type { RowShape } from "#codec.ts"
export { decodeBoundaryRows, decodeRows, encodeBoundaryRows, encodeRows, rowSchema, rowShape } from "#codec.ts"
export type { CompiledSchema, SchemaId } from "#compile.ts"
export { Schema } from "#compile.ts"
export type {
	ApplyExpected,
	ApplyOptions,
	ApplyOutcome,
	CoreWitness,
	DbInspection,
	PreparedQuery,
	QueryReader,
	Snapshot,
	StorageInspection
} from "#db.ts"
export { Db } from "#db.ts"
export {
	AuthoringError,
	NativeLoadError,
	NativeOperationError,
	NativeReportedError,
	SdkInvariantError
} from "#errors.ts"
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
	AnyClosedIdField,
	AnyClosedRoster,
	AnyField,
	BoolField,
	BytesField,
	ClosedHandleTuple,
	ClosedIdField,
	ClosedRoster,
	F64Field,
	FloatIntervalValue,
	I64Field,
	Infer,
	IntervalElementKind,
	IntervalField,
	IntervalValue,
	SignatureOf,
	StrField,
	U64Field,
	UuidField
} from "#fields.ts"
export { bool, bytes, f64, i64, interval, span, str, u64, uuid } from "#fields.ts"
export type { Same, SameLen } from "#judgment.ts"
export type { ClassesOf, ClassWall, LawfulStatements, RelationClasses, SchemaClasses } from "#law.ts"
export type { Violation, ViolationFact } from "#native.ts"
export type { FindColumn } from "#query/atom.ts"
export { ALLEN } from "#query/atom.ts"
export type { AnyComputeExpr, ComputeExpr, ComputeValue, QueryNode } from "#query/compute.ts"
export { Compute } from "#query/compute.ts"
export type { Agg, RowOfFind } from "#query/find.ts"
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
	Flatten,
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
	Relation,
	RelationData,
	RelationField,
	RelationFields,
	Selected,
	SelectionBinding,
	SelectionInput
} from "#relation.ts"
export { relation } from "#relation.ts"
export type { CompleteResult } from "#result.ts"
export type { CellValue } from "#rows.ts"
export { cellOf, factOfCells, flatRowsOf, keyCellsOf } from "#rows.ts"
export type { NativeRuntimeOptions } from "#runtime.ts"
export { NativeRuntime } from "#runtime.ts"
export type { CloseReport, OutstandingWork } from "#runtime-errors.ts"
export { CloseFailure, DbError, dbError, runtimeErrorCodes } from "#runtime-errors.ts"
export type {
	NumericCast,
	ScalarExpr,
	ScalarFieldRef,
	ScalarKind,
	ScalarLeafScope,
	ScalarLiteral,
	ScalarNode,
	ScalarResultKind,
	ScalarValue
} from "#scalar.ts"
export { Scalar } from "#scalar.ts"
export type { AnySchema, Schema as SchemaDeclaration, SchemaRelation, SchemaRelations } from "#schema.ts"
export { schema } from "#schema.ts"
export type { Key, QueryTemplate, Rel } from "#shape.ts"
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
	ValueTypeSpec
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
export { Uuid } from "#uuid.ts"
