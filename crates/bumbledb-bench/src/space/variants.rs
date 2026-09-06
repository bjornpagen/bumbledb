//! SPACE-02: the before/after layout-variant matrix.
//!
//! Each variant is a full storage build over identical generated data with
//! identical semantics, measured through the same census. Baselines are kept
//! **distinct**: the audited tree uses 8-byte fresh IDs and 32-byte
//! membership digests; the superseded earlier proposal used 28-byte IDs.
//! Comparing the successor's 16-byte IDs against both is legitimate;
//! combining the two baselines into one fictitious net saving is not, and the
//! arithmetic here refuses it by carrying the baseline in every delta.

/// Which historical baseline a delta is measured against. There is no
/// "combined" variant on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Baseline {
    /// The audited 0.x tree: 8-byte fresh IDs, 32-byte membership digest,
    /// immortal dictionary.
    Audited0x,
    /// The superseded proposal that carried 28-byte IDs. Only ID-width
    /// comparisons may cite it, and only as "superseded".
    Superseded28ByteIds,
}

/// One measured axis of SPACE-02.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// 32 → 16-byte local membership fingerprint, exact collision handling
    /// added: −16 raw bytes per fact at the membership entry.
    FingerprintWidth,
    /// 8 → 16-byte application-owned IDs against the audited tree
    /// (+8 bytes per occurrence), 28 → 16 against the superseded proposal
    /// (−12 per occurrence). Never netted together.
    IdWidth,
    /// Immortal dictionary versus inline text: repeated long strings can
    /// amortize interning; unique short strings and deleted historical
    /// strings do badly. Both populations are in the matrix.
    TextLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariantCell {
    pub axis: Axis,
    pub baseline: Baseline,
    pub name: &'static str,
    /// The workload regimes each cell must report (chapter 41 SPACE-02):
    /// disk (raw/compacted), RSS, peak scratch, CRUD and warm/post-write/
    /// cold/>RAM costs — encoded as a requirement, checked by the manifest.
    pub regimes: &'static [&'static str],
}

pub const REQUIRED_REGIMES: [&str; 7] = [
    "disk-raw",
    "disk-compacted",
    "rss",
    "peak-scratch",
    "crud",
    "warm-post-write-cold",
    "beyond-ram",
];

/// The complete SPACE-02 matrix.
#[must_use]
pub fn matrix() -> Vec<VariantCell> {
    vec![
        VariantCell {
            axis: Axis::FingerprintWidth,
            baseline: Baseline::Audited0x,
            name: "membership digest 32B -> exact-checked fingerprint 16B",
            regimes: &REQUIRED_REGIMES,
        },
        VariantCell {
            axis: Axis::IdWidth,
            baseline: Baseline::Audited0x,
            name: "fresh 8B IDs -> application-owned 16B IDs (+8B per occurrence)",
            regimes: &REQUIRED_REGIMES,
        },
        VariantCell {
            axis: Axis::IdWidth,
            baseline: Baseline::Superseded28ByteIds,
            name: "superseded 28B IDs -> 16B IDs (-12B per occurrence; historical)",
            regimes: &REQUIRED_REGIMES,
        },
        VariantCell {
            axis: Axis::TextLayout,
            baseline: Baseline::Audited0x,
            name: "immortal dictionary -> inline text, repeated-label population",
            regimes: &REQUIRED_REGIMES,
        },
        VariantCell {
            axis: Axis::TextLayout,
            baseline: Baseline::Audited0x,
            name: "immortal dictionary -> inline text, unique-churn population",
            regimes: &REQUIRED_REGIMES,
        },
    ]
}

/// Raw-byte delta per fact for the fingerprint change: exactly the 16
/// truncated digest bytes at the membership entry (chapter 41's illustrative
/// 9.6%-of-167 / 7.0%-of-228 figures divide by the historical totals — they
/// are arithmetic, not measured file reductions, and the census must measure
/// the end-to-end effect including changed node shapes and collision fetches).
pub const FINGERPRINT_SAVING_PER_FACT: u64 = 16;

/// Signed per-occurrence ID-width delta against a named baseline.
#[must_use]
pub const fn id_width_delta(baseline: Baseline) -> i64 {
    match baseline {
        Baseline::Audited0x => 8,
        Baseline::Superseded28ByteIds => -12,
    }
}

