/**
 * The one production binding of the generator: `makeGenerator` over the
 * native `MigrationCodec`. The CLI (`#migrations/cli.ts`) and the direct API
 * (`#migrations/index.ts`) both import THESE values, so their outcomes are
 * identical by construction (TS-MIG-10) — there is no second generator
 * instance, codec or workflow.
 */
import { makeGenerator } from "#migrations/generate.ts"
import { productionCodec } from "#migrations/native.ts"

const production = makeGenerator(productionCodec)

export const generateMigrations = production.generateMigrations
export const checkMigrations = production.checkMigrations
