import type { AnySchema } from "#schema.ts"
/**
 * Shared derived-type vocabulary of the chapter 35 core roster: `S` is a
 * declared core schema value's type, `Rel<S>` its ordinary (writable)
 * relations, `Fact<R>` the inferred row object, `Key<R>` the primary-key
 * object accepted by `QueryReader.get`, and `QueryTemplate<S, P, A>` the
 * immutable schema-bound logical query template. All are derived from the
 * existing typed descriptors — no second hand-maintained roster.
 */
import type { AnyRelation, Fact } from "#relation.ts"
import type { Query } from "#query/lower.ts"
import type { ParamsRecord } from "#query/scope.ts"

/** The ordinary relations of a schema (closed vocabularies are ground axioms, never ingestion targets). */
type Rel<S extends AnySchema> = Extract<S["relations"][keyof S["relations"]], AnyRelation>

/**
 * The primary-key object of a relation: the fields of its FIRST
 * materialized key statement (a closed relation's synthetic `(id)`,
 * otherwise the first declared `key`). Keys are declared statements now —
 * there is no fresh-implied key — so the exact projection is a schema
 * fact, verified at execution against the compiled tables; the type admits
 * any subset of the fact's own fields and the runtime refuses a mismatch
 * with a typed `DbError` before any native work.
 */
type Key<R extends AnyRelation> = Partial<Fact<R>>

/** Chapter 35's spelling of the immutable typed query template. */
type QueryTemplate<S extends AnySchema, P extends ParamsRecord, A> = Query<S["relations"], A, P>

export type { Key, QueryTemplate, Rel }
