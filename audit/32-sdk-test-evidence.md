# TypeScript audit: preserved test commands and reproduction evidence

Audit date: 2026-09-04. This document preserves the actual commands and inline source executed during the SDK audit. It is evidence accompanying `30-sdk-hosting.md`, not a newly installed test suite or a claim that proposed fixes have been verified.

## Provenance and limits

- Workspace: `/Users/bjorn/Documents/bumbledb`, including the pre-existing uncommitted changes described elsewhere in the audit.
- Runtime: Node `v26.4.0`, darwin-arm64.
- Existing native artifact: `bumbledb-node 0.20.3 (bumbledb storage format v8)`.
- Source selection: `--conditions=bumbledb-src` for package-internal source imports. The native artifact was already installed; this audit did not rebuild it.
- Fixture schema: primarily the existing `Ledger`/`Holder` declarations in `ts-log/test/fixtures.ts`; SDK-003 constructs a one-byte relation instead.
- No AWS requests, deployments, or user database accesses. Probes use newly allocated temporary directories plus `memStore`, except SDK-016, which uses `fsStore` in its own new temporary directory.
- All nine preserved probe invocations exited with status 0. Each was run once as the invocation shown; this is not a randomized stress campaign.
- The probe sources were passed directly to `node -e`. They were **never saved as standalone source files**. The originals preserved below come from the executed tool-call history. The printed temporary paths contain probe database artifacts, not original script files, and their continued existence is not guaranteed.
- These are bug-demonstration programs, not assertion-based regression tests. Several deliberately print an incorrect result and then exit successfully. Successful process exit does not mean the database behavior passed.

Version checks actually run:

```sh
node --version
```

From `/Users/bjorn/Documents/bumbledb/ts`:

```sh
node --conditions=bumbledb-src --input-type=module -e 'import {native} from "./src/native.ts"; console.log("native engine",native.engineVersion())'
```

Observed output:

```text
v26.4.0
native engine bumbledb-node 0.20.3 (bumbledb storage format v8)
```

## Existing tests: exact selected commands

From `/Users/bjorn/Documents/bumbledb/ts-log`:

```sh
node --conditions=bumbledb-src --test test/replica-writer.test.ts test/replica-open.test.ts test/tenants.test.ts test/writer.test.ts test/store.test.ts test/recovery.test.ts test/temporal-gate.test.ts test/checkpoint-orphan.test.ts test/keys.test.ts test/codec.test.ts test/chain.test.ts test/fingerprint.test.ts test/identity-lane.test.ts
```

Actual summary:

```text
tests 132
suites 26
pass 132
fail 0
cancelled 0
skipped 0
todo 0
duration_ms 10749.909584
```

From `/Users/bjorn/Documents/bumbledb/ts`:

```sh
node --conditions=bumbledb-src --test test/read-scope-leak.test.ts test/owned-read.test.ts test/marshal-bijection.test.ts test/native-loader.test.ts test/db.test.ts test/ffi.test.ts test/type-kernel.test.ts test/keyed-get.test.ts
```

Actual summary:

```text
tests 77
suites 12
pass 77
fail 0
cancelled 0
skipped 0
todo 0
duration_ms 1259.014541
```

Total: **209 existing tests passed**. No `build`, `pack`, native rebuild, or broad repository battery was invoked by this audit agent. Timings are incidental shared-machine test output, not benchmark evidence.

## Probe index

| Finding | Input/interleaving | Incorrect observation executed |
| --- | --- | --- |
| SDK-001 | First log PUT throws; next commit uses same writer | Published receipt while local and fresh-reader facts differ |
| SDK-002 | Dispose replica, then call retained writer | Commit still publishes |
| SDK-003 | Mutate recorded bytes during asynchronous commit | Local byte 2, published byte 1 |
| SDK-004 | Two borrows share one handle; release first twice | Second still-held borrower gets evicted |
| SDK-005 | Dispose pool while tenant open's GET is paused | Open returns live handle after pool disposal |
| SDK-006 | Acquire successor directory lease with controlled clock advance | Old pool still returns its old handle |
| SDK-008 | Write through exposed engine, then refresh | Accepted local fact disappears |
| SDK-014 | Delay losing candidate's PUT; read before re-judgment | Read exposes candidate that ultimately returns rejected |
| SDK-016 | Restart and bind existing cache to different same-schema prefix | Reads first tenant's facts under second tenant's configuration |

All following commands ran from `/Users/bjorn/Documents/bumbledb/ts-log`. To repeat them, use that package directory or the corresponding `ts-log` directory in another checkout. The long one-line bodies are intentionally preserved as executed rather than reformatted into a different harness. Repeating them creates new temporary fixture directories.

