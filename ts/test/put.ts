import { isFreshField } from "#marshal.ts"
import type { AnyRelation, Fact } from "#relation.ts"

type TestTx = {
	reserve(relation: AnyRelation, field: string, count: bigint): { at(index: bigint): bigint | undefined }
	insert(relation: AnyRelation, facts: Iterable<unknown>): unknown
}

function put<R extends AnyRelation>(tx: object, relation: R, partial: Record<string, unknown>): Fact<R> {
	const writer = tx as TestTx
	const fact: Record<string, unknown> = { ...partial }
	for (const declared of relation.data.fields) {
		if (isFreshField(declared.field) && fact[declared.name] === undefined) {
			const id = writer.reserve(relation, declared.name, 1n).at(0n)
			if (id === undefined) {
				throw new Error(`reserve(1) for ${relation.name}.${declared.name} returned empty`)
			}
			fact[declared.name] = id
		}
	}
	writer.insert(relation, [fact])
	return fact as Fact<R>
}

export { put }
