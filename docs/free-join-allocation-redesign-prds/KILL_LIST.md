# Allocation Kill List

These are blocking architecture defects. Do not proceed to planner sophistication, storage accelerators, NEON, or API cleanup until this list is cleared.

## Kill Criteria

An item is killed only when all are true:

- Production hot-path code no longer contains the listed pattern.
- Replacement uses compact IDs, borrowed references, stack/scratch buffers, dense slots, or arena ranges as appropriate.
- Existing correctness tests pass.
- Focused allocation fixture proves the allocation class changed.
- No-query-tracing JOB allocation gates pass.

## P0 Kill List

| ID | Kill Target | Evidence | Replacement | Passing Proof |
| --- | --- | --- | --- | --- |
| KILL-001 | Production COLT shared lock state | `crates/bumbledb-lmdb/src/colt.rs:25-30` uses `Arc<Mutex<ColtState>>` and `Arc<[usize]>` in `ColtSource` | Query-local arena owner plus `Copy` source handles containing arena/node/schema IDs | Source handle clone is integer copy only; no `Arc`, `Mutex`, `Rc`, or `RefCell` in production COLT source handles |
| KILL-002 | Production COLT `HashMap` map state | `crates/bumbledb-lmdb/src/colt.rs:45-50` and `crates/bumbledb-lmdb/src/colt.rs:350` use `HashMap<KeyOwned, usize>` | Arena-owned flat map table with bucket and entry ranges | `rg "HashMap" crates/bumbledb-lmdb/src/colt.rs crates/bumbledb-lmdb/src/colt/ght.rs` has no production hot-path match |
| KILL-003 | Per-node/per-child offset vectors | `crates/bumbledb-lmdb/src/colt.rs:48`, `crates/bumbledb-lmdb/src/colt.rs:135-145`, and `crates/bumbledb-lmdb/src/colt.rs:389-395` use `Vec<u32>` offset storage | `Range`, `Singleton`, and offset-pool `OffsetRange` only | Duplicate-heavy force stores singleton children without child vectors; many-offset child stores one arena range |
| KILL-004 | Key cloning before map iteration | `crates/bumbledb-lmdb/src/colt/ght.rs:28-35` clones map keys into a `Vec` before iteration | Streaming map-entry iterator over arena entries with borrowed keys | Iterating map keys does not allocate and does not materialize all keys |
| KILL-005 | Heap tuple batches | `crates/bumbledb-lmdb/src/colt/ght.rs:51-58`, `crates/bumbledb-lmdb/src/tuple.rs:161-164` use `Vec<EncodedTuple>` batches | Bounded arena batch of offsets/key refs plus reusable tuple scratch | Batch fill allocation bounded by batch size, not source cardinality |
| KILL-006 | Heap-owned `EncodedTuple` core type in hot path | `crates/bumbledb-lmdb/src/tuple.rs:100-117` stores `EncodedTuple { bytes: Vec<u8> }` | Borrowed key refs and inline/scratch key buffers in hot paths | No force/probe/iteration hot path constructs `EncodedTuple` unless final bounded output requires ownership |
| KILL-007 | Recursive probe-key allocation | `crates/bumbledb-lmdb/src/query/runtime_keys.rs:7-25`, `crates/bumbledb-lmdb/src/query/runtime_keys.rs:28-40`, `crates/bumbledb-lmdb/src/query/runtime.rs:246`, `crates/bumbledb-lmdb/src/query/runtime.rs:453`, and `crates/bumbledb-lmdb/src/query/runtime_vectorized.rs:141` allocate owned probe keys | `KeyScratch` or borrowed binding key refs reused per frame | Repeated probe fixture for 8-byte and 16-byte keys allocates zero or bounded setup only |
| KILL-008 | Source frames as maps | `crates/bumbledb-lmdb/src/query/runtime.rs:71`, `crates/bumbledb-lmdb/src/query/runtime_frame.rs:17-37`, and `crates/bumbledb-lmdb/src/query/cover.rs:54-120` use `BTreeMap<AtomOccurrenceId, ColtSource>` | Dense atom-indexed source slots plus compact source undo stack | Hot recursive execution does not clone maps and does not perform tree lookup per source access |
| KILL-009 | Projection dedup as `BTreeSet<Vec<u8>>` | `crates/bumbledb-lmdb/src/query/sink.rs:162-165`, `crates/bumbledb-lmdb/src/query/sink.rs:199-202`, and `crates/bumbledb-lmdb/src/query/sink.rs:320-335` allocate projection bytes per consume/check | Projection scratch encoder plus compact dedup table keyed by inline/scratch keys | Duplicate witness fixture remains duplicate-free and projection dedup allocation drops |
| KILL-010 | Base-image cache and scope heap churn | `crates/bumbledb-lmdb/src/base_image.rs:64-75`, `crates/bumbledb-lmdb/src/base_image.rs:106-114`, and `crates/bumbledb-lmdb/src/base_image.rs:165-169` use `Mutex<BTreeMap<...>>`, owned field vectors, and `Arc` cache entries | Dense field bitsets/small arrays and snapshot-local cache keyed without owned field vectors | Base image lookup avoids heap allocation for common field scopes |
| KILL-011 | Storage key builders returning `Vec<u8>` | `crates/bumbledb-lmdb/src/storage_format.rs:50-185` returns owned keys for every storage operation | `KeyScratch`/stack arrays with `write_*_key(out: &mut Vec<u8>)` APIs | Insert/delete/base-image load reuse key buffers instead of allocating per key |
| KILL-012 | Encoded fact as `Vec<Vec<u8>>` plus cloned relation descriptor | `crates/bumbledb-lmdb/src/storage_v5_codec.rs:26-31`, `crates/bumbledb-lmdb/src/storage_v5_codec.rs:60-83`, and `crates/bumbledb-lmdb/src/storage_v5_codec.rs:100-111` allocate per field and clone schema metadata | Encoded fact borrows relation descriptor and stores field ranges into one fact buffer | Insert/delete encode one contiguous fact buffer with no per-field heap vectors |
| KILL-013 | Value cloning during write encoding | `crates/bumbledb-lmdb/src/storage_v5_codec.rs:123-147` clones every supplied `Value` | Encode from `&Value` or generated serial enum without cloning user data | Insert path for supplied values performs no `Value::String` or `Value::Bytes` clone |
| KILL-014 | Primitive encoding returning heap `Vec<u8>` | `crates/bumbledb-lmdb/src/storage_v5_codec.rs:150-187` and `crates/bumbledb-lmdb/src/storage_v5_codec.rs:240-268` allocate for fixed-width encoded values | Encode into `[u8; 8]`, `[u8; 1]`, or caller scratch buffer | Input/source-filter encoding for fixed-width values allocates zero heap objects |
| KILL-015 | Dictionary lookup returning owned raw bytes | `crates/bumbledb-lmdb/src/storage_v5_codec.rs:378-381` returns `Vec<u8>` | Borrow from LMDB txn where lifetime allows, or decode directly into caller output only at public materialization | Dictionary comparison/lookup path avoids copying raw value bytes unless public `Value` is produced |
| KILL-016 | Planner debug snapshots and plan cloning | `crates/bumbledb-lmdb/src/query/binary2fj.rs:127-128`, `crates/bumbledb-lmdb/src/query/binary2fj.rs:181-189`, `crates/bumbledb-lmdb/src/query/binary2fj.rs:263-281`, and `crates/bumbledb-lmdb/src/query/planner_select.rs:98-104` allocate debug strings and clone candidates | Trace-only rewrite diagnostics behind compile-time tracing; choose by index or move selected candidate | Planning in no-trace mode does not allocate rewrite debug snapshots |
| KILL-017 | Plan representation as nested heap vectors everywhere | `crates/bumbledb-lmdb/src/query/free_join.rs:8-71` and validation paths use nested `Vec`s for subatoms, vars, covers, partitions | Compact plan arena with ranges into contiguous subatom/var/field arrays | Plan validation produces dense arrays and avoids per-node/per-subatom heap allocations |
| KILL-018 | Public write API takes `Fact` by value | `crates/bumbledb-lmdb/src/lib.rs:322-328` takes owned `Fact` for insert/delete | Borrowed fact view API or `FactRef` for write boundary | Insert/delete can encode caller-owned facts without moving/cloning whole value maps |

