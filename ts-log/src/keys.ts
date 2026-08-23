/**
 * The key layout (10): generation numbers zero-padded lowercase hex,
 * 16 chars; a prefix is a store; a tenant is a prefix under `t/`.
 */

function hex16(g: bigint): string {
	return g.toString(16).padStart(16, "0")
}

function manifestKey(prefix: string): string {
	return `${prefix}/manifest.json`
}

function logKey(prefix: string, braid: string, g: bigint): string {
	return `${prefix}/log/${braid}/${hex16(g)}`
}

function checkpointMdbKey(prefix: string, digest: string): string {
	return `${prefix}/ckpt/${digest}.mdb`
}

function checkpointJsonKey(prefix: string, digest: string): string {
	return `${prefix}/ckpt/${digest}.json`
}

function idsKey(prefix: string, relation: number, field: number): string {
	return `${prefix}/ids/${relation.toString(16).padStart(8, "0")}/${field.toString(16).padStart(4, "0")}`
}

function tenantPrefix(root: string, tenant: string): string {
	return `${root}/t/${tenant}`
}

export { checkpointJsonKey, checkpointMdbKey, hex16, idsKey, logKey, manifestKey, tenantPrefix }