/// Illustrative fingerprint savings at population scale, decimal units
/// (16 MB per million facts; 1.6 GB per 100 million) — arithmetic used by the
/// report prose, cross-checked in tests so the doc numbers cannot drift.
#[must_use]
pub const fn fingerprint_saving_bytes(facts: u64) -> u64 {
    facts * FINGERPRINT_SAVING_PER_FACT
}

/// Validate the opt-in experiment independently of CLI parsing.
/// # Errors
/// Unsupported population/sample counts or an incompatible corpus option.
pub fn validate_home_args(args: &crate::cli::StorageArgs) -> Result<(), String> {
    if !(256..=1_048_576).contains(&args.rows) || !args.rows.is_multiple_of(256) {
        return Err("home-costs --rows must be a multiple of 256 in 256..=1048576".into());
    }
    if !(1..=4096).contains(&args.samples) {
        return Err("home-costs --samples must be in 1..=4096".into());
    }
    if args.churn_dir.is_some() {
        return Err("home-costs cannot consume --churn-dir".into());
    }
    Ok(())
}

/// Measure home width versus secondary count without changing the corpus lane.
/// # Errors
/// Invalid arguments, storage/admission failure, a failed exact oracle, or I/O.
pub fn run_home_costs(args: &crate::cli::StorageArgs) -> Result<i32, String> {
    validate_home_args(args)?;
    home_costs::run(args)
}

// One generated application relation beside the existing layout variants,
// not another fixture/report framework. No production SDK additions.
mod home_costs {
    use crate::cli::StorageArgs;
    use crate::space::census::{self, StoreCensus};
    use crate::space::store_source::StoreCensusSource;
    use crate::space::{NAMESPACES, Namespace};
    use crate::{harness, json};
    use bumbledb::schema::{
        FieldDescriptor, RelationDescriptor, SchemaDescriptor, StatementDescriptor, ValueType,
    };
    use bumbledb::{Admission, Db, FieldId, RelationId, StatementId, Value};
    use std::fmt::Write as _;
    use std::io::Write as _;
    use std::path::{Path, PathBuf};
    use std::time::Instant;

    const RELATION: RelationId = RelationId(0);
    const WRITE_BATCH: usize = 256;
    const READ_BATCH: usize = 32;
    const WARMUPS: usize = 8;
    const PAYLOAD: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    type HomeDb = Db<SchemaDescriptor>;

    #[derive(Clone, Copy)]
    struct Shape {
        uuid: bool,
        secondaries: u16,
        shuffled: bool,
    }

    impl Shape {
        fn width(self) -> u64 {
            if self.uuid { 16 } else { 8 }
        }
        fn label(self) -> String {
            format!(
                "{}-{}-{}",
                if self.uuid { "uuid" } else { "u64" },
                self.secondaries,
                if self.shuffled {
                    "batch-sorted-random"
                } else {
                    "ascending"
                }
            )
        }
    }

