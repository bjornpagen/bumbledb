/**
 * The type-level judgment kernel. Two primitives, no imports, no runtime.
 *
 * {@link Same} is definitional equality by mutual extension — the ONE
 * spelling of `A = B` at the type tier. {@link SameLen} is Peano equality
 * on tuple lengths — zero equals zero, successor recurses on successor,
 * everything else is refused. An open array carries no Nat (its length is
 * `number`), so it proves nothing.
 */

type Same<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false

type SameLen<A extends readonly unknown[], B extends readonly unknown[]> = number extends A["length"]
	? false
	: number extends B["length"]
		? false
		: A extends readonly [unknown, ...infer ARest]
			? B extends readonly [unknown, ...infer BRest]
				? SameLen<ARest, BRest>
				: false
			: B extends readonly [unknown, ...unknown[]]
				? false
				: true

export type { Same, SameLen }
