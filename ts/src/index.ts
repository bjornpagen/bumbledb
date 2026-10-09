/**
 * @bjornpagen/bumbledb — the Effect-native TypeScript SDK for the
 * bumbledb embedded relational engine. Pure
 * schema/query/scalar construction is synchronous metadata; all work is
 * lazy, scoped and bounded on the one native runtime. No Promise, sync,
 * or disposal twin. The raw native bridge is not exported from this barrel.
 */

export { alternatives } from "./alternatives.ts"
export type {
	BoundsOnTarget,
	CapacityWeight,
	CapacityWindow,
	DurationRef,
	FieldRef,
	UnitDimensionBan,
	WeightOnSource
} from "./capacity.ts"
export { duration, ref, weigh, within } from "./capacity.ts"
export type { ChangeCounts, ChangeDraft, ChangeRecord } from "./changes.ts"
export { ChangeSet } from "./changes.ts"
export type {
	AnyClosed,
	AxiomRow,
	Axioms,
	Closed,
	PayloadField
} from "./closed.ts"
export { closed, closedId } from "./closed.ts"
export type { RowShape } from "./codec.ts"
export {
	decodeBoundaryField,
	decodeBoundaryRows,
	decodeRows,
	encodeBoundaryField,
	encodeBoundaryRows,
	encodeRows,
	fieldSchema,
	rowSchema,
	rowShape
} from "./codec.ts"
export type { SchemaId } from "./compile.ts"
export type {
	Consistency,
	DatabaseOptions,
	DatabasePool,
	DatabaseReader,
	PoolOptions,
	SubmitOptions,
	SubmitOutcome,
	Tuning
} from "./database/database.ts"
export { Database, RequestId } from "./database/database.ts"
export { FsStore } from "./database/fs.ts"
export type {
	Body,
	Bucket,
	ExecutorOptions,
	IoRequest,
	IoResponse,
	IoResult,
	Millis,
	ObjectStore,
	Reply,
	Target
} from "./database/io.ts"
export type { Fault, MemStoreOptions } from "./database/mem.ts"
export { MemStore } from "./database/mem.ts"
export type { AnyMigration, Migrations, Populate } from "./database/migration.ts"
export { Migration } from "./database/migration.ts"
export type { S3StoreOptions } from "./database/s3.ts"
export { S3Store } from "./database/s3.ts"
export type { PreparedQuery, QueryReader } from "./db.ts"
export type { CloseReport, OutstandingWork } from "./errors.ts"
export {
	AuthoringDiagnostic,
	AuthoringError,
	CloseFailure,
	DbError,
	dbError,
	NativeLoadError
} from "./errors.ts"
export type {
	AnyFace,
	Arity,
	Face,
	FaceArityMismatch,
	FaceFields,
	FaceOwner,
	FaceShapeMismatch,
	FaceShapes,
	FaceSource,
	OwnerOf,
	ProjectedShape,
	SameArity,
	SameShapes
} from "./face.ts"
export { on } from "./face.ts"
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
} from "./fields.ts"
export { bool, bytes, f64, i64, interval, str, u64, uuid } from "./fields.ts"
export type { Same, SameLen } from "./judgment.ts"
export type { ClassesOf, ClassWall, LawfulStatements, RelationClasses, SchemaClasses } from "./law.ts"
export type {
	EvidenceOut,
	FactOut,
	HeadOut,
	MigrationOut,
	OutcomeOut,
	ReceiptOut,
	RefusalOut,
	SettledOut,
	ViolationOut as Violation
} from "./native/binding.d.ts"
export type { FindColumn } from "./query/atom.ts"
export { ALLEN } from "./query/atom.ts"
export type { AnyComputeExpr, ComputeExpr, ComputeValue, QueryNode } from "./query/compute.ts"
export { Compute } from "./query/compute.ts"
export type { Agg, HeadRecordOf, RowOfFind } from "./query/find.ts"
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
} from "./query/lower.ts"
export { lowerQuery, query } from "./query/lower.ts"
export type {
	ClassedField,
	Flatten,
	ImportedFieldVar,
	MatchFields,
	MatchOwner,
	Param,
	ParamEntry,
	ParamsRecord,
	SetParam,
	Var,
	VarsOf
} from "./query/scope.ts"
export { v } from "./query/scope.ts"
export type { IntervalVar, SegmentOp, Segments } from "./query/segments.ts"
export type {
	AnyRelation,
	Fact,
	FieldsShape,
	Relation,
	RelationField,
	RelationFields
} from "./relation.ts"
export { relation } from "./relation.ts"
export type { CompleteResult } from "./result.ts"
export type { CellValue } from "./rows.ts"
export { cellOf, factOfCells, flatRowsOf, keyCellsOf } from "./rows.ts"
export type { BumbleOptions } from "./runtime.ts"
export { Bumble } from "./runtime.ts"
export type {
	IntervalKind,
	NumericCast,
	Rounding,
	ScalarInputKind,
	ScalarKind,
	ScalarLiteral,
	ScalarNode,
	ScalarResultKind,
	ScalarValue
} from "./scalar.ts"
export type { AnySchema, Schema, SchemaRelation, SchemaRelations } from "./schema.ts"
export { schema } from "./schema.ts"
export type { AnySelected, FieldsOf, Selected, SelectionBinding, SelectionInput } from "./selection.ts"
export { select } from "./selection.ts"
export type { Key, QueryTemplate, Rel } from "./shape.ts"
export type { CapacityBoundSpec, CapacityWindowSpec, LiteralSetSpec, LiteralSpec, ValueSpec } from "./spec.ts"
export type {
	CapacityStatement,
	ContainmentStatement,
	KeyStatement,
	MirrorsStatement,
	Statement
} from "./statements.ts"
export { capacity, contained, key, mirrors, renderStatement } from "./statements.ts"
export { Uuid } from "./uuid.ts"
