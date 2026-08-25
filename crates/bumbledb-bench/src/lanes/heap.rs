use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use bumbledb::ir::{Atom, AtomSource, FindTerm, Query, Rule, Term, VarId};
use bumbledb::schema::FieldId;
use bumbledb::{Answers, Db, InstanceBuilder, RelationId};

use crate::cli::HeapArgs;
use crate::corpus_gen::{self, GenConfig, MANDATE_SEGMENTS, Sizes};
use crate::harness::{self, Protocol, Stats};
use crate::json;
use crate::report::Provenance;
use crate::schema::{Account, AccountId, Ledger, ids};

pub const DEFAULT_PREFIXES: [u64; 4] = [256, 1_024, 4_096, 16_384];

#[derive(Debug, Clone, PartialEq)]
pub struct HeapReport {
    pub provenance: Provenance,
    pub scale: &'static str,
    pub seed: u64,
    pub samples: u32,
    pub point_reads: Vec<PointRow>,
    pub admission: Vec<AdmitRow>,
    pub publish_ns: u64,
    pub join_ns: u64,
    pub join_rows: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PointRow {
    pub name: String,
    pub heap: Stats,
    pub lmdb: Stats,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdmitRow {
    pub facts: u64,
    pub wall_ns: u64,
    pub facts_per_sec: f64,
    pub ns_per_fact: f64,
}

fn push_stats(out: &mut String, stats: &Stats) {
    super::push_stats(out, stats);
}

#[must_use]
pub fn to_json(report: &HeapReport) -> String {
    let mut out = String::new();
    out.push_str("{\"provenance\":");
    super::push_provenance(&mut out, &report.provenance);
    let _ = write!(
        out,
        ",\"scale\":\"{}\",\"seed\":{},\"samples\":{},\"point_reads\":[",
        report.scale, report.seed, report.samples
    );
    for (i, row) in report.point_reads.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"name\":");
        json::push_str_lit(&mut out, &row.name);
        out.push_str(",\"heap\":");
        push_stats(&mut out, &row.heap);
        out.push_str(",\"lmdb\":");
        push_stats(&mut out, &row.lmdb);
        out.push('}');
    }
    out.push_str("],\"admission\":[");
    for (i, row) in report.admission.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            "{{\"facts\":{},\"wall_ns\":{},\"facts_per_sec\":{:.2},\"ns_per_fact\":{:.2}}}",
            row.facts, row.wall_ns, row.facts_per_sec, row.ns_per_fact
        );
    }
    let _ = write!(
        out,
        "],\"publish_ns\":{},\"join_ns\":{},\"join_rows\":{}}}",
        report.publish_ns, report.join_ns, report.join_rows
    );
    out
}

fn to_markdown(report: &HeapReport) -> String {
    let mut out = String::new();
    out.push_str("# Heap-arm ladder\n\n");
    let _ = writeln!(
        out,
        "scale {} · seed {} · samples {}\n",
        report.scale, report.seed, report.samples
    );
    out.push_str("## Frozen vs LMDB point reads\n\n");
    out.push_str("| family | heap p50 ns | lmdb p50 ns | heap/lmdb |\n");
    out.push_str("| --- | ---: | ---: | ---: |\n");
    for row in &report.point_reads {
        #[allow(
            clippy::cast_precision_loss,
            reason = "p50 ns becomes a printed ratio; mantissa loss is below the table's two decimals"
        )]
        let ratio = row.heap.p50 as f64 / (row.lmdb.p50 as f64).max(1.0);
        let _ = writeln!(
            out,
            "| {} | {} | {} | {:.2}× |",
            row.name, row.heap.p50, row.lmdb.p50, ratio
        );
    }
    let _ = writeln!(
        out,
        "\njoin {} ns / {} rows · fromInstance {} ns\n",
        report.join_ns, report.join_rows, report.publish_ns
    );
    out.push_str("## Admission prefixes\n\n");
    out.push_str("| facts | wall ns | facts/s | ns/fact |\n");
    out.push_str("| ---: | ---: | ---: | ---: |\n");
    for row in &report.admission {
        let _ = writeln!(
            out,
            "| {} | {} | {:.0} | {:.1} |",
            row.facts, row.wall_ns, row.facts_per_sec, row.ns_per_fact
        );
    }
    if let Some(growth) = superlinear_term(&report.admission) {
        let _ = writeln!(out, "\nSuperlinear term: {growth}\n");
    } else {
        out.push_str("\nNo unexplained superlinear term on this prefix ladder.\n");
    }
    out.push_str("\nBare-metal ramdisk row rides issue 18's release checklist beside this lane.\n");
    out
}