## SDK-001 — Pending overwritten after a store failure

Exact invocation/source:

```sh
node --conditions=bumbledb-src --input-type=module -e 'import {openWriter} from "#writer.ts"; import {openReplica} from "#replica.ts"; import {memStore} from "#store.ts"; import {Holder,Ledger} from "#test/fixtures.ts"; import {mkdtemp} from "node:fs/promises"; import {tmpdir} from "node:os"; const dir=await mkdtemp(tmpdir()+"/bdb-audit-pending-"); const base=memStore(); let fail=true; const store={...base,async putCreate(k,b){ if(k.includes("/log/")&&fail){fail=false;throw new Error("injected PUT failure");}return base.putCreate(k,b)}}; const w=await openWriter({store,prefix:"p",dir:dir+"/w",theory:Ledger}); try{await w.commit(b=>b.insert(Holder,[{id:1n,name:"one"}]))}catch(e){console.log("failed",e.message)}; console.log("next",await w.commit(b=>b.insert(Holder,[{id:2n,name:"two"}]))); const r=await openReplica({store,prefix:"p",dir:dir+"/r",theory:Ledger}); console.log("writer",w.replica.db.read(i=>i.scan(Holder))); console.log("reader",r.db.read(i=>i.scan(Holder))); await w.replica[Symbol.asyncDispose](); await r[Symbol.asyncDispose](); console.log("artifact dir",dir)'
```

Actual output:

```text
failed injected PUT failure
next {
  tag: 'accepted',
  value: {
    value: { submitted: 1n, changed: 0n },
    braid: 'c00000000',
    slot: 1n,
    durability: 'published'
  }
}
writer [ { id: 1n, name: 'one' }, { id: 2n, name: 'two' } ]
reader [ { id: 2n, name: 'two' } ]
artifact dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/bdb-audit-pending-mhbnwj
```

Expected correctness property: a subsequent accepted command cannot leave the writer materialization inconsistent with authoritative replay or silently overwrite unresolved pending evidence. The failed first operation was not acknowledged; this probe does not claim that losing an unacknowledged fact alone violates durability. The dependent-child replay failure discussed in the finding is a static consequence, not an additional executed probe.

## SDK-002 — Publication after disposal

Exact invocation/source:

```sh
node --conditions=bumbledb-src --input-type=module -e 'import {openWriter} from "#writer.ts"; import {openReplica} from "#replica.ts"; import {memStore} from "#store.ts"; import {Holder,Ledger} from "#test/fixtures.ts"; import {mkdtemp} from "node:fs/promises"; import {tmpdir} from "node:os"; const dir=await mkdtemp(tmpdir()+"/bdb-audit-dispose-");const store=memStore();const w=await openWriter({store,prefix:"p",dir:dir+"/w",theory:Ledger});await w.replica[Symbol.asyncDispose](); console.log("commit after dispose",await w.commit(b=>b.insert(Holder,[{id:1n,name:"one"}]))); const r=await openReplica({store,prefix:"p",dir:dir+"/r",theory:Ledger});console.log("reader",r.db.read(i=>i.scan(Holder)));await r[Symbol.asyncDispose](); console.log("artifact dir",dir)'
```

Actual output:

```text
commit after dispose {
  tag: 'accepted',
  value: {
    value: { submitted: 1n, changed: 0n },
    braid: 'c00000000',
    slot: 1n,
    durability: 'published'
  }
}
reader [ { id: 1n, name: 'one' } ]
artifact dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/bdb-audit-dispose-cFFZ1n
```

Expected correctness property: disposal revokes further mutation/publication authority; a retained writer refuses before store mutation.

## SDK-003 — Mutable byte cell diverges from persisted bytes

Exact invocation/source:

```sh
node --conditions=bumbledb-src --input-type=module -e 'import {openWriter} from "#writer.ts"; import {openReplica} from "#replica.ts"; import {memStore} from "#store.ts"; import {bytes,relation,schema} from "@bjornpagen/bumbledb"; import {mkdtemp} from "node:fs/promises";import {tmpdir} from "node:os";const R=relation("R",{v:bytes(1)});const T=schema("T",{R},[]);const dir=await mkdtemp(tmpdir()+"/bdb-audit-bytes-");const store=memStore(); const w=await openWriter({store,prefix:"p",dir:dir+"/w",theory:T});const value=new Uint8Array([1]);const result=await w.commit(b=>{b.insert(R,[{v:value}]);setTimeout(()=>{value[0]=2},0)});const r=await openReplica({store,prefix:"p",dir:dir+"/r",theory:T});console.log("commit",result);console.log("writer",w.replica.db.read(i=>i.scan(R)));console.log("reader",r.db.read(i=>i.scan(R)));await w.replica[Symbol.asyncDispose]();await r[Symbol.asyncDispose]();console.log("artifact dir",dir)'
```

