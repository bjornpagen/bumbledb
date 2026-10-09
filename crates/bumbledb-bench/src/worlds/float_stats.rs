//! The `float_stats` world: one F64-heavy relation (1% NaN in `v`) and the
//! read families that exercise exact float folds, float comparisons and
//! computed float heads. Every family is checked against the naive evaluator
//! before it is timed; the engine is timed alone because SQLite has no exact
//! float reduction to compare against.
use std::path::Path;

use bumbledb::schema::{
    FieldId, RelationDescriptor, SchemaDescriptor, StatementDescriptor, ValueType,
};
use bumbledb::{
    Atom, AtomSource, CmpOp, Comparison, ConditionTree, Db, F64, FindTerm, FoldOp, ParamId, Query,
    RelationId, Rule, ScalarExpr, Term, Value, VarId,
};

use crate::harness::{self, Protocol, Stats};
use crate::oracle::differential::{Answers, engine_query};
use crate::oracle::naive::query::QueryError;
use crate::oracle::naive::{Delta, NaiveDb, ParamValue};
use crate::worlds::corpus_gen::Rng;

pub const READING: RelationId = RelationId(0);

/// `Reading(id, group, v, qty, price, a, b, c)`, keyed by `id`.
const FIELDS: [(&str, ValueType); 8] = [
    ("id", ValueType::U64),
    ("group", ValueType::U64),
    ("v", ValueType::F64),
    ("qty", ValueType::F64),
    ("price", ValueType::F64),
    ("a", ValueType::F64),
    ("b", ValueType::F64),
    ("c", ValueType::F64),
];

const ID: u16 = 0;
const GROUP: u16 = 1;
const V: u16 = 2;
const QTY: u16 = 3;
const PRICE: u16 = 4;
const A: u16 = 5;
const B: u16 = 6;
const C: u16 = 7;

/// Distinct `group` keys; the 1M-key grouping uses `id` instead.
pub const GROUPS: u64 = 10_000;

/// The constant of the `v > c` family.
pub const THRESHOLD: f64 = 0.5;

#[must_use]
pub fn descriptor() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Reading".into(),
            fields: FIELDS
                .iter()
                .map(|(name, value_type)| crate::fixture::field(name, *value_type))
                .collect(),
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: READING,
            projection: Box::new([FieldId(ID)]),
        }],
    }
}

fn float(rng: &mut Rng) -> f64 {
    let mantissa = rng.range(1 << 20) as f64 / f64::from(1u32 << 20);
    let scale = [1e-6, 1e-3, 1.0, 1e3, 1e9][usize::try_from(rng.range(5)).expect("small")];
    let sign = if rng.chance(1, 4) { -1.0 } else { 1.0 };
    sign * mantissa * scale
}

/// Row `i` of the world: `v` is NaN for one row in a hundred.
#[must_use]
pub fn row(seed: u64, i: u64) -> Vec<Value> {
    let mut rng = Rng::new(seed ^ i.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let v = if rng.chance(1, 100) {
        f64::NAN
    } else {
        float(&mut rng)
    };
    let mut next = || Value::F64(F64::from(float(&mut rng)));
    vec![
        Value::U64(i),
        Value::U64(i % GROUPS),
        Value::F64(F64::from(v)),
        next(),
        next(),
        next(),
        next(),
        next(),
    ]
}

fn var(id: u16) -> Term {
    Term::Var(VarId(id))
}

/// One atom binding every field `f` to variable `f`.
fn reading() -> Atom {
    Atom {
        source: AtomSource::Edb(READING),
        bindings: (0..u16::try_from(FIELDS.len()).expect("eight fields"))
            .map(|field| (FieldId(field), var(field)))
            .collect(),
    }
}

fn single(finds: Vec<FindTerm>, conditions: Vec<ConditionTree>) -> Query {
    Query::single(Rule {
        finds,
        atoms: vec![reading()],
        negated: vec![],
        conditions,
    })
}

fn folds() -> Vec<FindTerm> {
    [FoldOp::Sum, FoldOp::Mean, FoldOp::Min, FoldOp::Max]
        .into_iter()
        .map(|op| FindTerm::Aggregate { op, over: VarId(V) })
        .collect()
}

fn column(id: u16) -> ScalarExpr {
    ScalarExpr::Var(VarId(id))
}

pub struct FloatFamily {
    pub name: &'static str,
    pub about: &'static str,
    pub query: fn() -> Query,
    pub params: fn() -> Vec<ParamValue>,
}

fn no_params() -> Vec<ParamValue> {
    Vec::new()
}

#[must_use]
pub fn families() -> &'static [FloatFamily] {
    &[
        FloatFamily {
            name: "float_stats_global",
            about: "SUM/AVG/MIN/MAX of v over every row",
            query: || single(folds(), vec![]),
            params: no_params,
        },
        FloatFamily {
            name: "float_stats_grouped_10k",
            about: "SUM/AVG/MIN/MAX of v per group, 10k groups",
            query: || {
                let mut finds = vec![FindTerm::Var(VarId(GROUP))];
                finds.extend(folds());
                single(finds, vec![])
            },
            params: no_params,
        },
        FloatFamily {
            name: "float_stats_grouped_by_row",
            about: "SUM/AVG/MIN/MAX of v per id, one group per row",
            query: || {
                let mut finds = vec![FindTerm::Var(VarId(ID))];
                finds.extend(folds());
                single(finds, vec![])
            },
            params: no_params,
        },
        FloatFamily {
            name: "float_filter_gt",
            about: "ids with v > c for a constant c",
            query: || {
                single(
                    vec![FindTerm::Var(VarId(ID))],
                    vec![ConditionTree::Leaf(Comparison {
                        op: CmpOp::Gt,
                        lhs: var(V),
                        rhs: Term::Param(ParamId(0)),
                    })],
                )
            },
            params: || vec![ParamValue::Scalar(Value::F64(F64::from(THRESHOLD)))],
        },
        FloatFamily {
            name: "float_computed_product",
            about: "qty * price per row",
            query: || {
                single(
                    vec![
                        FindTerm::Var(VarId(ID)),
                        FindTerm::Compute(ScalarExpr::Multiply(
                            Box::new(column(QTY)),
                            Box::new(column(PRICE)),
                        )),
                    ],
                    vec![],
                )
            },
            params: no_params,
        },
        FloatFamily {
            name: "float_computed_quotient",
            about: "(a - b) / c per row",
            query: || {
                single(
                    vec![
                        FindTerm::Var(VarId(ID)),
                        FindTerm::Compute(ScalarExpr::Divide(
                            Box::new(ScalarExpr::Subtract(
                                Box::new(column(A)),
                                Box::new(column(B)),
                            )),
                            Box::new(column(C)),
                        )),
                    ],
                    vec![],
                )
            },
            params: no_params,
        },
    ]
}