fn superlinear_term(rows: &[AdmitRow]) -> Option<String> {
    let first = rows.first()?;
    let last = rows.last()?;
    if first.ns_per_fact <= 0.0 || last.facts <= first.facts {
        return None;
    }
    let growth = last.ns_per_fact / first.ns_per_fact;
    (growth > 2.0).then(|| {
        format!(
            "ns/fact grew {growth:.2}× from {} to {} facts",
            first.facts, last.facts
        )
    })
}

fn sizes_for_postings(postings: u64) -> Sizes {
    let accounts = (postings / 200).max(1);
    let orgs = 8u64;
    Sizes {
        postings,
        entries: (postings / 2).max(1),
        accounts,
        holders: (accounts / 4).max(1),
        instruments: postings.clamp(1, 32),
        orgs,
        org_parents: orgs - 1,
        posting_tags: postings,
        mandates: accounts * MANDATE_SEGMENTS,
    }
}

fn fact_count(sizes: &Sizes) -> u64 {
    (0..ids::RELATIONS)
        .map(|rel| sizes.rows(RelationId(rel)))
        .sum()
}

fn load_builder(cfg: GenConfig, sizes: &Sizes) -> Result<InstanceBuilder<Ledger>, String> {
    let mut builder = InstanceBuilder::new(Ledger).map_err(|e| format!("builder: {e:?}"))?;
    for rel in 0..ids::RELATIONS {
        let rel = RelationId(rel);
        let n = sizes.rows(rel);
        let rows = (0..n).map(|i| corpus_gen::row(&cfg, sizes, rel, i));
        builder
            .load_dyn(rel, rows)
            .map_err(|e| format!("load {rel:?}: {e:?}"))?;
    }
    Ok(builder)
}

