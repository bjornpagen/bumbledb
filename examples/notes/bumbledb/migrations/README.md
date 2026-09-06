# Generated migration repository

The example's source checkout does not contain a generated chain. From
`examples/notes/`, after installing matching packages, run:

```sh
pnpm run generate
```

`scripts/generate-history.ts` passes the staged schemas and evolution intent
to the real generator. It writes five plans, their schema snapshots, a
manifest, `snapshots.json`, `index.ts`, and `runtime-contract.json` here.
Review and commit that generated data in a deployed application's repository.
Do not hand-author hashes, plans, or runtime contracts.

The packed-consumer check generates this chain in its temporary copy of the
Notes application and exercises it in the application tests. Those outputs
are not release migrations installed in this source directory.

Deployment reads generated artifacts; it must not generate a schema diff on
the server. The current migration runner supports local history authorities
only. Hosted S3 migration orchestration remains unfinished; never run a local
migration against a hosted history's disposable cache.
