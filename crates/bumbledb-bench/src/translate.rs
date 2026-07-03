//! The IR→SQL translator (docs/benchmarks/09-sql-translator.md;
//! `docs/architecture/50-validation.md` names it infrastructure): total,
//! mechanical `Query` → `SQLite` SQL, faithful to set semantics. Where the
//! translator and the engine disagree, the hand-written goldens
//! ([`goldens`]) decide who is wrong — the 3-way arbitration anchor.
//!
//! Semantics mapping:
//! - Projection = `SELECT DISTINCT` over the find variables.
//! - Aggregation = the normative template: fold over a `SELECT DISTINCT`
//!   of **all bound variables** (the distinct full binding set), grouped
//!   by the non-aggregated finds; a *global* aggregate appends
//!   `HAVING COUNT(*) > 0` so SQL's one-NULL-row-over-empty collapses to
//!   the engine's empty set.
//! - A zero-binding atom (nonemptiness gate) becomes `EXISTS (SELECT 1
//!   FROM t)`.
//! - Never-interned strings/bytes need no special case: SQL compares
//!   values, which is exactly the sentinel semantics.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use bumbledb::ir::{Atom, CmpOp, Comparison, FindTerm, Term};
use bumbledb::{AggOp, ParamId, Query, Schema, Value, VarId};

/// A translated query: positional SQL plus the `ParamId` bound to each
/// `?N` (index `i` maps to placeholder `i + 1`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Translated {
    pub sql: String,
    pub params: Vec<ParamId>,
}

fn sql_string_literal(raw: &[u8]) -> Result<String, String> {
    let text = std::str::from_utf8(raw).map_err(|_| "non-UTF-8 string literal".to_owned())?;
    Ok(format!("'{}'", text.replace('\'', "''")))
}

fn sql_literal(value: &Value) -> Result<String, String> {
    Ok(match value {
        Value::Bool(v) => u8::from(*v).to_string(),
        Value::Enum(ordinal) => ordinal.to_string(),
        Value::U64(v) => {
            if *v >= 1 << 63 {
                return Err(format!("u64 literal {v} breaks the SQLite mapping axiom"));
            }
            v.to_string()
        }
        Value::I64(v) => v.to_string(),
        Value::String(raw) => sql_string_literal(raw)?,
        Value::Bytes(raw) => {
            let mut hex = String::with_capacity(raw.len() * 2 + 3);
            hex.push_str("X'");
            for b in raw {
                let _ = write!(hex, "{b:02X}");
            }
            hex.push('\'');
            hex
        }
    })
}

fn op_sql(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => "=",
        CmpOp::Ne => "<>",
        CmpOp::Lt => "<",
        CmpOp::Le => "<=",
        CmpOp::Gt => ">",
        CmpOp::Ge => ">=",
    }
}

struct Builder<'q> {
    schema: &'q Schema,
    /// FROM entries: `"Table" AS tN`.
    from: Vec<String>,
    /// WHERE conjuncts.
    predicates: Vec<String>,
    /// Var → its first binding's column reference (`tN."col"`).
    columns: BTreeMap<VarId, String>,
    /// `ParamId` → positional index (params may repeat; one `?N` each).
    param_index: BTreeMap<ParamId, usize>,
    params: Vec<ParamId>,
}

