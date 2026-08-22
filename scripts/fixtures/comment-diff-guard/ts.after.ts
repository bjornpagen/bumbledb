export function answer(xs: number[]): number {
	const msg = "not a // comment";
	const tpl = `still not a /* comment */`;
	return xs.reduce((a, b) => a + b, 0);
}

/** Semantics the signature cannot carry. */
export function name(s: string): string {
	return s;
}
