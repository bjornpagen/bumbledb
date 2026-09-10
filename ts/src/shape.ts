import type { Query } from "#query/lower.ts"
import type { ParamsRecord } from "#query/scope.ts"
/**
 * Shared derived types: `S` is a
 * declared core schema value's type, `Rel<S>` its ordinary (writable)
 * relations, `Fact<R>` the inferred row object, `Key<K>` the exact value
 * of a declared key accepted by `QueryReader.get`, and `QueryTemplate<S, P, A>` the
 * immutable schema-bound logical query template. All are derived from the
 * existing typed descriptors — no second hand-maintained roster.
 */
import type { AnyRelation, Fact } from "#relation.ts"
import type { AnySchema } from "#schema.ts"
import type { KeyStatement } from "#statements.ts"

/** The ordinary relations of a schema (closed vocabularies are ground axioms, never ingestion targets). */
type Rel<S extends AnySchema> = Extract<S["relations"][keyof S["relations"]], AnyRelation>

/**
 * The structural value of one declared key, derived from its owner's
 * fields and its exact projection. Every selected field is required.
 */
type Key<K extends KeyStatement<AnyRelation, readonly string[]>> = Pick<
	Fact<K["owner"]>,
	Extract<K["projection"][number], keyof Fact<K["owner"]>>
>

/** Immutable typed query template. */
type QueryTemplate<S extends AnySchema, P extends ParamsRecord, A> = Query<S["relations"], A, P>

export type { Key, QueryTemplate, Rel }
