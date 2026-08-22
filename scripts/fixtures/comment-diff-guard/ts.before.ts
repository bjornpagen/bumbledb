export function answer(xs: number[]): number {
	// narration: fold the array
	const msg = "not a // comment";
	const tpl = `still not a /* comment */`;
	return xs.reduce((a, b) => a + b, 0); // trailing fold
}

/** Public contract sentence that will be tightened. */
export function name(s: string): string {
	return s;
}
