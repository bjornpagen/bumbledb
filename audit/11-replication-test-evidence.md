# Replication reproduction evidence

Date: 2026-09-04. Companion to [the replication/storage audit](10-replication-storage.md).

## Result

Ten assertions/scenarios executed successfully against the current repository's Rust implementations. “Successfully” here means the harness confirmed the undesirable behavior, not that the implementation passed the desired property. No repository source or existing tests were changed.

| Scenario | Finding | Observation |
|---|---|---|
| Stale writer after GC | REP-001 | Actual Writer returned Published at slot 1; a fresh checkpoint-seeded Replica stayed at generation 2 without the new value |
| Incomparable checkpoints | REP-002 | Actual compacted snapshots [0,2] then [3,0] both published; after collecting B1, a new Replica served B=0 |
| Equal GC clock | REP-003 | A newly written predecessor checkpoint was deleted |
| Newly opened default checkpointer | REP-003 | The normal Checkpointer::open/run path with 90-day retention deleted a predecessor created 1,107 ms earlier |
| Ambiguous counter birth | REP-004 | The definite winner and the ambiguous caller both received 0..4096 |
| Lower writer token | REP-006 | Three ordinary attempts returned Moved against unchanged counter bytes; the harness stopped the fourth call with an artificial error |
| Scratch after successful older publish | REP-007 | Recovery deleted both objects of a still-reachable predecessor |
| Delayed backlink | REP-008 | Successfully published checkpoint 2 was skipped by checkpoint 3's stale predecessor |
| Filesystem case aliases | REP-011 | Creating tenant-a after tenant-A returned Exists and reading it returned tenant-A bytes |
| Per-tenant cache data leak | REP-011 | Distinct, case-sensitive remote tenant prefixes contained different facts; the tenant-a handle served the persisted local tenant-A state |

The last case uses MemStore as a case-sensitive remote (like S3) and the real Tenants/Replica/local-directory layer. It separates the cache-isolation bug from object-store case aliasing: the remote objects themselves were correctly isolated.

## What this does and does not prove

- The full stale-writer and checkpointer tests use real database files, compaction, FsStore, public Writer/Replica/Checkpointer APIs, and actual catalog digests. They are not pure mocks of those implementations.
- The component-regression test constructs two valid prefix snapshots using restore_to_vector, representing delayed/incomparable checkpointer views. It then invokes the real snapshot upload, checkpoint publication, collection, and fresh-replica paths. It does not claim a nondeterministic live race was observed in this run.
- The ID ambiguity case injects the permitted unproved transport outcome and executes the same prove_create helper used inside S3Store. No AWS request, account, or network fault was exercised. The production S3 call graph is the separate static evidence for why the composition matters.
- The allocator-livelock harness intentionally returns an infrastructure error on the fourth call, rather than hanging the audit indefinitely. All three earlier swaps are asserted to be Moved and the stored counter is asserted unchanged.
- The case-alias tests are host-filesystem specific. They reproduced on this Mac's scratch filesystem. A case-sensitive filesystem should not reproduce that particular spelling collision; the key-to-path mapping still needs an explicit support contract.
- FsStore lease TOCTOU, split body/generation persistence, and open-before-exclusivity cleanup remain **static adversarial schedules**, not process-suspension/power-loss reproductions in this audit.
- The harness was built as an external temporary Cargo package. It used the repository's current path crates and pinned repository toolchain but resolved its own dependency lock; for example blake3 resolved to 1.8.7. This is not represented as an exact reproduction under the workspace Cargo.lock. The root audit records the separate workspace-locked test run.

## Execution

Temporary package: /tmp/bumbledb-replication-audit.xjDrVs. Source and manifest are reproduced below so the evidence does not depend on that temporary directory surviving.

Command, run from the repository directory to select its toolchain:

```text
cargo run --manifest-path /tmp/bumbledb-replication-audit.xjDrVs/Cargo.toml -- /tmp/bumbledb-replication-audit.xjDrVs/output-final
```