    fn descriptor(shape: Shape) -> SchemaDescriptor {
        let mut fields = vec![FieldDescriptor {
            name: "id".into(),
            value_type: if shape.uuid {
                ValueType::Uuid
            } else {
                ValueType::U64
            },
        }];
        fields.extend((1..=4).map(|n| FieldDescriptor {
            name: format!("secondary{n}").into(),
            value_type: ValueType::Uuid,
        }));
        fields.push(FieldDescriptor {
            name: "payload".into(),
            value_type: ValueType::String,
        });
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "Entity".into(),
                fields,
                extension: None,
            }],
            // The export oracle checks the actual selected route's order;
            // it does not reimplement the production selection algorithm.
            statements: (0..=shape.secondaries)
                .map(|field| StatementDescriptor::Functionality {
                    relation: RELATION,
                    projection: Box::from([FieldId(field)]),
                })
                .collect(),
        }
    }

    fn key(uuid: bool, field: usize, id: u64, seed: u64) -> Value {
        if field == 0 && !uuid {
            return Value::U64(id);
        }
        // Odd multiplication modulo 2^62 is bijective. Version/variant bits
        // are fixed outside those bits; miss IDs cannot alias live IDs.
        let word = if field == 0 {
            id
        } else {
            const MULTIPLIERS: [u64; 4] = [
                0x9e37_79b9_7f4a_7c15,
                0xbf58_476d_1ce4_e5b9,
                0x94d0_49bb_1331_11eb,
                0xd6e8_feb8_6659_fd93,
            ];
            id.wrapping_mul(MULTIPLIERS[field - 1])
                .wrapping_add(seed.wrapping_add(field as u64 * 0x0001_0001))
                & ((1u64 << 62) - 1)
        };
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&[0x01, 0x95, 0, 0, 0, 0, 0x70, 0]);
        bytes[8..].copy_from_slice(&(word | (1u64 << 63)).to_be_bytes());
        Value::Uuid(bumbledb::Uuid::from_bytes(bytes))
    }

    fn data(shape: Shape, rows: u64, seed: u64) -> Vec<Vec<Value>> {
        (0..rows)
            .map(|id| {
                let mut row: Vec<_> = (0..=4)
                    .map(|field| key(shape.uuid, field, id, seed))
                    .collect();
                row.push(Value::String(PAYLOAD.into()));
                row
            })
            .collect()
    }

    fn order(rows: usize, shuffled: bool, seed: u64) -> Vec<usize> {
        let mut ids: Vec<_> = (0..rows).collect();
        if shuffled {
            let mut rng = crate::corpus_gen::Rng::new(seed);
            for i in (1..rows).rev() {
                let j = usize::try_from(rng.range((i + 1) as u64)).expect("bounded row index");
                ids.swap(i, j);
            }
        }
        ids
    }

    fn data_digest(db: &HomeDb, rows: &[Vec<Value>]) -> Result<String, String> {
        let work = harness::bench_work();
        let mut digest = bumbledb::digest::Digest::new();
        for row in rows {
            let bytes = bumbledb::canonical::CanonicalRow::encode(
                db.schema().relation(RELATION).fields(),
                row,
                &work,
            )
            .map_err(|error| format!("encode data fingerprint: {error:?}"))?;
            digest.update(&(bytes.len() as u64).to_be_bytes());
            digest.update(&bytes);
        }
        Ok(crate::corpus_gen::digest_hex(&digest.finalize()))
    }

    fn load(db: &HomeDb, rows: &[Vec<Value>], ids: &[usize]) -> Result<Vec<u64>, String> {
        let mut samples = Vec::with_capacity(ids.len() / WRITE_BATCH);
        for batch in ids.chunks(WRITE_BATCH) {
            let work = harness::bench_work();
            let start = Instant::now();
            let result = db.write(work, |tx| {
                tx.insert_dyn(RELATION, batch.iter().map(|&id| rows[id].as_slice()))
            });
            let elapsed = nanos(start);
            match result.map_err(|error| format!("durable batch: {error:?}"))? {
                Admission::Accepted(_) => samples.push(elapsed),
                Admission::Rejected(error) => {
                    return Err(format!("durable batch rejected: {error:?}"));
                }
            }
        }
        Ok(samples)
    }

    fn census(
        db: &HomeDb,
        dir: &Path,
        expected: &[Vec<Value>],
        shape: Shape,
    ) -> Result<StoreCensus, String> {
        let verified = db
            .verify_store()
            .map_err(|error| format!("offline index/law verification: {error:?}"))?;
        if !verified.findings().is_empty() {
            return Err(format!(
                "offline index/law findings: {:?}",
                verified.findings()
            ));
        }
        let work = harness::bench_work();
        let store = db.integration_store();
        let snapshot = store
            .snapshot(&work)
            .map_err(|error| format!("census snapshot: {error:?}"))?;
        let count = snapshot
            .row_count(RELATION)
            .map_err(|error| format!("row count: {error:?}"))?;
        if count != expected.len() as u64 {
            return Err(format!("row count mismatch: {count} vs {}", expected.len()));
        }
        // Both layouts export in the actual selected home's order. UUID
        // secondary permutations ensure selecting a secondary fails this
        // full-row/order oracle, even when it has the same width as primary.
        let mut index = 0;
        snapshot
            .export(&work, &mut |relation, bytes| {
                let row = bumbledb::canonical::decode(
                    db.schema().relation(RELATION).fields(),
                    bytes,
                    &work,
                )?;
                if relation != RELATION
                    || expected.get(index).map(Vec::as_slice) != Some(row.values())
                {
                    return Err(bumbledb::store::StoreError::ForeignSchema);
                }
                index += 1;
                Ok(())
            })
            .map_err(|error| {
                format!("full-row/export-selected-primary oracle at row {index}: {error:?}")
            })?;
        if index != expected.len() {
            return Err(format!(
                "export row count mismatch: {index} vs {}",
                expected.len()
            ));
        }
        let map = store
            .map_report(&work)
            .map_err(|error| format!("map report: {error:?}"))?;
        let mut source = StoreCensusSource::open(&snapshot, &work, map);
        let measured = census::run(&mut source, &dir.join("data.mdb"))?
            .with_map(map.virtual_map_bytes, map.live_transactions);
        check_census(
            bumbledb::store::format::LAYOUT,
            &measured,
            snapshot.physical_key_widths(),
            shape,
            expected.len() as u64,
        )?;
        Ok(measured)
    }

    fn check_census(
        layout: u32,
        measured: &StoreCensus,
        widths: bumbledb::store::PhysicalKeyWidths,
        shape: Shape,
        n: u64,
    ) -> Result<(), String> {
        let facts = measured.namespace(Namespace::Fact);
        let members = measured.namespace(Namespace::Membership);
        let det = measured.namespace(Namespace::Determinant);
        // This generated schema has one scalar primary and `secondaries`
        // distinct scalar UUID keys, never an interval projection. Layout 5
        // elides membership but retains the primary determinant. Layout 6
        // moves that primary into the row key and copies its home into each
        // secondary value. Layout 7 removes interval tails only, so these
        // scalar-only shapes have exactly the same physical bill as layout 6.
        // Derive multiplicities from the descriptor, not measured entries.
        let clustered = match layout {
            5 => false,
            6 | 7 => true,
            other => {
                return Err(format!(
                    "home-costs physical oracle does not cover layout {other}"
                ));
            }
        };
        let row_keys = n * (widths.row as u64 + if clustered { shape.width() } else { 0 });
        let det_entries = n * (u64::from(shape.secondaries) + u64::from(!clustered));
        let det_keys = n
            * (u64::from(shape.secondaries) * (widths.determinant_overhead as u64 + 16)
                + if clustered {
                    0
                } else {
                    widths.determinant_overhead as u64 + shape.width()
                });
        let det_values = if clustered {
            det_entries * shape.width()
        } else {
            0
        };
        if facts.entries != n
            || facts.key_bytes != row_keys
            || members != census::NamespaceCensus::default()
            || det.entries != det_entries
            || det.key_bytes != det_keys
            || det.value_bytes != det_values
            || measured.namespace(Namespace::Unknown) != census::NamespaceCensus::default()
        {
            return Err(format!(
                "layout {layout} physical roster mismatch: facts={facts:?}, members={members:?}, determinants={det:?}"
            ));
        }
        Ok(())
    }

    struct ReadSample {
        statement: u16,
        hit: bool,
        batch_ns: Vec<u64>,
    }

    fn reads(
        db: &HomeDb,
        rows: &[Vec<Value>],
        shape: Shape,
        args: &StorageArgs,
    ) -> Result<Vec<ReadSample>, String> {
        let pin_work = harness::bench_work();
        let snapshot = db
            .snapshot(&pin_work)
            .map_err(|error| format!("read snapshot: {error:?}"))?;
        let mut rng = crate::corpus_gen::Rng::new(args.seed);
        let mut measured = Vec::new();
        for statement in 0..=shape.secondaries {
            for hit in [true, false] {
                let mut batch_ns = Vec::with_capacity(args.samples as usize);
                for sample in 0..WARMUPS + args.samples as usize {
                    let ids: Vec<_> = (0..READ_BATCH)
                        .map(|_| rng.range(rows.len() as u64))
                        .collect();
                    let keys: Vec<_> = ids
                        .iter()
                        .map(|&id| {
                            [key(
                                shape.uuid,
                                usize::from(statement),
                                if hit { id } else { args.rows + id },
                                args.seed,
                            )]
                        })
                        .collect();
                    let work = harness::bench_work();
                    let mut results = Vec::with_capacity(READ_BATCH);
                    let start = Instant::now();
                    for values in &keys {
                        results.push(snapshot.get_dyn(
                            RELATION,
                            StatementId(statement),
                            values,
                            &work,
                        ));
                    }
                    let elapsed = nanos(start);
                    // Parameter construction, verification, result dropping,
                    // work creation and snapshot admission are outside timing.
                    for (result, id) in results.into_iter().zip(ids) {
                        let row = result.map_err(|error| {
                            format!("statement {statement} hit={hit}: {error:?}")
                        })?;
                        match row {
                            Some(row)
                                if hit
                                    && row.values()
                                        == rows
                                            [usize::try_from(id).expect("bounded row index")]
                                        .as_slice() => {}
                            None if !hit => {}
                            _ => {
                                return Err(format!(
                                    "statement {statement} hit={hit}: full-row oracle mismatch for id {id}"
                                ));
                            }
                        }
                    }
                    if sample >= WARMUPS {
                        batch_ns.push(elapsed);
                    }
                }
                measured.push(ReadSample {
                    statement,
                    hit,
                    batch_ns,
                });
            }
        }
        Ok(measured)
    }

    fn push_samples(out: &mut String, samples: &[u64], operations: usize) {
        let _ = write!(
            out,
            "{{\"operations_per_sample\":{operations},\"raw_batch_ns\":{samples:?},\"batch_ns\":"
        );
        let mut sorted = samples.to_vec();
        crate::lanes::push_stats(out, &harness::stats(&mut sorted));
        out.push('}');
    }

    fn push_reads(out: &mut String, reads: &[ReadSample]) {
        out.push('[');
        for (index, sample) in reads.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                "{{\"statement\":{},\"hit\":{},\"timing\":",
                sample.statement, sample.hit
            );
            push_samples(out, &sample.batch_ns, READ_BATCH);
            out.push('}');
        }
        out.push(']');
    }

    fn push_census(out: &mut String, measured: &StoreCensus) {
        let pages = measured.pages;
        let _ = write!(
            out,
            "{{\"file_bytes\":{},\"allocated_disk_bytes\":{},\"live_raw_bytes\":{},\"used_page_bytes\":{},\"free_page_bytes\":{},\"page_overhead_bytes\":{},\"page_size\":{},\"depth\":{},\"branch_pages\":{},\"leaf_pages\":{},\"overflow_pages\":{},\"free_pages\":{},\"entries\":{}",
            measured.file_bytes,
            measured.allocated_bytes,
            measured.live_raw_bytes(),
            pages.used_page_bytes(),
            pages.free_page_bytes(),
            measured.page_overhead_bytes(),
            pages.page_size,
            pages.depth,
            pages.branch_pages,
            pages.leaf_pages,
            pages.overflow_pages,
            pages.free_pages,
            pages.entries
        );
        out.push_str(",\"virtual_map_bytes\":");
        match measured.virtual_map_bytes {
            Some(n) => {
                let _ = write!(out, "{n}");
            }
            None => out.push_str("null"),
        }
        out.push_str(",\"live_transactions\":");
        match measured.live_transactions {
            Some(n) => {
                let _ = write!(out, "{n}");
            }
            None => out.push_str("null"),
        }
        out.push_str(",\"namespaces\":{");
        for (index, namespace) in NAMESPACES.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            json::push_str_lit(out, namespace.label());
            let cell = measured.namespace(*namespace);
            let _ = write!(
                out,
                ":{{\"entries\":{},\"key_bytes\":{},\"value_bytes\":{}}}",
                cell.entries, cell.key_bytes, cell.value_bytes
            );
        }
        out.push_str("}}");
    }

    fn cell(root: &Path, shape: Shape, args: &StorageArgs) -> Result<String, String> {
        let desc = descriptor(shape);
        let raw_dir = root.join(format!("{}-raw", shape.label()));
        let compact_dir = root.join(format!("{}-compact", shape.label()));
        let db = match Db::create(&raw_dir, desc.clone(), harness::bench_work())
            .map_err(|error| format!("create: {error:?}"))?
        {
            Admission::Accepted(db) => db,
            Admission::Rejected(error) => return Err(format!("empty schema rejected: {error:?}")),
        };
        let rows = data(shape, args.rows, args.seed);
        let ids = order(rows.len(), shape.shuffled, args.seed);
        let digest = data_digest(&db, &rows)?;
        let mut order_hash = bumbledb::digest::Digest::new();
        for &id in &ids {
            order_hash.update(&(id as u64).to_be_bytes());
        }
        let schema = bumbledb::schema::fingerprint::fingerprint(db.schema()).to_string();
        let (load_samples, load_clock) = crate::clockproxy::stamped(|| load(&db, &rows, &ids))?;
        let raw = census(&db, &raw_dir, &rows, shape)?;
        let (raw_reads, raw_clock) = crate::clockproxy::stamped(|| reads(&db, &rows, shape, args))?;
        db.compact(&compact_dir, harness::bench_work())
            .map_err(|error| format!("compact: {error:?}"))?;
        drop(db);
        let db = Db::open(&compact_dir, desc, harness::bench_work())
            .map_err(|error| format!("reopen compact: {error:?}"))?;
        let compact = census(&db, &compact_dir, &rows, shape)?;
        if raw.per_namespace != compact.per_namespace {
            return Err("compaction changed live namespace entries/bytes".into());
        }
        let (compact_reads, compact_clock) =
            crate::clockproxy::stamped(|| reads(&db, &rows, shape, args))?;
        drop(db);
        let mut out = String::new();
        let _ = write!(
            out,
            "{{\"primary\":\"{}\",\"home_bytes\":{},\"secondaries\":{},\"insertion\":\"{}\",\"schema_digest\":\"{schema}\",\"data_digest\":\"{digest}\",\"insertion_digest\":\"{}\",\"durable_insert\":",
            if shape.uuid { "uuid" } else { "u64" },
            shape.width(),
            shape.secondaries,
            if shape.shuffled {
                "batch-sorted-random"
            } else {
                "ascending"
            },
            crate::corpus_gen::digest_hex(&order_hash.finalize())
        );
        push_samples(&mut out, &load_samples, WRITE_BATCH);
        out.push_str(",\"raw\":");
        push_census(&mut out, &raw);
        out.push_str(",\"raw_reads\":");
        push_reads(&mut out, &raw_reads);
        out.push_str(",\"compacted\":");
        push_census(&mut out, &compact);
        out.push_str(",\"compacted_reads\":");
        push_reads(&mut out, &compact_reads);
        out.push_str(",\"clock_proxy\":{");
        for (index, (label, stamp)) in [
            ("durable_insert", load_clock),
            ("raw_reads", raw_clock),
            ("compacted_reads", compact_clock),
        ]
        .into_iter()
        .enumerate()
        {
            if index != 0 {
                out.push(',');
            }
            json::push_str_lit(&mut out, label);
            let _ = write!(out, ":{{\"pre\":{},\"post\":{}}}", stamp.pre, stamp.post);
        }
        out.push('}');
        out.push('}');
        // Retain both newly created stores for physical follow-up diagnosis.
        Ok(out)
    }

    pub(super) fn run(args: &StorageArgs) -> Result<i32, String> {
        let out_dir = args.out.clone().unwrap_or_else(|| {
            PathBuf::from("bench-out").join(format!(
                "{}-home-costs",
                crate::report::timestamp_iso8601().replace(':', "-")
            ))
        });
        std::fs::create_dir_all(&out_dir).map_err(|error| format!("home-costs output: {error}"))?;
        let report_path = out_dir.join("home-costs.json");
        if report_path.exists() {
            return Err(format!("refuse existing report {}", report_path.display()));
        }
        std::fs::create_dir_all(&args.dir)
            .map_err(|error| format!("home-costs scratch parent: {error}"))?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| format!("clock before epoch: {error}"))?
            .as_nanos();
        let scratch = args
            .dir
            .join(format!("home-costs-{}-{stamp}", std::process::id()));
        // A collision refuses before any store is opened/deleted. Failures
        // retain this owned scratch path for diagnosis, never a success report.
        std::fs::create_dir(&scratch).map_err(|error| format!("home-costs scratch: {error}"))?;
        let mut out = String::from(
            "{\"experiment\":\"home-costs-v1\",\"status\":\"verified\",\"provenance\":",
        );
        crate::report::push_provenance(&mut out, &crate::report::provenance(Path::new(".")));
        out.push_str(",\"scratch_root\":");
        json::push_str_lit(&mut out, &scratch.to_string_lossy());
        let _ = write!(
            out,
            ",\"layout\":{},\"binary_digest\":\"{}\",\"rows\":{},\"seed\":{},\"samples\":{},\"payload_bytes\":64,\"read_warmup_batches\":{WARMUPS}",
            bumbledb::store::format::LAYOUT,
            crate::corpus_gen::digest_hex(&crate::verify::binary_fingerprint()),
            args.rows,
            args.seed,
            args.samples
        );
        out.push_str(",\"method\":\"Durable 256-row SDK writes. Random IDs are shuffled globally then canonically sorted within each transaction. Reads use a pinned SDK snapshot; 32 gets per timed batch, full returned rows checked outside timing. Census/export verification precedes warm reads. All four UUID secondary fields remain present.\",\"limitations\":\"Resident diagnostic, not a cold or beyond-RAM result. No clock-proxy normalization, RSS or significance claim. Batch samples share one growing store or one pinned snapshot; they are not independent store replicates. Run captured binaries serially in A/B/B/A order with matching schema/data/insertion digests. Binary digest identifies the executable; git_rev describes the launch checkout, not uncommitted build contents.\",\"cells\":[");
        let mut first = true;
        for uuid in [false, true] {
            for secondaries in [0, 1, 2, 4] {
                for shuffled in [false, true] {
                    let shape = Shape {
                        uuid,
                        secondaries,
                        shuffled,
                    };
                    eprintln!("home-costs: {} / {} rows", shape.label(), args.rows);
                    let result = cell(&scratch, shape, args).map_err(|error| {
                        format!(
                            "home-costs {}: {error}; owned scratch retained at {}",
                            shape.label(),
                            scratch.display()
                        )
                    })?;
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    out.push_str(&result);
                }
            }
        }
        out.push_str("]}\n");
        // Publish only a fully written, synced JSON document. Hard-linking
        // within the artifact directory refuses a concurrent existing report.
        let pending = out_dir.join(format!(".home-costs-{stamp}.json"));
        let mut report = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending)
            .map_err(|error| format!("create pending report: {error}"))?;
        report
            .write_all(out.as_bytes())
            .map_err(|error| format!("write report: {error}"))?;
        report
            .sync_all()
            .map_err(|error| format!("sync report: {error}"))?;
        drop(report);
        std::fs::hard_link(&pending, &report_path)
            .map_err(|error| format!("publish report: {error}"))?;
        std::fs::remove_file(&pending)
            .map_err(|error| format!("remove pending report: {error}"))?;
        println!("{}", report_path.display());
        Ok(0)
    }

    fn nanos(start: Instant) -> u64 {
        u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use bumbledb::schema::ValidateDescriptor as _;

        #[test]
        fn generated_keys_are_exact_unique_and_secondaries_cannot_be_narrower() {
            for uuid in [false, true] {
                for secondaries in [0, 1, 2, 4] {
                    let shape = Shape {
                        uuid,
                        secondaries,
                        shuffled: false,
                    };
                    let schema = descriptor(shape).validate().unwrap();
                    let compiled = schema.compiled_theory().unwrap();
                    assert_eq!(compiled.projections().len(), usize::from(secondaries) + 1);
                    assert_eq!(
                        compiled
                            .projection_of_statement(StatementId(0))
                            .unwrap()
                            .encoding
                            .routing_width(),
                        usize::try_from(shape.width()).unwrap()
                    );
                    for field in 0..=4 {
                        let values: std::collections::BTreeSet<_> = (0..512)
                            .map(|id| {
                                bumbledb::store::encode_scalar_group(
                                    &[key(uuid, field, id, 7)],
                                    std::slice::from_ref(
                                        &schema.relation(RELATION).fields()[field],
                                    ),
                                )
                                .unwrap()
                            })
                            .collect();
                        assert_eq!(values.len(), 512);
                    }
                    for statement in 1..=secondaries {
                        assert_eq!(
                            compiled
                                .projection_of_statement(StatementId(statement))
                                .unwrap()
                                .encoding
                                .routing_width(),
                            16
                        );
                    }
                }
            }
        }

        // Pure census inputs: no database, filesystem or timing work. Literal
        // bytes below describe the one-relation/one-byte-projection schema,
        // independently of the production-width metadata supplied to the oracle.
        fn physical_census(
            n: u64,
            row_key: u64,
            det_entries: u64,
            det_keys: u64,
            det_values: u64,
        ) -> StoreCensus {
            let mut per_namespace = [census::NamespaceCensus::default(); NAMESPACES.len()];
            for (namespace, cell) in [
                (
                    Namespace::Fact,
                    census::NamespaceCensus {
                        entries: n,
                        key_bytes: n * row_key,
                        value_bytes: 0, // Full canonical rows have their own export oracle.
                    },
                ),
                (
                    Namespace::Determinant,
                    census::NamespaceCensus {
                        entries: n * det_entries,
                        key_bytes: n * det_keys,
                        value_bytes: n * det_values,
                    },
                ),
            ] {
                let index = NAMESPACES
                    .iter()
                    .position(|&kind| kind == namespace)
                    .unwrap();
                per_namespace[index] = cell;
            }
            StoreCensus {
                per_namespace,
                pages: census::PageStats::default(),
                file_bytes: 0,
                allocated_bytes: 0,
                virtual_map_bytes: None,
                live_transactions: None,
            }
        }

        #[test]
        fn home_census_covers_scalar_layout7_and_preserves_layout5_and6_bills() {
            for uuid in [false, true] {
                for secondaries in [0, 1, 2, 4] {
                    let shape = Shape {
                        uuid,
                        secondaries,
                        shuffled: false,
                    };
                    let schema = descriptor(shape).validate().unwrap();
                    let widths = bumbledb::store::PhysicalKeyWidths::for_schema(&schema).unwrap();
                    let secondary_count = u64::from(secondaries);
                    let home = if uuid { 16 } else { 8 };
                    let clustered = physical_census(
                        256,
                        if uuid { 26 } else { 18 },
                        secondary_count,
                        secondary_count * 26,
                        secondary_count * home,
                    );
                    for layout in [6, 7, bumbledb::store::format::LAYOUT] {
                        check_census(layout, &clustered, widths, shape, 256).unwrap();
                    }
                    let unclustered = physical_census(
                        256,
                        10,
                        secondary_count + 1,
                        secondary_count * 26 + if uuid { 26 } else { 18 },
                        0,
                    );
                    check_census(5, &unclustered, widths, shape, 256).unwrap();
                    assert!(check_census(5, &clustered, widths, shape, 256).is_err());
                    assert!(check_census(7, &unclustered, widths, shape, 256).is_err());
                    assert!(check_census(8, &clustered, widths, shape, 256).is_err());
                }
            }
        }

        #[test]
        fn home_census_rejects_wrong_widths_missing_indexes_and_unexpected_namespaces() {
            let shape = Shape {
                uuid: false,
                secondaries: 2,
                shuffled: false,
            };
            let schema = descriptor(shape).validate().unwrap();
            let widths = bumbledb::store::PhysicalKeyWidths::for_schema(&schema).unwrap();
            let good = physical_census(256, 18, 2, 52, 16);
            check_census(7, &good, widths, shape, 256).unwrap();
            for bad in [
                physical_census(256, 26, 2, 52, 16), // Wrong primary home width.
                physical_census(256, 18, 2, 84, 16), // Spurious interval tails.
                physical_census(256, 18, 1, 26, 8),  // Missing secondary.
                physical_census(256, 18, 3, 70, 24), // Unelided primary.
                physical_census(256, 18, 2, 52, 0),  // Missing secondary homes.
            ] {
                assert!(check_census(7, &bad, widths, shape, 256).is_err());
            }
            for namespace in [Namespace::Membership, Namespace::Unknown] {
                let index = NAMESPACES
                    .iter()
                    .position(|&kind| kind == namespace)
                    .unwrap();
                for cell in [
                    census::NamespaceCensus {
                        entries: 1,
                        key_bytes: 26,
                        value_bytes: 0,
                    },
                    census::NamespaceCensus {
                        entries: 0,
                        key_bytes: 1,
                        value_bytes: 0,
                    },
                ] {
                    let mut bad = good.clone();
                    bad.per_namespace[index] = cell;
                    assert!(check_census(7, &bad, widths, shape, 256).is_err());
                }
            }
            for bad_widths in [
                bumbledb::store::PhysicalKeyWidths {
                    row: widths.row + 1,
                    ..widths
                },
                bumbledb::store::PhysicalKeyWidths {
                    determinant_overhead: widths.determinant_overhead + 1,
                    ..widths
                },
            ] {
                assert!(check_census(7, &good, bad_widths, shape, 256).is_err());
            }
        }

        #[test]
        fn insertion_order_is_reproducible_and_changes_batches_not_the_row_set() {
            let ascending = order(1024, false, 7);
            let shuffled = order(1024, true, 7);
            assert_eq!(shuffled, order(1024, true, 7));
            assert_ne!(shuffled, ascending);
            assert_ne!(
                shuffled[..WRITE_BATCH]
                    .iter()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>(),
                ascending[..WRITE_BATCH].iter().copied().collect()
            );
            let mut sorted = shuffled;
            sorted.sort_unstable();
            assert_eq!(sorted, ascending);
        }

        #[test]
        fn timing_json_preserves_acquisition_order_and_labels_batch_units() {
            let mut output = String::new();
            push_samples(&mut output, &[30, 10, 20], READ_BATCH);
            let parsed = json::parse(&output).unwrap();
            assert_eq!(
                parsed
                    .get("operations_per_sample")
                    .and_then(json::Value::as_f64),
                Some(32.0)
            );
            assert_eq!(
                parsed
                    .get("raw_batch_ns")
                    .and_then(json::Value::as_arr)
                    .unwrap(),
                &[
                    json::Value::Num(30.0),
                    json::Value::Num(10.0),
                    json::Value::Num(20.0)
                ]
            );
            assert_eq!(
                parsed
                    .get("batch_ns")
                    .unwrap()
                    .get("p50")
                    .and_then(json::Value::as_f64),
                Some(20.0)
            );
        }
    }
}
