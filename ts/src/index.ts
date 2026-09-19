export type { AlgebraicRootDescription } from "#algebraic-root.ts"
export { AlgebraicRoot } from "#algebraic-root.ts"
export type {
	BetaExpectationObservation,
	BetaIntegral,
	BetaObservation,
	BetaProbabilityObservation,
	BetaSourceDescription
} from "#beta-source.ts"
export { BetaSource } from "#beta-source.ts"
export { Event } from "#event.ts"
export { ExactRational } from "#exact.ts"
export type { FamilyFunctionDescription, FamilyFunctionPiece } from "#family-function.ts"
export { FamilyFunction } from "#family-function.ts"
export type { FamilyKernelDescription } from "#family-kernel.ts"
export { FamilyKernel } from "#family-kernel.ts"
export type { FamilyJeffreyTarget } from "#family-revision.ts"
export { FamilyRevision } from "#family-revision.ts"
export type { FamilyRevisionInspection, FamilyRevisionOutcome, FamilyRevisionReceipt } from "#family-revision-data.ts"
export type { FiniteFunctionDescription, FunctionPiece } from "#finite-function.ts"
export { FiniteFunction } from "#finite-function.ts"
export type { FiniteKernelDescription, SourceExtension } from "#finite-kernel.ts"
export { FiniteKernel } from "#finite-kernel.ts"
export type { FunctionCover, FunctionPatch } from "#function-cover.ts"
export type { ParameterFunctionDescription, ParameterFunctionPiece } from "#parameter-function.ts"
export { ParameterFunction } from "#parameter-function.ts"
export type { CommonParameterSource, ParameterRefinementDescription } from "#parameter-refinement.ts"
export { ParameterRefinement } from "#parameter-refinement.ts"
export type { ParameterRegionDescription, RealWitness } from "#parameter-region.ts"
export { ParameterDomain, ParameterRegion, PolynomialSigns } from "#parameter-region.ts"
export type { ParameterRestrictionDescription } from "#parameter-restriction.ts"
export { ParameterRestriction } from "#parameter-restriction.ts"
export type {
	ParameterExpectationObservation,
	ParameterGuard,
	ParameterProbabilityObservation,
	ParameterSourceDescription,
	ParameterWorld,
	WorldCardinality
} from "#parameter-source.ts"
export type { ParameterBinding, PolynomialPower, PolynomialTerm } from "#polynomial.ts"
export { ExactPolynomial } from "#polynomial.ts"
export type { ExpectationObservation, ProbabilityObservation } from "#source-operation.ts"
export type { JeffreyTarget } from "#source-revision.ts"
export { SourceRevision } from "#source-revision.ts"
export type { RevisionOutcome, RevisionReceipt, SourceRevisionInspection } from "#source-revision-data.ts"
/**
 * @bjornpagen/bumbledb — the Effect-native TypeScript SDK for the
 * bumbledb embedded relational engine. Pure
 * schema/query/scalar construction is synchronous metadata; all work is
 * lazy, scoped and bounded on the one native runtime. No Promise, sync,
 * or disposal twin. The raw native bridge is not exported from this barrel.
 */