Initial compilation took 21.51 seconds; the final incremental compile took 1.19 seconds. The final run exited 0. Each scenario asserts the failure property and aborts if it is not observed. The source creates only its explicitly supplied temporary output tree. It does not call S3 or modify application databases. The imported test support creates/removes only its own test locations; this harness uses its TestLog against the fresh output tree.

### Captured final output

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.19s
     Running `/tmp/bumbledb-replication-audit.xjDrVs/target/debug/replication-audit /tmp/bumbledb-replication-audit.xjDrVs/output-final`
GC_EQUAL_CLOCK: freshly created prior checkpoint deleted, outcome=Swept(Sweep { log_deleted: [], checkpoints_deleted: [[111, 170, 201, 7, 42, 222, 61, 129, 34, 91, 66, 124, 3, 73, 130, 246, 71, 56, 224, 200, 37, 222, 94, 16, 1, 40, 254, 170, 241, 1, 9, 119]], swept_below: {BraidId(RelationId(0)): 2, BraidId(RelationId(2)): 0} })
SCRATCH_ANCESTOR: reopening removed both objects of still-reachable previous checkpoint
AMBIGUOUS_IDS: winner=Drawn { range: 0..4096, token: 7 }; ambiguous caller=Drawn { range: 0..4096, token: 8 }
LOW_WRITER_ID: high=Drawn { range: 0..4096, token: 20 }, lower made 4 unchanged attempts before harness aborted loop
STALE_BACKLINK: checkpoint 2 was published successfully but checkpoint 3 links directly to 1, orphaning 2
FS_CASE_ALIAS: first=Created(Etag("406a9bbb816837a5a39c71c8338dbb6d16906cca0b26dd23ca2e540c6fa228fc")); second=Exists; lower-key bytes="tenant-A"
STALE_SLOT: sweep=Swept(Sweep { log_deleted: ["log/c00000000/0000000000000001"], checkpoints_deleted: [], swept_below: {BraidId(RelationId(0)): 2, BraidId(RelationId(2)): 0} }); acknowledged=Accepted(Slotted { value: (), braid: BraidId(RelationId(0)), slot: 1, durability: Published }); fresh checkpoint reader remains original generation 2, new value absent
COMPONENT_REGRESSION: published [0,2] -> [3,0]; fresh reader returns B=0 despite acknowledged B slots 1..2
CHECKPOINTER_RESTART: default 90-day retention deleted a predecessor created 1107ms earlier; run=Ready { compact: Quiet, gc: Swept(Sweep { log_deleted: [], checkpoints_deleted: [[191, 45, 6, 67, 90, 3, 24, 70, 182, 75, 128, 10, 17, 117, 216, 195, 82, 239, 190, 33, 116, 118, 41, 101, 198, 57, 8, 30, 234, 141, 182, 209]], swept_below: {BraidId(RelationId(0)): 0, BraidId(RelationId(2)): 0} }) }
TENANT_CASE_LEAK: isolated case-sensitive remote prefixes contain different facts; tenant-a opened cached tenant-A facts through local case-alias
```

## Re-running or promoting into regression tests

Copy the manifest and source blocks into a new temporary directory (update the two repository-absolute paths if needed). Use a new output directory on each run: store creation intentionally refuses already-existing fixtures. After fixing each defect, invert its assertions to the desired contract and add the resulting test to the appropriate permanent suite. Do not run this destructive-failure harness against a real tenant directory.

The test support module comes from crates/bumbledb-log/tests/lane_d_support/mod.rs and provides the existing two-braid theory and honest batch encoder. The exact manifest was:

```toml
[package]
name = "bumbledb-replication-audit"
version = "0.0.0"
edition = "2024"

[[bin]]
name = "replication-audit"
path = "repro.rs"