/// The engine store and the naive model holding the same `rows` rows.
pub struct Stores {
    pub db: Db<SchemaDescriptor>,
    pub naive: NaiveDb,
}

/// Rows per load commit.
const CHUNK: u64 = 65_536;

pub fn load(dir: &Path, seed: u64, rows: u64) -> Result<Stores, String> {
    let descriptor = descriptor();
    let db = harness::create_db(&dir.join("db.bdb"), descriptor.clone())?;
    let mut start = 0;
    while start < rows {
        let end = (start + CHUNK).min(rows);
        harness::committed(
            "float_stats load",
            db.write(harness::bench_work(), |tx| {
                tx.insert_dyn(READING, (start..end).map(|i| row(seed, i)))
                    .map(bumbledb::MutationReport::changed)
            }),
        )?;
        start = end;
    }
    let mut naive = NaiveDb::new(&descriptor);
    naive
        .apply(&Delta {
            deletes: vec![],
            inserts: (0..rows).map(|i| (READING, row(seed, i))).collect(),
        })
        .map_err(|violations| format!("float_stats naive load: {violations:?}"))?;
    Ok(Stores { db, naive })
}

/// Engine and naive answers must be identical before the family is timed.
/// # Errors
/// On any disagreement, naming the family.
pub fn gate(stores: &Stores, family: &FloatFamily) -> Result<usize, String> {
    let query = (family.query)();
    let params = (family.params)();
    let model = match stores.naive.query(&query, &params) {
        Ok(rows) => Answers::Ok(rows),
        Err(QueryError::Overflow { .. }) => Answers::Overflow,
        Err(QueryError::Scalar { .. }) => Answers::Scalar,
    };
    let engine = engine_query(&stores.db, &query, &params);
    if engine != model {
        return Err(format!(
            "{}: engine and naive disagree — not timing a wrong answer",
            family.name
        ));
    }
    match engine {
        Answers::Ok(rows) => Ok(rows.len()),
        Answers::Overflow | Answers::Scalar => Err(format!(
            "{}: the family refuses at runtime on both engines",
            family.name
        )),
    }
}

#[derive(Debug, Clone)]
pub struct FloatStatsRow {
    pub family: &'static str,
    pub about: &'static str,
    pub answers: usize,
    pub ours: Stats,
}

/// Gates, then times, every family over a fresh `rows`-row world under `dir`.
pub fn run(
    dir: &Path,
    seed: u64,
    rows: u64,
    proto: Protocol,
) -> Result<Vec<FloatStatsRow>, String> {
    let stores = load(dir, seed, rows)?;
    let mut out = Vec::new();
    for family in families() {
        let answers = gate(&stores, family)?;
        let mut prepared = stores
            .db
            .prepare(&(family.query)(), harness::bench_work())
            .map_err(|e| format!("{}: prepare: {e:?}", family.name))?;
        let params = (family.params)();
        let args = crate::worlds::families::param_args(&params);
        let measured = harness::measure(proto, || {
            let buffer = stores
                .db
                .read(harness::bench_work(), |snap| {
                    snap.execute_collect(&mut prepared, &args)
                })
                .map_err(|e| format!("{}: execute: {e:?}", family.name))?;
            Ok(buffer.len() as u64)
        })?;
        out.push(FloatStatsRow {
            family: family.name,
            about: family.about,
            answers,
            ours: measured.stats,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_family_agrees_with_the_naive_model() {
        let dir = crate::fixture::TempDir::new("float-stats");
        let stores = load(dir.path(), 7, 2_000).expect("load");
        let nan_rows = (0..2_000)
            .filter(|i| matches!(&row(7, *i)[usize::from(V)], Value::F64(v) if v.to_f64().is_nan()))
            .count();
        assert!(nan_rows > 0, "the world carries NaN readings");
        for family in families() {
            let answers = gate(&stores, family).unwrap_or_else(|e| panic!("{e}"));
            assert!(answers > 0, "{} answers", family.name);
        }
    }
}