fn join_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: AtomSource::Edb(ids::ACCOUNT),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: AtomSource::Edb(ids::HOLDER),
                bindings: vec![(FieldId(0), Term::Var(VarId(1)))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

/// # Errors
/// # Panics
#[allow(
    clippy::too_many_lines,
    reason = "one run body owns the heap-arm report; splitting would hide the lane order"
)]
pub fn run(args: &HeapArgs) -> Result<i32, String> {
    let out_dir = args.out.clone().unwrap_or_else(|| {
        PathBuf::from("bench-out").join(format!(
            "{}-heap",
            crate::report::timestamp_iso8601().replace(':', "-")
        ))
    });
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("out dir: {e}"))?;
    let scratch = out_dir.join("scratch");
    std::fs::create_dir_all(&scratch).map_err(|e| format!("scratch: {e}"))?;

    let proto = Protocol {
        warmups: 8,
        samples: args.samples.unwrap_or(32),
    };
    let cfg = GenConfig {
        seed: args.seed,
        scale: args.scale,
    };
    let sizes = Sizes::of(cfg.scale);

    let builder = load_builder(cfg, &sizes)?;
    let admission = builder.admit().map_err(|e| format!("admit: {e:?}"))?;
    let heap = match admission {
        bumbledb::Admission::Accepted(instance) => instance,
        bumbledb::Admission::Rejected(v) => return Err(format!("rejected: {v}")),
    };

    let publish_dir = scratch.join("from-instance");
    let publish_start = Instant::now();
    let db = Db::from_instance_nosync(&publish_dir, &heap)
        .map_err(|e| format!("from_instance: {e:?}"))?;
    let publish_ns = u64::try_from(publish_start.elapsed().as_nanos()).expect("fits");

    let key = AccountId(0);
    let heap_get = harness::measure(proto, || {
        heap.get(key)
            .map(|got| u64::from(got.is_some()))
            .map_err(|e| format!("heap get: {e:?}"))
    })?;
    let lmdb_get = harness::measure(proto, || {
        db.read(|snap| snap.get(key).map(|got| u64::from(got.is_some())))
            .map_err(|e| format!("lmdb get: {e:?}"))
    })?;

    let fact: Account = heap
        .get(key)
        .map_err(|e| format!("seed get: {e:?}"))?
        .ok_or_else(|| "account 0 missing".to_owned())?;
    let heap_contains = harness::measure(proto, || {
        heap.contains(&fact)
            .map(u64::from)
            .map_err(|e| format!("heap contains: {e:?}"))
    })?;
    let lmdb_contains = harness::measure(proto, || {
        db.read(|snap| snap.contains(&fact).map(u64::from))
            .map_err(|e| format!("lmdb contains: {e:?}"))
    })?;

    let expected_accounts = sizes.accounts;
    let heap_scan = harness::measure(proto, || {
        let n = heap
            .scan(ids::ACCOUNT)
            .map_err(|e| format!("heap scan: {e:?}"))?
            .try_fold(0u64, |n, row| {
                row.map(|_| n + 1).map_err(|e| format!("{e:?}"))
            })?;
        if n != expected_accounts {
            return Err(format!("heap scan {n} != {expected_accounts}"));
        }
        Ok(n)
    })?;
    let lmdb_scan = harness::measure(proto, || {
        let n = db
            .read(|snap| {
                snap.scan(ids::ACCOUNT)?
                    .try_fold(0u64, |acc, row| row.map(|_| acc + 1))
            })
            .map_err(|e| format!("lmdb scan: {e:?}"))?;
        if n != expected_accounts {
            return Err(format!("lmdb scan {n} != {expected_accounts}"));
        }
        Ok(n)
    })?;

    let mut prepared = heap
        .prepare(&join_query())
        .map_err(|e| format!("prepare: {e:?}"))?;
    let join_start = Instant::now();
    let mut out = Answers::new();
    heap.execute(&mut prepared, &[], &mut out)
        .map_err(|e| format!("join: {e:?}"))?;
    let join_ns = u64::try_from(join_start.elapsed().as_nanos()).expect("fits");
    let join_rows = u64::try_from(out.len()).expect("fits");
    if join_rows != expected_accounts {
        return Err(format!("join rows {join_rows} != {expected_accounts}"));
    }

    let mut admission_rows = Vec::new();
    for &postings in &args.prefixes {
        let prefix = sizes_for_postings(postings);
        let facts = fact_count(&prefix);
        let builder = load_builder(cfg, &prefix)?;
        let start = Instant::now();
        let adm = builder
            .admit()
            .map_err(|e| format!("admit prefix {postings}: {e:?}"))?;
        let wall_ns = u64::try_from(start.elapsed().as_nanos()).expect("fits");
        match adm {
            bumbledb::Admission::Accepted(_) => {}
            bumbledb::Admission::Rejected(v) => {
                return Err(format!("prefix {postings} rejected: {v}"));
            }
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "reporting accepts lossy integer-to-float conversion"
        )]
        let facts_per_sec = if wall_ns == 0 {
            0.0
        } else {
            facts as f64 * 1e9 / wall_ns as f64
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "reporting accepts lossy integer-to-float conversion"
        )]
        let ns_per_fact = if facts == 0 {
            0.0
        } else {
            wall_ns as f64 / facts as f64
        };
        admission_rows.push(AdmitRow {
            facts,
            wall_ns,
            facts_per_sec,
            ns_per_fact,
        });
    }

    let report = HeapReport {
        provenance: crate::report::provenance(Path::new(".")),
        scale: args.scale.label(),
        seed: args.seed,
        samples: proto.samples,
        point_reads: vec![
            PointRow {
                name: "get".to_owned(),
                heap: heap_get.stats,
                lmdb: lmdb_get.stats,
            },
            PointRow {
                name: "contains".to_owned(),
                heap: heap_contains.stats,
                lmdb: lmdb_contains.stats,
            },
            PointRow {
                name: "scan".to_owned(),
                heap: heap_scan.stats,
                lmdb: lmdb_scan.stats,
            },
        ],
        admission: admission_rows,
        publish_ns,
        join_ns,
        join_rows,
    };
    std::fs::write(out_dir.join("heap-report.json"), to_json(&report))
        .map_err(|e| format!("artifact: {e}"))?;
    let markdown = to_markdown(&report);
    std::fs::write(out_dir.join("heap-report.md"), &markdown)
        .map_err(|e| format!("artifact: {e}"))?;
    print!("{markdown}");
    println!("artifacts: {}", out_dir.display());
    let _ = std::fs::remove_dir_all(scratch);
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus_gen::Scale;
    use crate::json::Value;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bumbledb-heap-lane-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn tiny_ladder_emits_the_three_lanes() {
        let dir = scratch("tiny");
        let out = dir.join("out");
        let code = run(&HeapArgs {
            scale: Scale::Tiny,
            seed: 1,
            dir: dir.clone(),
            samples: Some(4),
            prefixes: vec![64, 256],
            out: Some(out.clone()),
        })
        .expect("tiny heap ladder runs");
        assert_eq!(code, 0);
        let raw = std::fs::read_to_string(out.join("heap-report.json")).expect("artifact");
        let parsed = crate::json::parse(&raw).expect("valid JSON");
        let names: Vec<&str> = parsed
            .get("point_reads")
            .and_then(Value::as_arr)
            .expect("point_reads")
            .iter()
            .filter_map(|row| row.get("name").and_then(Value::as_str))
            .collect();
        assert_eq!(names, ["get", "contains", "scan"]);
        let admits = parsed
            .get("admission")
            .and_then(Value::as_arr)
            .expect("admission");
        assert_eq!(admits.len(), 2);
        for row in admits {
            assert!(row.get("facts").is_some(), "facts");
            assert!(row.get("wall_ns").is_some(), "wall_ns");
        }
        assert!(parsed.get("primer").is_none());
        assert!(out.join("heap-report.md").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