export { alternatives } from "#alternatives.ts"
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
export type { ChangeCounts, ChangeDraft, ChangeRecord } from "#changes.ts"
export { ChangeSet } from "#changes.ts"
export type {
	AnyClosed,
	AxiomRow,
	Axioms,
	Closed,
	PayloadField
} from "#closed.ts"
export { closed, closedId } from "#closed.ts"
export type { RowShape } from "#codec.ts"
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
} from "#codec.ts"
export type { CompiledSchema, SchemaId } from "#compile.ts"
export { Schema } from "#compile.ts"
export type {
	ApplyOutcome,
	CoreWitness,
	DbInspection,
	JudgeOutcome,
	PreparedQuery,
	QueryReader,
	Snapshot,
	StorageInspection,
	WriteExpected,
	WriteOptions
} from "#db.ts"
export { Db } from "#db.ts"
export {
	AuthoringDiagnostic,
	AuthoringError,
	NativeLoadError,
	NativeOperationError,
	NativeReportedError,
	SdkInvariantError
} from "#errors.ts"
export { EventDescriptor } from "#event-descriptor.ts"
export type {
	EventDescriptorDescription,
	EventDescriptorInspection,
	EventFibreDescription,
	EventMapDescription
} from "#event-descriptor-data.ts"
export { EventMemory } from "#event-memory.ts"
export type { EventMemoryDescription, EventMemoryInspection, EventMemoryTransition } from "#event-memory-data.ts"
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
	EventField,
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
export { bool, bytes, event, f64, i64, interval, str, u64, uuid } from "#fields.ts"
export type { Same, SameLen } from "#judgment.ts"
export type { ClassesOf, ClassWall, LawfulStatements, RelationClasses, SchemaClasses } from "#law.ts"
export type {
	AtomIr,
	AtomSourceIr,
	CmpOpIr,
	ComparisonIr,
	ConditionTreeIr,
	EventExprIr,
	EventTestIr,
	FindTermIr,
	HeadTermIr,
	InteriorIr,
	QueryIr,
	RecIr,
	RelationExprIr,
	RuleIr,
	ScalarExprIr,
	TermIr,
	Violation,
	ViolationFact
} from "#native.ts"
export { ObservationNumber } from "#observation-number.ts"
export { ObservationPredicate } from "#observation-predicate.ts"
export type { NonemptyProjection, ProjectionTerm } from "#projection.ts"
export type { FindColumn } from "#query/atom.ts"
export { ALLEN } from "#query/atom.ts"
export type { AnyComputeExpr, ComputeExpr, ComputeValue, QueryNode } from "#query/compute.ts"
export { Compute } from "#query/compute.ts"
export type {
	DescriptionHead,
	DescriptionParameter,
	DescriptionRow,
	DescriptionTable,
	QueryDescription
} from "#query/description.ts"
export { describeQuery, queryFromDescription } from "#query/description.ts"
export type { EventFind, EventOperand, EventVar } from "#query/event.ts"
export {
	EventExpr,
	EventTest,
	expectation,
	payoffRatio,
	probability,
	type RationalPayoff,
	RelationExpr
} from "#query/event.ts"
export type { ExpectationAnswer, ExpectationResult } from "#query/expectation.ts"
export { expectationResult } from "#query/expectation.ts"
export type { Agg, HeadRecordOf, RowOfFind } from "#query/find.ts"
export { Guard, type GuardContext, type GuardExpr, GuardPlan } from "#query/guard.ts"
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
export { NumberExpr, type NumberOperand } from "#query/number.ts"
export { type NumberAnswer, type NumberResult, numberResult } from "#query/number-result.ts"
export { PredicateExpr, type PredicateOperand, PredicateTest } from "#query/predicate.ts"
export { type PredicateAnswer, type PredicateResult, predicateResult } from "#query/predicate-result.ts"
export type { ProbabilityAnswer, ProbabilityResult } from "#query/probability.ts"
export { probabilityResult } from "#query/probability.ts"
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
} from "#query/scope.ts"
export { v } from "#query/scope.ts"
export type { IntervalVar, SegmentOp, Segments } from "#query/segments.ts"
export type {
	AnyRelation,
	Fact,
	FieldsShape,
	Relation,
	RelationField,
	RelationFields
} from "#relation.ts"
export { relation } from "#relation.ts"
export type { CompleteResult } from "#result.ts"
export type { CellValue } from "#rows.ts"
export { cellOf, factOfCells, flatRowsOf, keyCellsOf } from "#rows.ts"
export type { NativeRuntimeOptions } from "#runtime.ts"
export { NativeRuntime } from "#runtime.ts"
export type { CloseReport, EventOperandFault, OutstandingWork } from "#runtime-errors.ts"
export { CloseFailure, DbError, dbError, runtimeErrorCodes } from "#runtime-errors.ts"
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
} from "#scalar.ts"
export type { AnySchema, Schema as SchemaDeclaration, SchemaRelation, SchemaRelations } from "#schema.ts"
export { schema } from "#schema.ts"
export type { AnySelected, FieldsOf, Selected, SelectionBinding, SelectionInput } from "#selection.ts"
export { select } from "#selection.ts"
export type { Key, QueryTemplate, Rel } from "#shape.ts"
export type {
	CapacityBoundSpec,
	CapacityWindowSpec,
	FieldSpec,
	LiteralSetSpec,
	LiteralSpec,
	ProjectionSpec,
	RelationSpec,
	RowSpec,
	SchemaSpec,
	SideSpec,
	StatementSpec,
	ValueSpec,
	ValueTypeSpec
} from "#spec.ts"
export type {
	CapacityStatement,
	ContainmentStatement,
	KeyStatement,
	MirrorsStatement,
	Statement
} from "#statements.ts"
export { capacity, contained, key, mirrors, renderStatement } from "#statements.ts"
export { Uuid } from "#uuid.ts"
