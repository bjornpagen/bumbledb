/** The two database buckets of one stage. Every name derives from the stage so a stage is torn down whole. */
export const region = "us-east-1"
export const zone = "use1-az4"

export function stage(): string {
	const value = process.env.ALCHEMY_STAGE
	if (value === undefined || !/^[a-z0-9-]{1,20}$/.test(value)) {
		throw new Error("ALCHEMY_STAGE must be 1-20 of [a-z0-9-]")
	}
	return value
}

export function buckets(name: string) {
	return {
		log: `bumbledb-${name}-log--${zone}--x-s3`,
		checkpoints: `bumbledb-${name}-ckpt`
	}
}