impl Builder<'_> {
    fn param_ref(&mut self, param: ParamId) -> String {
        let next = self.params.len();
        let index = *self.param_index.entry(param).or_insert_with(|| {
            self.params.push(param);
            next
        });
        format!("?{}", index + 1)
    }

    fn atom(&mut self, atom: &Atom) -> Result<(), String> {
        let relation = self.schema.relation(atom.relation);
        if atom.bindings.is_empty() {
            // The nonemptiness gate.
            self.predicates
                .push(format!("EXISTS (SELECT 1 FROM \"{}\")", relation.name()));
            return Ok(());
        }
        let alias = format!("t{}", self.from.len());
        self.from
            .push(format!("\"{}\" AS {alias}", relation.name()));
        for (field, term) in &atom.bindings {
            let column = format!(
                "{alias}.\"{}\"",
                relation.fields()[usize::from(field.0)].name
            );
            match term {
                Term::Var(var) => {
                    if let Some(first) = self.columns.get(var) {
                        // A later binding (cross-atom or in-atom repeat)
                        // equates to the first.
                        self.predicates.push(format!("{first} = {column}"));
                    } else {
                        self.columns.insert(*var, column);
                    }
                }
                Term::Literal(value) => {
                    self.predicates
                        .push(format!("{column} = {}", sql_literal(value)?));
                }
                Term::Param(param) => {
                    let placeholder = self.param_ref(*param);
                    self.predicates.push(format!("{column} = {placeholder}"));
                }
            }
        }
        Ok(())
    }

    fn side(&mut self, term: &Term) -> Result<String, String> {
        match term {
            Term::Var(var) => self
                .columns
                .get(var)
                .cloned()
                .ok_or_else(|| format!("comparison variable {} unbound", var.0)),
            Term::Literal(value) => sql_literal(value),
            Term::Param(param) => Ok(self.param_ref(*param)),
        }
    }

    fn comparison(&mut self, comparison: &Comparison) -> Result<(), String> {
        let lhs = self.side(&comparison.lhs)?;
        let rhs = self.side(&comparison.rhs)?;
        self.predicates
            .push(format!("{lhs} {} {rhs}", op_sql(comparison.op)));
        Ok(())
    }
}

/// Translates one validated-shape query over the given schema.
///
/// # Errors
///
/// A message naming the untranslatable construct. Total over the ledger
/// grammar with one documented exception: a query whose every atom is a
/// gate (no bound columns exist to select from) — the query generator
/// never produces one.
pub fn translate(query: &Query, schema: &Schema) -> Result<Translated, String> {
    let mut b = Builder {
        schema,
        from: Vec::new(),
        predicates: Vec::new(),
        columns: BTreeMap::new(),
        param_index: BTreeMap::new(),
        params: Vec::new(),
    };
    for atom in &query.atoms {
        b.atom(atom)?;
    }
    for comparison in &query.predicates {
        b.comparison(comparison)?;
    }
    if b.from.is_empty() {
        return Err("no bound atoms: nothing to select from".to_owned());
    }

    let from = b.from.join(", ");
    let where_clause = if b.predicates.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", b.predicates.join(" AND "))
    };

    let has_aggregates = query
        .finds
        .iter()
        .any(|f| matches!(f, FindTerm::Aggregate { .. }));

    let sql = if has_aggregates {
        // The normative template: fold over the DISTINCT full binding set.
        let mut inner_cols: Vec<String> = Vec::new();
        for (var, column) in &b.columns {
            inner_cols.push(format!("{column} AS v{}", var.0));
        }
        let inner = format!(
            "SELECT DISTINCT {} FROM {from}{where_clause}",
            inner_cols.join(", ")
        );
        let group: Vec<String> = query
            .finds
            .iter()
            .filter_map(|f| match f {
                FindTerm::Var(var) => Some(format!("v{}", var.0)),
                FindTerm::Aggregate { .. } => None,
            })
            .collect();
        let outer_cols: Vec<String> = query
            .finds
            .iter()
            .map(|f| match f {
                FindTerm::Var(var) => format!("v{}", var.0),
                FindTerm::Aggregate { op, over } => {
                    let agg = match op {
                        AggOp::Sum => "SUM",
                        AggOp::Min => "MIN",
                        AggOp::Max => "MAX",
                        AggOp::Count => "COUNT",
                    };
                    match over {
                        Some(var) => format!("{agg}(v{})", var.0),
                        None => "COUNT(*)".to_owned(),
                    }
                }
            })
            .collect();
        let tail = if group.is_empty() {
            // Global aggregate: SQL yields one NULL row over empty input;
            // the engine yields the empty set. HAVING collapses them.
            " HAVING COUNT(*) > 0".to_owned()
        } else {
            format!(" GROUP BY {}", group.join(", "))
        };
        format!("SELECT {} FROM ({inner}){tail}", outer_cols.join(", "))
    } else {
        let cols: Vec<String> = query
            .finds
            .iter()
            .map(|f| match f {
                FindTerm::Var(var) => b
                    .columns
                    .get(var)
                    .cloned()
                    .ok_or_else(|| format!("find variable {} unbound", var.0)),
                FindTerm::Aggregate { .. } => unreachable!("no aggregates here"),
            })
            .collect::<Result<_, _>>()?;
        format!(
            "SELECT DISTINCT {} FROM {from}{where_clause}",
            cols.join(", ")
        )
    };

    Ok(Translated {
        sql,
        params: b.params,
    })
}