[dependencies]
bumbledb = { path = "/Users/bjorn/Documents/bumbledb/crates/bumbledb" }
bumbledb-log = { path = "/Users/bjorn/Documents/bumbledb/crates/bumbledb-log", default-features = false }
blake3 = "1.8.5"
```

Toolchain: `rustc 1.99.0-nightly (d453bdd8f 2026-08-14)`.

### Harness source

```rust
#[path = "/Users/bjorn/Documents/bumbledb/crates/bumbledb-log/tests/lane_d_support/mod.rs"]
mod support;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc,Mutex};
use bumbledb::{Admission, Db, SchemaDescriptor, Value};
use bumbledb::schema::{FieldId, RelationId};
use bumbledb_log::codec::Codec;
use bumbledb_log::checkpointer::{Checkpointer, CheckpointerOpened};
use bumbledb_log::gc::{gc_at, PublishClock, Restore, restore_to_vector, CHECKPOINT_RETAIN_MS};
use bumbledb_log::lease::{Leased, lease_block};
use bumbledb_log::manifest::{Checkpoint, Head, Manifest, Published, ckpt_doc_key, ckpt_mdb_key, create_manifest, log_key, manifest_key, publish_checkpoint};
use bumbledb_log::replica::{Opened, Replica, Vector, record_ckpt_scratch, sweep_at_open};
use bumbledb_log::store::{Create, Etag, Fenced, Fetched, ObjectStore, Poll, Result as StoreResult, StoreError, StoreKey, Swap, prove_create};
use bumbledb_log::store::mem::MemStore;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::writer::{Writer, WriterOpened, Options, Durability};
use support::{TestLog, theory, codec, kitchen_braid, note_braid, insert_recipe, insert_note};