## P1 Kill List

| ID | Kill Target | Evidence | Replacement | Passing Proof |
| --- | --- | --- | --- | --- |
| KILL-019 | Public `Fact` representation as `BTreeMap<String, Value>` | `crates/bumbledb-lmdb/src/lib.rs:341-346` | Ordered field vector or schema-indexed borrowed fact view | Fact construction no longer implies tree map allocation for hot ETL path |
| KILL-020 | Public `Value::String(String)` and `Value::Bytes(Vec<u8>)` in write-heavy APIs | `crates/bumbledb-lmdb/src/lib.rs:379-396` | Borrowed `ValueRef` for write/query inputs plus owned `Value` only for final output | Write path can accept borrowed string/bytes values |
| KILL-021 | `QueryResultSet::new` full sort/dedup of nested vectors | `crates/bumbledb-lmdb/src/lib.rs:486-490` | Sink produces canonical ordered unique output directly where possible | Final materialization avoids redundant sort/dedup when sink already canonicalized |
| KILL-022 | Query builder clones `ValueType`, literals, and names aggressively | `crates/bumbledb-core/src/query_builder.rs:177-270`, `crates/bumbledb-core/src/query_builder.rs:284-319`, and `crates/bumbledb-core/src/query_builder.rs:336-344` | Builder stores descriptor IDs and interns variable/input names | Query construction allocations are outside runtime, but should not define engine internals |
| KILL-023 | Schema descriptors reused as hot-path owned strings/vectors | `crates/bumbledb-core/src/schema/descriptors.rs:3-16`, `crates/bumbledb-core/src/schema/descriptors.rs:120-133`, and `crates/bumbledb-core/src/schema/descriptors.rs:260-304` | Compiled schema view with IDs, slices, and fixed-width metadata | Runtime never clones logical descriptors to do storage/query work |
| KILL-024 | Benchmark/test harness cloning full datasets | `crates/bumbledb-bench/src/runner.rs:87-90`, `crates/bumbledb-bench/src/job/load.rs:7-257`, and `crates/bumbledb-test-support/src/lib.rs:126-195` | Stream/load borrowed rows where practical | Bench allocations are separated from query allocation gates |

## Non-Negotiable Blockers Before New Feature Work

- KILL-001 through KILL-009 must be cleared before any more planner/source-filter/storage-accelerator/NEON work.
- KILL-010 through KILL-015 must be cleared before claiming storage/query allocation architecture is clean.
- KILL-016 through KILL-018 must be cleared before final API cleanup or performance ratchet.
- KILL-019 through KILL-024 can follow, but they must not be used to excuse runtime heap churn.