Actual output:

```text
commit {
  tag: 'accepted',
  value: {
    value: undefined,
    braid: 'c00000000',
    slot: 1n,
    durability: 'published'
  }
}
writer [ { v: Uint8Array(1) [ 2 ] } ]
reader [ { v: Uint8Array(1) [ 1 ] } ]
artifact dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/bdb-audit-bytes-VcQhLa
```

Expected correctness property: local application and fresh replay use the same immutable command meaning. This exact probe relies on the timer firing during the real asynchronous sidecar-I/O window; that interleaving occurred in the recorded execution. It does not claim a deterministic timer schedule on every future machine. Escaped-recorder variants in the report were not separately executed.

## SDK-004 — One borrow releases another

Exact invocation/source:

```sh
node --conditions=bumbledb-src --input-type=module -e 'import {openWriter} from "#writer.ts"; import {openTenants} from "#tenants.ts"; import {memStore} from "#store.ts"; import {Ledger} from "#test/fixtures.ts";import {mkdtemp} from "node:fs/promises";import {tmpdir} from "node:os";const dir=await mkdtemp(tmpdir()+"/bdb-audit-pool-");const base=memStore();const w=await openWriter({store:base,prefix:"p/t/a",dir:dir+"/birth",theory:Ledger});await w.replica[Symbol.asyncDispose]();const pool=openTenants({store:base,root:"p",dir:dir+"/pool",theory:Ledger});const first=await pool.get("a");const second=await pool.get("a");console.log("same handle",first===second);first.release();first.release();console.log("evicts second active borrower",await pool.evict("a"));try{console.log(second.db)}catch(e){console.log("second dead",e.message)};await pool[Symbol.asyncDispose]();console.log("artifact dir",dir)'
```

Actual output:

```text
same handle true
evicts second active borrower { Symbol(disposedHandle): Symbol(disposedHandle) }
second dead replica is disposed
artifact dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/bdb-audit-pool-Gu5THq
```

Expected correctness property: each successful acquisition owns an independent, idempotently releasable borrow. The trigger intentionally includes a duplicate release—caller misuse that the shared handle cannot distinguish from another borrow's legitimate release. The stale-slot and `await using` variants in the finding were not separately executed.

## SDK-005 — Open completes after pool shutdown

Exact invocation/source:

```sh
node --conditions=bumbledb-src --input-type=module -e 'import {openWriter} from "#writer.ts"; import {openTenants} from "#tenants.ts";import {memStore} from "#store.ts";import {Holder,Ledger} from "#test/fixtures.ts";import {mkdtemp} from "node:fs/promises";import {tmpdir} from "node:os";const dir=await mkdtemp(tmpdir()+"/bdb-audit-pool-close-");const base=memStore();const w=await openWriter({store:base,prefix:"p/t/a",dir:dir+"/birth",theory:Ledger});await w.replica[Symbol.asyncDispose]();let release;let entered;const enteredP=new Promise(r=>entered=r);const blocked=new Promise(r=>release=r);const store={...base,async get(k){entered();await blocked;return base.get(k)}};const pool=openTenants({store,root:"p",dir:dir+"/pool",theory:Ledger});const opening=pool.get("a");await enteredP;await pool[Symbol.asyncDispose]();console.log("pool disposed");release();const handle=await opening;console.log("get completed after pool disposal",handle.db.read(i=>i.count(Holder)));await pool[Symbol.asyncDispose]();console.log("artifact dir",dir)'
```

Actual output:

```text
pool disposed
get completed after pool disposal 0n
artifact dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/bdb-audit-pool-close-ih0YiT
```

Expected correctness property: pool disposal joins or cancels all outstanding opens; it cannot return before a later open installs a new live slot. The second disposal in this probe cleans up the slot/timer that incorrectly appeared after the first shutdown.

## SDK-006 — Missing old token is accepted as successful renewal

Exact invocation/source:

```sh
node --conditions=bumbledb-src --input-type=module -e 'import {openWriter} from "#writer.ts";import {openTenants} from "#tenants.ts";import {memStore,acquireFsLease,releaseFsLease} from "#store.ts";import {Ledger} from "#test/fixtures.ts";import {mkdtemp} from "node:fs/promises";import {tmpdir} from "node:os";const dir=await mkdtemp(tmpdir()+"/bdb-audit-lease-loss-");const store=memStore();const w=await openWriter({store,prefix:"p/t/a",dir:dir+"/birth",theory:Ledger});await w.replica[Symbol.asyncDispose]();const pool=openTenants({store,root:"p",dir:dir+"/pool",theory:Ledger});const old=await pool.get("a");const original=Date.now;let next;try{Date.now=()=>original()+400000;next=await acquireFsLease(dir+"/pool","a",300000,"refuse")}finally{Date.now=original};console.log("new owner token",next.token);const still=await pool.get("a");console.log("old pool returned same handle after takeover",still===old);still.release();old.release();await pool[Symbol.asyncDispose]();await releaseFsLease(next);console.log("artifact dir",dir)'
```

Actual output:

```text
new owner token 2n
old pool returned same handle after takeover true
artifact dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/bdb-audit-lease-loss-blKmTw
```

Expected correctness property: once successor token 2 owns the directory, the old pool must refuse to issue token-1 capabilities. The injected clock advance is local to this diagnostic process and restored in `finally`; no system clock was changed. This proves the missing-token fail-open branch using the actual filesystem lease implementation. It is not a multiprocess scheduler-pause or simultaneous-engine-write stress test.

## SDK-008 — A successful exposed engine write disappears on refresh

Exact invocation/source:

```sh
node --conditions=bumbledb-src --input-type=module -e 'import {openWriter} from "#writer.ts";import {memStore} from "#store.ts";import {Holder,Ledger} from "#test/fixtures.ts";import {mkdtemp} from "node:fs/promises";import {tmpdir} from "node:os";const dir=await mkdtemp(tmpdir()+"/bdb-audit-direct-write-");const store=memStore();const w=await openWriter({store,prefix:"p",dir:dir+"/w",theory:Ledger});console.log("direct",w.replica.db.write(tx=>tx.insert(Holder,[{id:1n,name:"one"}])));console.log("before refresh",w.replica.db.read(i=>i.scan(Holder)));await w.replica.refresh();console.log("after refresh",w.replica.db.read(i=>i.scan(Holder)));await w.replica[Symbol.asyncDispose]();console.log("artifact dir",dir)'
```

Actual output:

```text
direct {
  tag: 'accepted',
  value: { value: { submitted: 1n, changed: 1n }, generation: 1n }
}
before refresh [ { id: 1n, name: 'one' } ]
after refresh []
artifact dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/bdb-audit-direct-write-lYRuPo
```

Expected correctness property: a replicated read capability must not expose a normal successful mutation operation that silently bypasses authoritative publication. This is a public capability/design defect; the probe intentionally calls the exposed engine write instead of the correct log-writer path.

## SDK-014 — A rejected candidate becomes visible to an ordinary reader

Exact invocation/source:

```sh
node --conditions=bumbledb-src --input-type=module -e 'import {openWriter} from "#writer.ts";import {memStore} from "#store.ts";import {Holder,Ledger} from "#test/fixtures.ts";import {mkdtemp} from "node:fs/promises";import {tmpdir} from "node:os";const dir=await mkdtemp(tmpdir()+"/bdb-audit-speculative-");const base=memStore();let release;let entered;const signal=new Promise(r=>entered=r);const block=new Promise(r=>release=r);const heldStore={...base,async putCreate(k,b){if(k.includes("/log/")){entered();await block}return base.putCreate(k,b)}};const a=await openWriter({store:base,prefix:"p",dir:dir+"/a",theory:Ledger});const b=await openWriter({store:heldStore,prefix:"p",dir:dir+"/b",theory:Ledger});await a.commit(x=>x.insert(Holder,[{id:1n,name:"winner"}]));const pending=b.commit(x=>x.insert(Holder,[{id:1n,name:"loser"}]));await signal;console.log("visible before publication",b.replica.db.read(i=>i.scan(Holder)));release();console.log("commit eventually",(await pending).tag);console.log("visible afterward",b.replica.db.read(i=>i.scan(Holder)));await a.replica[Symbol.asyncDispose]();await b.replica[Symbol.asyncDispose]();console.log("artifact dir",dir)'
```

Actual output:

```text
visible before publication [ { id: 1n, name: 'loser' } ]
commit eventually rejected
visible afterward [ { id: 1n, name: 'winner' } ]
artifact dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/bdb-audit-speculative-Frn5c0
```