fn heads(c: &Codec, a: u64, b: u64) -> BTreeMap<bumbledb_log::braids::BraidId, Head> {
    c.braids().components().keys().map(|id| (*id, Head {g: if *id == kitchen_braid(c) {a} else {b}, hash: [0;32], ts: 1})).collect()
}
fn manifest(store: &impl ObjectStore, c: &Codec) {
    create_manifest(store, "", &Manifest {fingerprint: *c.fingerprint(), checkpoint: None}).unwrap();
}
fn open_replica(root: &Path, dir: &Path) -> Replica<SchemaDescriptor, FsStore> {
    match Replica::open(FsStore::new(root), "", dir, theory()).unwrap() {
        Opened::Ready(r) => *r,
        Opened::Refused(r) => panic!("replica refused {r:?}"),
    }
}
fn store_snapshot(store: &impl ObjectStore, doc: &Checkpoint, db: &Db<SchemaDescriptor>, dir: &Path) {
    db.compact(dir).unwrap();
    let bytes = std::fs::read(dir.join("data.mdb")).unwrap();
    store.put_create(&ckpt_mdb_key("", &doc.digest()), &bytes).unwrap();
}
fn gc_clock_sentinel() {
    let store = MemStore::new(); let c = codec(); manifest(&store, &c);
    let a = Checkpoint {braids: heads(&c, 1, 0), catalog: [1;32], writer: 1, prev: None};
    store.put_create(&ckpt_mdb_key("", &a.digest()), b"snapshot-a").unwrap();
    assert_eq!(publish_checkpoint(&store,"",c.braids(),&a).unwrap(), Published::Replaced);
    let b = Checkpoint {braids: heads(&c, 2, 0), catalog: [2;32], writer: 1, prev: Some(a.digest())};
    store.put_create(&ckpt_mdb_key("", &b.digest()), b"snapshot-b").unwrap();
    assert_eq!(publish_checkpoint(&store,"",c.braids(),&b).unwrap(), Published::Replaced);
    let now = 1_800_000_000_000;
    let outcome = gc_at(&store,"",&c,CHECKPOINT_RETAIN_MS,PublishClock {now_ms:now,publish_ms:now}).unwrap();
    assert!(store.get(&ckpt_doc_key("",&a.digest())).unwrap().is_none());
    println!("GC_EQUAL_CLOCK: freshly created prior checkpoint deleted, outcome={outcome:?}");
}
fn scratch_ancestor(base: &Path) {
    let store = MemStore::new(); let c = codec(); manifest(&store,&c);
    let a = Checkpoint {braids: heads(&c,1,0), catalog:[1;32],writer:1,prev:None};
    store.put_create(&ckpt_mdb_key("",&a.digest()),b"snapshot-a").unwrap();
    publish_checkpoint(&store,"",c.braids(),&a).unwrap();
    record_ckpt_scratch(base,&a.digest()).unwrap();
    let b = Checkpoint {braids: heads(&c,2,0),catalog:[2;32],writer:2,prev:Some(a.digest())};
    store.put_create(&ckpt_mdb_key("",&b.digest()),b"snapshot-b").unwrap();
    publish_checkpoint(&store,"",c.braids(),&b).unwrap();
    sweep_at_open(&store,"",base).unwrap();
    assert!(store.get(&ckpt_doc_key("",&a.digest())).unwrap().is_none());
    assert!(store.get(&ckpt_mdb_key("",&a.digest())).unwrap().is_none());
    println!("SCRATCH_ANCESTOR: reopening removed both objects of still-reachable previous checkpoint");
}
struct AmbiguousBirth {inner: MemStore, injected: AtomicBool, winner:Mutex<Option<Leased>>}
impl ObjectStore for AmbiguousBirth {
    fn get(&self,k:&StoreKey)->StoreResult<Option<Fetched>> { self.inner.get(k) }
    fn get_if_changed(&self,k:&StoreKey,e:&Etag)->StoreResult<Poll> { self.inner.get_if_changed(k,e) }
    fn put_create<'a>(&self,k:&StoreKey,b:impl Into<Fenced<'a>>)->StoreResult<Create> {
        let b=b.into();
        if !self.injected.swap(true,Ordering::SeqCst) {
            let winner=lease_block(&self.inner,"",RelationId(0),FieldId(0),7,1)?;
            *self.winner.lock().unwrap()=Some(winner);
            // S3Store's exact post-transport helper after a 409/timeout.
            prove_create(&self.inner,k,b.bytes,Create::Ambiguous)
        } else { self.inner.put_create(k,b) }
    }
    fn put_swap<'a>(&self,k:&StoreKey,b:impl Into<Fenced<'a>>,e:&Etag)->StoreResult<Swap> {self.inner.put_swap(k,b,e)}
    fn delete(&self,k:&StoreKey)->StoreResult<()> {self.inner.delete(k)}
}
fn duplicate_ids() {
    let store=AmbiguousBirth {inner:MemStore::new(),injected:AtomicBool::new(false),winner:Mutex::new(None)};
    let loser=lease_block(&store,"",RelationId(0),FieldId(0),8,1).unwrap();
    let winner=store.winner.lock().unwrap();
    match (winner.as_ref().unwrap(),&loser) {
        (Leased::Drawn {range:a,..},Leased::Drawn {range:b,..})=>assert_eq!(a,b),
        _=>panic!("not drawn"),
    }
    println!("AMBIGUOUS_IDS: winner={:?}; ambiguous caller={loser:?}",winner.as_ref().unwrap());
}
struct StopMoved { inner:MemStore, swaps:AtomicU64 }
impl ObjectStore for StopMoved {
    fn get(&self,k:&StoreKey)->StoreResult<Option<Fetched>>{self.inner.get(k)}
    fn get_if_changed(&self,k:&StoreKey,e:&Etag)->StoreResult<Poll>{self.inner.get_if_changed(k,e)}
    fn put_create<'a>(&self,k:&StoreKey,b:impl Into<Fenced<'a>>)->StoreResult<Create>{self.inner.put_create(k,b)}
    fn put_swap<'a>(&self,k:&StoreKey,b:impl Into<Fenced<'a>>,e:&Etag)->StoreResult<Swap>{
        if self.swaps.fetch_add(1,Ordering::SeqCst)>=3 {return Err(StoreError {op:"audit-stop",key:k.to_string(),source:std::io::Error::other("stop repeated fenced-out attempts")});}
        let r=self.inner.put_swap(k,b,e)?;assert_eq!(r,Swap::Moved);Ok(r)
    }
    fn delete(&self,k:&StoreKey)->StoreResult<()>{self.inner.delete(k)}
}
fn low_id_stall() {
    let store=StopMoved {inner:MemStore::new(),swaps:AtomicU64::new(0)};
    let high=lease_block(&store.inner,"",RelationId(0),FieldId(0),20,1).unwrap();
    let low=lease_block(&store,"",RelationId(0),FieldId(0),10,1);
    assert!(low.is_err());
    let counter=store.inner.get(&bumbledb_log::lease::ids_key("",RelationId(0),FieldId(0))).unwrap().unwrap();
    assert_eq!(counter.bytes,b"4096");
    println!("LOW_WRITER_ID: high={high:?}, lower made {} unchanged attempts before harness aborted loop",store.swaps.load(Ordering::SeqCst));
}
fn stale_writer_recreates_slot(base:&Path) {
    let object_root=base.join("objects");
    let mut log=TestLog::new(object_root.clone(),"");
    let braid=kitchen_braid(&log.codec);
    let stale=match Writer::open(FsStore::new(&object_root),"",&base.join("stale"),theory(),Options::new(10)).unwrap(){WriterOpened::Ready(w)=>w,WriterOpened::Refused(r)=>panic!("{r:?}")};
    log.publish(braid,&[insert_recipe(1)],100);
    log.publish(braid,&[insert_recipe(2)],200);
    let builder=open_replica(&object_root,&base.join("builder"));
    let scratch=base.join("scratch"); std::fs::create_dir_all(&scratch).unwrap();
    log.checkpoint(builder.db().unwrap(),&scratch);
    let before=builder.db().unwrap().catalog_digest().unwrap();
    let sweep=gc_at(&log.store,"",&log.codec,1,PublishClock {now_ms:300,publish_ms:200}).unwrap();
    assert!(log.store.get(&log_key("",braid,1)).unwrap().is_none());
    let outcome=stale.commit(|batch|{batch.insert(support::RECIPE,vec![Box::from([Value::U64(999)])]);Ok(())}).unwrap();
    match &outcome {Admission::Accepted(a)=>{assert_eq!(a.slot,1);assert_eq!(a.durability,Durability::Published)},_=>panic!("not accepted")};
    let fresh=open_replica(&object_root,&base.join("fresh"));
    assert_eq!(fresh.vector().at(braid),2);
    assert_eq!(fresh.db().unwrap().catalog_digest().unwrap(),before);
    println!("STALE_SLOT: sweep={sweep:?}; acknowledged={outcome:?}; fresh checkpoint reader remains original generation 2, new value absent");
    stale.quiesce();
}
fn checkpoint_component_regression(base:&Path) {
    let object_root=base.join("objects");let mut log=TestLog::new(object_root.clone(),"");
    let a=kitchen_braid(&log.codec);let b=note_braid(&log.codec);
    for n in 1..=3 {log.publish(a,&[insert_recipe(n)],100+n);}
    for n in 1..=2 {log.publish(b,&[insert_note(n,"acknowledged")],100+n);}
    let av:Vector=[(a,3),(b,0)].into_iter().collect();let bv:Vector=[(a,0),(b,2)].into_iter().collect();
    let restore=|v:&Vector,dir:&Path|match restore_to_vector(&log.store,"",dir,&theory(),v).unwrap(){Restore::Restored {db,..}=>db,Restore::Refused(r)=>panic!("{r:?}")};
    let db_a=restore(&av,&base.join("a"));let db_b=restore(&bv,&base.join("b"));
    let mut ah=log.heads.clone();ah.insert(b,Head {g:0,hash:[0;32],ts:0});
    let mut bh=log.heads.clone();bh.insert(a,Head {g:0,hash:[0;32],ts:0});
    let candidate=Checkpoint {braids:ah,catalog:db_a.catalog_digest().unwrap(),writer:1,prev:None};
    let incumbent=Checkpoint {braids:bh,catalog:db_b.catalog_digest().unwrap(),writer:2,prev:None};
    store_snapshot(&log.store,&candidate,&db_a,&base.join("compact-a"));
    store_snapshot(&log.store,&incumbent,&db_b,&base.join("compact-b"));
    assert_eq!(publish_checkpoint(&log.store,"",log.codec.braids(),&incumbent).unwrap(),Published::Replaced);
    gc_at(&log.store,"",&log.codec,1,PublishClock {now_ms:300,publish_ms:200}).unwrap();
    assert!(log.store.get(&log_key("",b,1)).unwrap().is_none());
    assert_eq!(publish_checkpoint(&log.store,"",log.codec.braids(),&candidate).unwrap(),Published::Replaced);
    let fresh=open_replica(&object_root,&base.join("fresh"));
    assert_eq!(fresh.vector().at(b),0); assert_eq!(fresh.vector().at(a),3);
    assert_eq!(fresh.db().unwrap().catalog_digest().unwrap(),db_a.catalog_digest().unwrap());
    println!("COMPONENT_REGRESSION: published [0,2] -> [3,0]; fresh reader returns B=0 despite acknowledged B slots 1..2");
}
fn stale_backlink() {
    let store=MemStore::new();let c=codec();manifest(&store,&c);
    let first=Checkpoint {braids:heads(&c,1,0),catalog:[1;32],writer:1,prev:None};
    publish_checkpoint(&store,"",c.braids(),&first).unwrap();
    let slow=Checkpoint {braids:heads(&c,3,0),catalog:[3;32],writer:2,prev:Some(first.digest())};
    let fast=Checkpoint {braids:heads(&c,2,0),catalog:[2;32],writer:3,prev:Some(first.digest())};
    publish_checkpoint(&store,"",c.braids(),&fast).unwrap();
    publish_checkpoint(&store,"",c.braids(),&slow).unwrap();
    let current=Manifest::parse(&store.get(&manifest_key("")).unwrap().unwrap().bytes).unwrap().checkpoint.unwrap();
    let current=Checkpoint::parse(&store.get(&ckpt_doc_key("",&current)).unwrap().unwrap().bytes,c.braids()).unwrap();
    assert_eq!(current.prev,Some(first.digest()));
    assert!(store.get(&ckpt_doc_key("",&fast.digest())).unwrap().is_some());
    println!("STALE_BACKLINK: checkpoint 2 was published successfully but checkpoint 3 links directly to 1, orphaning 2");
}
fn checkpointer_restart_retention(base:&Path) {
    let root=base.join("objects");let mut log=TestLog::new(root.clone(),"");
    let braid=kitchen_braid(&log.codec);let now=bumbledb_log::store::unix_ms();
    log.publish(braid,&[insert_recipe(1)],now);
    let mut builder=open_replica(&root,&base.join("builder"));
    let scratch=base.join("scratch");std::fs::create_dir_all(&scratch).unwrap();
    let prior=log.checkpoint(builder.db().unwrap(),&scratch);
    log.publish(braid,&[insert_recipe(2)],now);
    builder.refresh().unwrap();
    log.checkpoint(builder.db().unwrap(),&scratch);
    let mut checkpointer=match Checkpointer::open(FsStore::new(&root),"",&base.join("checkpointer"),theory(),99).unwrap(){CheckpointerOpened::Ready(c)=>c,CheckpointerOpened::Refused(r)=>panic!("{r:?}")};
    let outcome=checkpointer.run().unwrap();
    assert!(log.store.get(&ckpt_doc_key("",&prior)).unwrap().is_none());
    assert!(log.store.get(&ckpt_mdb_key("",&prior)).unwrap().is_none());
    println!("CHECKPOINTER_RESTART: default 90-day retention deleted a predecessor created {}ms earlier; run={outcome:?}",bumbledb_log::store::unix_ms()-now);
}
fn fs_case_alias(base:&Path) {
    let store=FsStore::new(base);
    let first=store.put_create(&StoreKey::of("tenant-A/manifest"),b"tenant-A").unwrap();
    let second=store.put_create(&StoreKey::of("tenant-a/manifest"),b"tenant-a").unwrap();
    let read=store.get(&StoreKey::of("tenant-a/manifest")).unwrap().unwrap();
    println!("FS_CASE_ALIAS: first={first:?}; second={second:?}; lower-key bytes={:?}",String::from_utf8_lossy(&read.bytes));
}
#[derive(Clone)]
struct SharedMem(Arc<MemStore>);
impl ObjectStore for SharedMem {
    fn get(&self,k:&StoreKey)->StoreResult<Option<Fetched>>{self.0.get(k)}
    fn get_if_changed(&self,k:&StoreKey,e:&Etag)->StoreResult<Poll>{self.0.get_if_changed(k,e)}
    fn put_create<'a>(&self,k:&StoreKey,b:impl Into<Fenced<'a>>)->StoreResult<Create>{self.0.put_create(k,b)}
    fn put_swap<'a>(&self,k:&StoreKey,b:impl Into<Fenced<'a>>,e:&Etag)->StoreResult<Swap>{self.0.put_swap(k,b,e)}
    fn delete(&self,k:&StoreKey)->StoreResult<()>{self.0.delete(k)}
}
fn tenant_case_leak(base:&Path) {
    use bumbledb_log::tenants::{open_tenants,Tenant,TenantOptions};
    let store=SharedMem(Arc::new(MemStore::new()));let c=codec();let braid=kitchen_braid(&c);
    for (id,value) in [("tenant-A",111),("tenant-a",222)] {
        let prefix=format!("t/{id}");
        create_manifest(&store,&prefix,&Manifest{fingerprint:*c.fingerprint(),checkpoint:None}).unwrap();
        let header=bumbledb_log::codec::BatchHeader{fingerprint:*c.fingerprint(),braid,braid_gen:1,prev:[0;32],writer:value,timestamp:1};
        let bytes=c.encode(&header,&[insert_recipe(value)]).unwrap();
        store.put_create(&log_key(&prefix,braid,1),&bytes).unwrap();
    }
    let options=TenantOptions{budget_bytes:u64::MAX,max_open:10};
    let a_digest={
        let mut tenants=open_tenants(store.clone(),"",base,theory(),options);
        let live=match tenants.tenant("tenant-A").unwrap(){Tenant::Live(l)=>l,Tenant::Refused(r)=>panic!("{r:?}")};
        live.db().unwrap().catalog_digest().unwrap()
    };
    let b_digest={
        let mut tenants=open_tenants(store.clone(),"",base,theory(),options);
        let live=match tenants.tenant("tenant-a").unwrap(){Tenant::Live(l)=>l,Tenant::Refused(r)=>panic!("{r:?}")};
        live.db().unwrap().catalog_digest().unwrap()
    };
    let expected=match Replica::open(store,"t/tenant-a",&base.join("correct-b"),theory()).unwrap(){Opened::Ready(r)=>r.db().unwrap().catalog_digest().unwrap(),Opened::Refused(r)=>panic!("{r:?}")};
    assert_eq!(a_digest,b_digest);assert_ne!(b_digest,expected);
    println!("TENANT_CASE_LEAK: isolated case-sensitive remote prefixes contain different facts; tenant-a opened cached tenant-A facts through local case-alias");
}
fn main() {
    let base=std::env::args().nth(1).expect("scratch root");let base=Path::new(&base);
    gc_clock_sentinel(); scratch_ancestor(&base.join("scratch-ancestor")); duplicate_ids(); low_id_stall();
    stale_backlink(); fs_case_alias(&base.join("case"));
    stale_writer_recreates_slot(&base.join("stale-slot"));
    checkpoint_component_regression(&base.join("component"));
    checkpointer_restart_retention(&base.join("checkpointer-restart"));
    tenant_case_leak(&base.join("tenant-cache"));
}
```
