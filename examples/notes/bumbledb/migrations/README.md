# Generated migration artifacts — PENDING F3 GENERATION

This directory receives the canonical generated chain for the staged
evolution history in `src/db/evolution-stages.ts`:

```
0000-initialize.plan.json
0001-note-pinned.plan.json
0002-create-tag-seed-tag.plan.json
0003-note-text.plan.json
0004-outbox-attachment.plan.json
meta/0000.schema.json … meta/0004.schema.json
manifest.json
snapshots.json
index.ts                    default export { manifest, plans, snapshots }
runtime-contract.json
```

The artifacts are NOT hand-authored and are NOT checked in yet: plan and
prefix digests are native canonical hashes whose byte format is
provisional until the F3 format freeze (C12), and the campaign's
no-execution rule forbids running the generator before the F3 barrier.
Fabricating digest values here would be invented evidence.

At F3, run:

```sh
pnpm run generate     # scripts/generate-history.ts → this directory
```

review the emitted plans, and commit them. Deterministic regeneration is
part of the gate (TS-MIG-01): rerunning writes nothing and a fresh
regeneration is byte-identical. Deployment consumes these committed files
as inert data (`scripts/migrate.ts`, `scripts/init-tenant.ts`); no server
ever runs generation.
