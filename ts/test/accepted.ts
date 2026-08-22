function accepted<T>(admission: { readonly tag: string; readonly value?: T }): T {
	if (admission.tag !== "accepted" || admission.value === undefined) {
		throw new Error(`expected accepted admission, got ${admission.tag}`)
	}
	return admission.value
}

export { accepted }
