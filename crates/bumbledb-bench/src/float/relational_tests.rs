//! Real SQLite / simple set evaluator / LMDB engine comparisons. Expected
//! scalar order is host numeric order plus the documented canonical NaN case,
//! not the engine's sortable-key implementation.
use std::collections::BTreeSet;

use bumbledb::{
    CmpOp, Comparison, ConditionTree, FindTerm, FoldOp, ParamId, Query, Rule, Term, Value, VarId,
};

use crate::differential::{Answers, engine_query};
use crate::fixture::{TempDir, atom, var};
use crate::naive::{NaiveDb, ParamValue, Tuple};
use crate::querygen::target::{self, ids};
use crate::translate::{LaneCase, ParamSlot, translate};

fn selection(op: CmpOp, rhs: Term) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(ids::FLOAT_VALUE, &[(1, var(0))])],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            lhs: var(0),
            op,
            rhs,
        })],
    })
}

fn sqlite_rows(
    conn: &rusqlite::Connection,
    query: &Query,
    params: &[ParamValue],
) -> BTreeSet<Tuple> {
    let sets: Vec<_> = params
        .iter()
        .enumerate()
        .filter_map(|(i, p)| match p {
            ParamValue::Set(values) => Some((ParamId(u16::try_from(i).unwrap()), values.clone())),
            ParamValue::Scalar(_) => None,
        })
        .collect();
    let sql = translate(query, target::schema(), &sets).unwrap();
    let bindings: Vec<_> = sql
        .params
        .iter()
        .map(|slot| {
            let ParamSlot::Whole(id) = slot else {
                panic!("scalar parameters only")
            };
            let ParamValue::Scalar(value) = &params[usize::from(id.0)] else {
                panic!("scalar slot")
            };
            crate::sqlmap::to_sql_row(std::slice::from_ref(value)).remove(0)
        })
        .collect();
    conn.prepare(&sql.sql)
        .unwrap()
        .query_map(rusqlite::params_from_iter(bindings), |row| {
            let bytes: Vec<u8> = row.get(0)?;
            Ok(Tuple(vec![Value::F64(
                super::from_sql_bytes(&bytes).unwrap(),
            )]))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn all_float_comparisons_literals_params_sets_and_folds_agree() {
    let dir = TempDir::new("float-relational-oracles");
    let db = target::publish_admitted(dir.path());
    let naive = NaiveDb::new(&target::descriptor());
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    for ddl in crate::sqlmap::schema_ddl(target::schema()) {
        conn.execute(&ddl, []).unwrap();
    }
    let relation = target::schema().relation(ids::FLOAT_VALUE);
    for (id, bits) in target::FLOAT_BITS.iter().enumerate() {
        conn.execute(
            &crate::sqlmap::insert_sql(relation),
            rusqlite::params_from_iter(crate::sqlmap::to_sql_row(&[
                Value::U64(id as u64),
                Value::F64(bumbledb::F64::from_bits(*bits)),
            ])),
        )
        .unwrap();
    }
    let check = |query: &Query, params: &[ParamValue]| {
        let expected = naive.query(query, params).unwrap();
        assert_eq!(
            engine_query(&db, query, params),
            Answers::Ok(expected.clone()),
            "{query:?}, {params:?}"
        );
        assert_eq!(
            sqlite_rows(&conn, query, params),
            expected,
            "{query:?}, {params:?}"
        );
    };
    for bits in target::FLOAT_BITS {
        let value = Value::F64(bumbledb::F64::from_bits(bits));
        for op in [
            CmpOp::Eq,
            CmpOp::Ne,
            CmpOp::Lt,
            CmpOp::Le,
            CmpOp::Gt,
            CmpOp::Ge,
        ] {
            check(&selection(op, Term::Literal(value.clone())), &[]);
            check(
                &selection(op, Term::Param(ParamId(0))),
                &[ParamValue::Scalar(value.clone())],
            );
        }
        check(
            &selection(CmpOp::Eq, Term::ParamSet(ParamId(0))),
            &[ParamValue::Set(vec![value.clone(), value])],
        );
    }
    check(
        &selection(CmpOp::Eq, Term::ParamSet(ParamId(0))),
        &[ParamValue::Set(vec![])],
    );
    for op in [FoldOp::Min, FoldOp::Max] {
        let mut query = selection(CmpOp::Eq, var(0));
        query.rules[0].conditions.clear();
        query.rules[0].finds = vec![FindTerm::Aggregate { op, over: VarId(0) }];
        let query = Query::single(query.rules.remove(0));
        check(&query, &[]);
    }
    // Two bounds enter range folding, unlike the single-comparison cases
    // above. The ordered word domain contains holes (negative zero and
    // noncanonical NaNs); an exclusive bound can land inside such a hole.
    // Compare against both independent evaluators instead of interpreting the
    // synthetic boundary as if it were a canonical float value.
    for (lower, upper) in [
        (0x8000_0000_0000_0001, 0x0000_0000_0000_0001),
        (0x0000_0000_0000_0000, 0x0000_0000_0000_0000),
        (0x7ff0_0000_0000_0000, 0x7ff8_0000_0000_0000),
        (0x7ff8_0000_0000_0000, 0x7ff8_0000_0000_0000),
        (0xfff0_0000_0000_0000, 0xffef_ffff_ffff_ffff),
        (0x7fef_ffff_ffff_ffff, 0x7ff0_0000_0000_0000),
    ] {
        for lower_op in [CmpOp::Gt, CmpOp::Ge] {
            for upper_op in [CmpOp::Lt, CmpOp::Le] {
                let mut query = selection(
                    lower_op,
                    Term::Literal(Value::F64(bumbledb::F64::from_bits(lower))),
                );
                query.rules[0]
                    .conditions
                    .push(ConditionTree::Leaf(Comparison {
                        lhs: var(0),
                        op: upper_op,
                        rhs: Term::Literal(Value::F64(bumbledb::F64::from_bits(upper))),
                    }));
                check(&query, &[]);
            }
        }
    }
    // A self-join creates multiple witnesses; projection must still return a set.
    let mut joined = selection(CmpOp::Le, var(1));
    joined.rules[0]
        .atoms
        .push(atom(ids::FLOAT_VALUE, &[(1, var(1))]));
    check(&joined, &[]);
}

#[test]
fn float_sum_cannot_accidentally_run_over_sqlite_blobs() {
    let mut query = selection(CmpOp::Eq, var(0));
    query.rules[0].conditions.clear();
    query.rules[0].finds = vec![FindTerm::Aggregate {
        op: FoldOp::Sum,
        over: VarId(0),
    }];
    let query = Query::single(query.rules.remove(0));
    assert!(
        translate(&query, target::schema(), &[])
            .unwrap_err()
            .contains("exact numerical oracle")
    );
    assert_eq!(
        crate::translate::sqlite_expressible(&LaneCase::Query(&query)),
        Err(crate::translate::Inexpressible::FloatArithmetic)
    );
}