/// The hand-written golden SQL per read family — the 3-way arbitration
/// anchor (docs/benchmarks/09): when the engine and `SQLite` disagree,
/// compare the translator's output against these; golden ≠ translator ⇒
/// translator bug, golden == translator ⇒ a human reads the semantics
/// docs and rules which engine is wrong. Written BY HAND, never
/// regenerated from the translator.
pub mod goldens {
    /// point — `Q(amount, at) :- Posting(id = ?0, amount, at)`.
    pub const POINT: &str =
        "SELECT DISTINCT t0.\"amount\", t0.\"at\" FROM \"Posting\" AS t0 WHERE t0.\"id\" = ?1";

    /// `fk_walk` — `Q(name, amount) :- Posting(account = ?0, amount),
    /// Account(id = a, holder = h), Holder(id = h, name)` with the
    /// posting's account equated to the account's id.
    pub const FK_WALK: &str = "SELECT DISTINCT t2.\"name\", t0.\"amount\" FROM \"Posting\" AS t0, \"Account\" AS t1, \"Holder\" AS t2 WHERE t0.\"account\" = ?1 AND t1.\"id\" = ?1 AND t1.\"holder\" = t2.\"id\"";

    /// balance — `Q(a, Sum(amount)) :- Posting(account = a, amount),
    /// Account(id = a, holder = ?0)`.
    pub const BALANCE: &str = "SELECT v0, SUM(v1) FROM (SELECT DISTINCT t0.\"account\" AS v0, t0.\"amount\" AS v1 FROM \"Posting\" AS t0, \"Account\" AS t1 WHERE t0.\"account\" = t1.\"id\" AND t1.\"holder\" = ?1) GROUP BY v0";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ids, schema};
    use bumbledb::ir::Term;

    fn var(id: u16) -> Term {
        Term::Var(VarId(id))
    }

    #[test]
    fn point_matches_its_hand_written_golden() {
        // Q(amount, at) :- Posting(id = ?0, amount, at).
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::ID, Term::Param(ParamId(0))),
                    (ids::posting::AMOUNT, var(0)),
                    (ids::posting::AT, var(1)),
                ],
            }],
            predicates: vec![],
        };
        let t = translate(&query, schema()).expect("translates");
        assert_eq!(t.sql, goldens::POINT);
        assert_eq!(t.params, vec![ParamId(0)]);
    }

    #[test]
    fn fk_walk_matches_its_hand_written_golden() {
        // Q(name, amount) :- Posting(account = ?0, amount),
        //                    Account(id = ?0, holder = h),
        //                    Holder(id = h, name).
        // (The account is pinned by the same param on both sides — the
        // join predicate through ?1 twice, param reused.)
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![
                Atom {
                    relation: ids::POSTING,
                    bindings: vec![
                        (ids::posting::ACCOUNT, Term::Param(ParamId(0))),
                        (ids::posting::AMOUNT, var(1)),
                    ],
                },
                Atom {
                    relation: ids::ACCOUNT,
                    bindings: vec![
                        (ids::account::ID, Term::Param(ParamId(0))),
                        (ids::account::HOLDER, var(2)),
                    ],
                },
                Atom {
                    relation: ids::HOLDER,
                    bindings: vec![(ids::holder::ID, var(2)), (ids::holder::NAME, var(0))],
                },
            ],
            predicates: vec![],
        };
        let t = translate(&query, schema()).expect("translates");
        assert_eq!(t.sql, goldens::FK_WALK);
        assert_eq!(t.params, vec![ParamId(0)], "one placeholder, reused");
    }

    #[test]
    fn balance_matches_its_hand_written_golden() {
        // Q(a, Sum(amount)) :- Posting(account = a, amount),
        //                      Account(id = a, holder = ?0).
        let query = Query {
            finds: vec![
                FindTerm::Var(VarId(0)),
                FindTerm::Aggregate {
                    op: AggOp::Sum,
                    over: Some(VarId(1)),
                },
            ],
            atoms: vec![
                Atom {
                    relation: ids::POSTING,
                    bindings: vec![
                        (ids::posting::ACCOUNT, var(0)),
                        (ids::posting::AMOUNT, var(1)),
                    ],
                },
                Atom {
                    relation: ids::ACCOUNT,
                    bindings: vec![
                        (ids::account::ID, var(0)),
                        (ids::account::HOLDER, Term::Param(ParamId(0))),
                    ],
                },
            ],
            predicates: vec![],
        };
        let t = translate(&query, schema()).expect("translates");
        assert_eq!(t.sql, goldens::BALANCE);
    }

    #[test]
    fn every_construct_translates() {
        // Gate atom → EXISTS; repeated in-atom var; same-atom and
        // cross-atom comparisons; every operator; literal escaping.
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![
                Atom {
                    relation: ids::POSTING,
                    bindings: vec![
                        (ids::posting::AMOUNT, var(0)),
                        (ids::posting::AT, var(1)),
                        (
                            ids::posting::MEMO,
                            Term::Literal(Value::String(b"it's a 'quote'".to_vec().into())),
                        ),
                    ],
                },
                Atom {
                    relation: ids::TAG,
                    bindings: vec![],
                },
            ],
            predicates: vec![
                Comparison {
                    op: CmpOp::Lt,
                    lhs: var(0),
                    rhs: var(1),
                },
                Comparison {
                    op: CmpOp::Ge,
                    lhs: var(1),
                    rhs: Term::Literal(Value::I64(-5)),
                },
                Comparison {
                    op: CmpOp::Ne,
                    lhs: var(0),
                    rhs: Term::Param(ParamId(0)),
                },
            ],
        };
        let t = translate(&query, schema()).expect("translates");
        assert!(
            t.sql.contains("EXISTS (SELECT 1 FROM \"Tag\")"),
            "{}",
            t.sql
        );
        assert!(t.sql.contains("'it''s a ''quote'''"), "{}", t.sql);
        assert!(t.sql.contains("t0.\"amount\" < t0.\"at\""), "{}", t.sql);
        assert!(t.sql.contains(">= -5"), "{}", t.sql);
        assert!(t.sql.contains("<> ?1"), "{}", t.sql);
        assert_eq!(t.params, vec![ParamId(0)]);

        // Repeated in-atom variable equates its two columns.
        let repeated = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                relation: ids::POSTING,
                bindings: vec![(ids::posting::AMOUNT, var(0)), (ids::posting::AT, var(0))],
            }],
            predicates: vec![],
        };
        let t = translate(&repeated, schema()).expect("translates");
        assert!(t.sql.contains("t0.\"amount\" = t0.\"at\""), "{}", t.sql);
    }

    #[test]
    fn global_aggregates_carry_the_having_rule() {
        // Q(Count) :- Posting(amount = x): SQL's NULL-row-over-empty must
        // collapse to the engine's empty set.
        let query = Query {
            finds: vec![FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            }],
            atoms: vec![Atom {
                relation: ids::POSTING,
                bindings: vec![(ids::posting::AMOUNT, var(0))],
            }],
            predicates: vec![],
        };
        let t = translate(&query, schema()).expect("translates");
        assert!(t.sql.ends_with("HAVING COUNT(*) > 0"), "{}", t.sql);
        assert!(t.sql.contains("SELECT DISTINCT"), "{}", t.sql);

        // Min/Max over the distinct binding set, grouped.
        let grouped = Query {
            finds: vec![
                FindTerm::Var(VarId(0)),
                FindTerm::Aggregate {
                    op: AggOp::Min,
                    over: Some(VarId(1)),
                },
                FindTerm::Aggregate {
                    op: AggOp::Max,
                    over: Some(VarId(1)),
                },
            ],
            atoms: vec![Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::ACCOUNT, var(0)),
                    (ids::posting::AMOUNT, var(1)),
                ],
            }],
            predicates: vec![],
        };
        let t = translate(&grouped, schema()).expect("translates");
        assert!(t.sql.contains("MIN(v1)"), "{}", t.sql);
        assert!(t.sql.contains("MAX(v1)"), "{}", t.sql);
        assert!(t.sql.ends_with("GROUP BY v0"), "{}", t.sql);
    }

    #[test]
    fn errors_name_the_untranslatable_construct() {
        let gates_only = Query {
            finds: vec![FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            }],
            atoms: vec![Atom {
                relation: ids::TAG,
                bindings: vec![],
            }],
            predicates: vec![],
        };
        let err = translate(&gates_only, schema()).unwrap_err();
        assert!(err.contains("no bound atoms"), "{err}");
    }
}