Expected correctness property: an ordinary committed-read surface cannot expose a candidate that ultimately returns rejected. The store wrapper controls a deterministic promise barrier around B's publication; no arbitrary timeout was needed to sample the visibility window.

## SDK-016 — Same-schema cache rebound to a different namespace

The exact command below is the complete parent orchestrator plus both inline child sources. It runs the seed and check phases in separate Node processes with a 30-second bound per child. Their sources also existed only as inline strings, not temporary script files.

Exact invocation/source:

```sh
node --conditions=bumbledb-src --input-type=module -e 'import {mkdtemp} from "node:fs/promises"; import {tmpdir} from "node:os"; import {spawnSync} from "node:child_process"; const dir=await mkdtemp(tmpdir()+"/bdb-audit-cache-identity-"); const seed=String.raw`import {openWriter} from "#writer.ts"; import {fsStore} from "#store.ts"; import {Holder,Ledger} from "#test/fixtures.ts"; const dir=process.argv[1]; const store=fsStore(dir+"/objects"); const a=await openWriter({store,prefix:"tenant-a",dir:dir+"/cache",theory:Ledger});const b=await openWriter({store,prefix:"tenant-b",dir:dir+"/other-cache",theory:Ledger});await a.commit(tx=>tx.insert(Holder,[{id:1n,name:"tenant-a-secret"}]));await b.commit(tx=>tx.insert(Holder,[{id:1n,name:"tenant-b-data"}]));await a.replica[Symbol.asyncDispose]();await b.replica[Symbol.asyncDispose]();console.log("seeded both prefixes with different rows at vector1");`; const check=String.raw`import {openReplica} from "#replica.ts"; import {fsStore} from "#store.ts"; import {Holder,Ledger} from "#test/fixtures.ts"; const dir=process.argv[1];const store=fsStore(dir+"/objects");const reused=await openReplica({store,prefix:"tenant-b",dir:dir+"/cache",theory:Ledger});const fresh=await openReplica({store,prefix:"tenant-b",dir:dir+"/fresh-b",theory:Ledger});console.log("reused cache opened for tenant-b",reused.db.read(i=>i.scan(Holder)));console.log("fresh cache opened for tenant-b",fresh.db.read(i=>i.scan(Holder)));console.log("reused vector",[...reused.vector]);console.log("fresh vector",[...fresh.vector]);await reused[Symbol.asyncDispose]();await fresh[Symbol.asyncDispose]();`; for(const [phase,code] of [["seed",seed],["check",check]]){const r=spawnSync(process.execPath,["--conditions=bumbledb-src","--input-type=module","-e",code,dir],{cwd:process.cwd(),encoding:"utf8",timeout:30000});console.log(phase,"status",r.status,"signal",r.signal);console.log(r.stdout);if(r.stderr) console.log(r.stderr);if(r.status!==0)break;} console.log("artifact dir",dir)'
```

Actual output:

```text
seed status 0 signal null
seeded both prefixes with different rows at vector1

check status 0 signal null
reused cache opened for tenant-b [ { id: 1n, name: 'tenant-a-secret' } ]
fresh cache opened for tenant-b [ { id: 1n, name: 'tenant-b-data' } ]
reused vector [ [ 'c00000000', 1n ], [ 'c00000002', 0n ] ]
fresh vector [ [ 'c00000000', 1n ], [ 'c00000002', 0n ] ]

artifact dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/bdb-audit-cache-identity-UR8Mtw
```

Expected correctness property: a local directory configured for a different logical database must be rejected or reseeded before serving facts or replaying pending commands. The trigger is an ordinary configuration-consistency mistake, not an authorized cross-tenant read. Both remote namespaces contain legitimate, different fixture rows. Process separation ensures the result is not explained by retained live Node handles. This probe does not depend on filesystem case folding; the related ordinary tenant-alias scenario is covered by REP-011 in the companion replication evidence.

## What was not executed

No standalone C ABI reproduction of SDK-013 was run by this agent; that finding is established by the engine-owning `Arc` retained in permanently leaked read tombstones. No new runtime probes were run for SDK-007, SDK-009, SDK-010, SDK-011, SDK-012, or SDK-015. No proposed repair was implemented, so there is no before/after test result. Preserve those distinctions when converting this audit into issue tickets or release gates.

The source references and recommended expanded regression matrices live in `30-sdk-hosting.md` and `31-ffi-packaging.md`. The minimal probes here should be converted into deterministic assertion-based tests, broadened to the fault schedules named in those findings, and checked against a freshly built native artifact during remediation.
