//! Normalization (docs/architecture/20-query-ir.md): lowers a [`ValidatedQuery`] into the paper-form
//! conjunctive query execution consumes — distinct-variable atom
//! occurrences, per-atom filters, and residual comparisons
//! (`docs/architecture/20-query-ir.md`, Deviation vs paper §2: the paper's
//! all-distinct-variables / pushed-selections assumption is a WLOG; we own
//! the lowering because there is no external optimizer).
//!
//! Infallible: the witness guarantees every input is lowerable.

use crate::encoding::{encode_bool, encode_i64};
use crate::image::view::{Const, FilterPredicate};
use crate::ir::validate::ValidatedQuery;
use crate::ir::{CmpOp, Term, Value, VarId};
use crate::schema::{FieldId, RelationId};
use crate::storage::dict::{TAG_BYTES, TAG_STRING};

/// Dense atom-occurrence id. Everything downstream (plan validity, trie
/// schemas) quantifies over occurrences, never relation names — self-joins
/// are ordinary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OccId(pub u16);

/// One atom occurrence in paper form: distinct variables only, plus the
/// filters lowered out of its bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub occ_id: OccId,
    pub relation: RelationId,
    /// Distinct variables with the field each is read from (a repeated
    /// variable keeps its first field; later positions became filters).
    pub vars: Vec<(FieldId, VarId)>,
    /// Per-occurrence filters, evaluated at the source (filtered view).
    pub filters: Vec<FilterPredicate>,
}

/// A comparison whose sides are variables — evaluated inside the join at
/// the earliest plan node where both are bound (placement is the 30-execution doc's job).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedComparison {
    pub op: CmpOp,
    pub lhs: VarId,
    pub rhs: VarId,
}

/// The paper-form query: occurrences + residuals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedQuery {
    pub occurrences: Vec<Occurrence>,
    pub residuals: Vec<PlacedComparison>,
}

/// Lowers a literal into column-form constant representation. String/Bytes
/// stay raw bytes (`PendingIntern`) — resolution to intern-id words happens
/// per execution, where a dictionary miss means an empty result.
fn lower_literal(value: &Value) -> Const {
    match value {
        Value::Bool(b) => Const::Byte(encode_bool(*b)),
        Value::Enum(ordinal) => Const::Byte(*ordinal),
        Value::U64(v) => Const::Word(*v),
        Value::I64(v) => Const::Word(u64::from_be_bytes(encode_i64(*v))),
        Value::String(bytes) => Const::PendingIntern {
            tag: TAG_STRING,
            bytes: bytes.clone(),
        },
        Value::Bytes(bytes) => Const::PendingIntern {
            tag: TAG_BYTES,
            bytes: bytes.clone(),
        },
    }
}

/// Mirrors an operator across the comparison when the constant was on the
/// left: `c < x` becomes `x > c`.
fn flip(op: CmpOp) -> CmpOp {
    match op {
        CmpOp::Eq => CmpOp::Eq,
        CmpOp::Ne => CmpOp::Ne,
        CmpOp::Lt => CmpOp::Gt,
        CmpOp::Le => CmpOp::Ge,
        CmpOp::Gt => CmpOp::Lt,
        CmpOp::Ge => CmpOp::Le,
    }
}

/// Places each comparison. Var-vs-constant pushes down as a filter on the
/// variable's first occurrence (sound for multi-occurrence variables —
/// join equality propagates the restriction); same-atom var-vs-var lowers
/// to a per-atom `FieldsCompare` filter; only cross-atom var-vs-var pairs
/// become residuals (docs/architecture/20-query-ir.md).
fn place_comparisons(
    query: &ValidatedQuery,
    occurrences: &mut [Occurrence],
) -> Vec<PlacedComparison> {
    let mut residuals = Vec::new();
    for comparison in &query.query().predicates {
        match (&comparison.lhs, &comparison.rhs) {
            (Term::Var(lhs), Term::Var(rhs)) => {
                let same_atom = occurrences.iter().find_map(|occ| {
                    let left = occ.vars.iter().find(|(_, v)| v == lhs);
                    let right = occ.vars.iter().find(|(_, v)| v == rhs);
                    match (left, right) {
                        (Some((lf, _)), Some((rf, _))) => Some((occ.occ_id, *lf, *rf)),
                        _ => None,
                    }
                });
                if let Some((occ_id, left, right)) = same_atom {
                    let occ = occurrences
                        .iter_mut()
                        .find(|o| o.occ_id == occ_id)
                        .expect("just found");
                    occ.filters.push(FilterPredicate::FieldsCompare {
                        left,
                        right,
                        op: comparison.op,
                    });
                } else {
                    residuals.push(PlacedComparison {
                        op: comparison.op,
                        lhs: *lhs,
                        rhs: *rhs,
                    });
                }
            }
            (Term::Var(var), constant) | (constant, Term::Var(var)) => {
                let var_on_left = matches!(&comparison.lhs, Term::Var(v) if v == var);
                let op = if var_on_left {
                    comparison.op
                } else {
                    flip(comparison.op)
                };
                let value = match constant {
                    Term::Param(param) => Const::Param(*param),
                    Term::Literal(literal) => lower_literal(literal),
                    Term::Var(_) => unreachable!("matched the var-var arm above"),
                };
                let (occurrence, field) = occurrences
                    .iter()
                    .enumerate()
                    .find_map(|(occ_idx, occ)| {
                        occ.vars
                            .iter()
                            .find(|(_, v)| v == var)
                            .map(|(field, _)| (occ_idx, *field))
                    })
                    .expect("validated: comparison variables are atom-bound");
                occurrences[occurrence]
                    .filters
                    .push(FilterPredicate::Compare { field, op, value });
            }
            _ => unreachable!("validated: constant comparisons are rejected"),
        }
    }
    residuals
}

/// Lowers the witness into paper form.
///
/// # Panics
///
/// Only on programmer-invariant violations already excluded by validation
/// (e.g. a comparison variable bound by no atom).
#[must_use]
pub fn normalize(query: &ValidatedQuery) -> NormalizedQuery {
    let mut occurrences: Vec<Occurrence> = query
        .query()
        .atoms
        .iter()
        .enumerate()
        .map(|(idx, atom)| {
            let occ_id = OccId(u16::try_from(idx).expect("validated: atom count fits u16"));
            let mut vars: Vec<(FieldId, VarId)> = Vec::new();
            let mut filters = Vec::new();
            for (field, term) in &atom.bindings {
                match term {
                    Term::Var(var) => {
                        // A repeated variable keeps its first field binding
                        // as the variable position; subsequent positions
                        // lower to same-fact equality filters.
                        if let Some((first_field, _)) = vars.iter().find(|(_, v)| v == var) {
                            filters.push(FilterPredicate::FieldsCompare {
                                left: *first_field,
                                right: *field,
                                op: CmpOp::Eq,
                            });
                        } else {
                            vars.push((*field, *var));
                        }
                    }
                    Term::Param(param) => filters.push(FilterPredicate::Compare {
                        field: *field,
                        op: CmpOp::Eq,
                        value: Const::Param(*param),
                    }),
                    Term::Literal(value) => filters.push(FilterPredicate::Compare {
                        field: *field,
                        op: CmpOp::Eq,
                        value: lower_literal(value),
                    }),
                }
            }
            Occurrence {
                occ_id,
                relation: atom.relation,
                vars,
                filters,
            }
        })
        .collect();

    let residuals = place_comparisons(query, &mut occurrences);

    NormalizedQuery {
        occurrences,
        residuals,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{encode_fact, ValueRef};
    use crate::ir::validate::validate;
    use crate::ir::{Atom, Comparison, FindTerm, ParamId, Query};
    use crate::schema::{
        FieldDescriptor, Generation, RelationDescriptor, Schema, SchemaDescriptor, ValueType,
    };
    use crate::storage::commit::commit;
    use crate::storage::delta::WriteDelta;
    use crate::storage::env::Environment;
    use crate::testutil::TempDir;

    /// R(id u64 serial, a i64, b i64) + S(x u64, y i64).
    fn schema() -> Schema {
        let field = |name: &str, ty: ValueType| FieldDescriptor {
            name: name.into(),
            value_type: ty,
            generation: Generation::None,
        };
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    name: "R".into(),
                    fields: vec![
                        FieldDescriptor {
                            name: "id".into(),
                            value_type: ValueType::U64,
                            generation: Generation::Serial,
                        },
                        field("a", ValueType::I64),
                        field("b", ValueType::I64),
                    ],
                    constraints: vec![],
                },
                RelationDescriptor {
                    name: "S".into(),
                    fields: vec![field("x", ValueType::U64), field("y", ValueType::I64)],
                    constraints: vec![],
                },
            ],
        }
        .validate()
        .expect("valid fixture")
    }

    const R: RelationId = RelationId(0);
    const S: RelationId = RelationId(1);

    fn var(id: u16) -> Term {
        Term::Var(VarId(id))
    }

    fn normalized(query: &Query) -> NormalizedQuery {
        normalize(&validate(&schema(), query).expect("valid"))
    }

    #[test]
    fn repeated_variable_lowers_and_executes_through_the_evaluator() {
        // R(a = v, b = v): one var position, one same-fact equality filter.
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                relation: R,
                bindings: vec![(FieldId(1), var(0)), (FieldId(2), var(0))],
            }],
            predicates: vec![],
        };
        let norm = normalized(&query);
        assert_eq!(norm.occurrences[0].vars, vec![(FieldId(1), VarId(0))]);
        assert_eq!(
            norm.occurrences[0].filters,
            vec![FilterPredicate::FieldsCompare {
                left: FieldId(1),
                right: FieldId(2),
                op: CmpOp::Eq,
            }]
        );

        // ...and the lowered filter executes on a real image.
        let dir = TempDir::new("normalize-execute");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        for (id, a, b) in [(1u64, 5i64, 5i64), (2, 5, 6), (3, -1, -1)] {
            let mut bytes = Vec::new();
            encode_fact(
                &[ValueRef::U64(id), ValueRef::I64(a), ValueRef::I64(b)],
                schema.relation(R).layout(),
                &mut bytes,
            );
            delta.insert(&view, R, &bytes).expect("insert");
        }
        drop(view);
        commit(delta, &env).expect("commit");
        let txn = env.read_txn().expect("txn");
        let image = crate::image::build(&txn, &schema, R).expect("build");
        let filtered =
            crate::image::view::apply(&image, &norm.occurrences[0].filters, &[], Vec::new());
        // Exactly the a == b rows survive.
        let ids: Vec<u64> = filtered
            .positions()
            .map(|p| filtered.image().column_words(0)[p as usize])
            .collect();
        assert_eq!(ids.len(), 2);
        assert!(!ids.contains(&2));
    }

    #[test]
    fn literal_and_param_bindings_lower_to_eq_filters() {
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                relation: R,
                bindings: vec![
                    (FieldId(0), var(0)),
                    (FieldId(1), Term::Literal(Value::I64(-7))),
                    (FieldId(2), Term::Param(ParamId(0))),
                ],
            }],
            predicates: vec![],
        };
        let norm = normalized(&query);
        assert_eq!(
            norm.occurrences[0].filters,
            vec![
                FilterPredicate::Compare {
                    field: FieldId(1),
                    op: CmpOp::Eq,
                    value: Const::Word(u64::from_be_bytes(encode_i64(-7))),
                },
                FilterPredicate::Compare {
                    field: FieldId(2),
                    op: CmpOp::Eq,
                    value: Const::Param(ParamId(0)),
                },
            ]
        );
    }

    #[test]
    fn string_literals_stay_raw_as_pending_interns() {
        // Add a string field via S? S has none — reuse R.id as U64 and use
        // a Bytes literal on a bytes field... the fixture lacks one, so
        // check lower_literal directly (the unit under test).
        assert_eq!(
            lower_literal(&Value::String(Box::from(&b"acme"[..]))),
            Const::PendingIntern {
                tag: TAG_STRING,
                bytes: Box::from(&b"acme"[..]),
            }
        );
        assert_eq!(
            lower_literal(&Value::Bytes(Box::from(&[7u8][..]))),
            Const::PendingIntern {
                tag: TAG_BYTES,
                bytes: Box::from(&[7u8][..]),
            }
        );
    }

    #[test]
    fn same_relation_atoms_get_distinct_occurrences_with_independent_filters() {
        // A self-join: R(id=v0, a=1) x R(id=v1, a=2).
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![
                Atom {
                    relation: R,
                    bindings: vec![
                        (FieldId(0), var(0)),
                        (FieldId(1), Term::Literal(Value::I64(1))),
                    ],
                },
                Atom {
                    relation: R,
                    bindings: vec![
                        (FieldId(0), var(1)),
                        (FieldId(1), Term::Literal(Value::I64(2))),
                    ],
                },
            ],
            predicates: vec![],
        };
        let norm = normalized(&query);
        assert_eq!(norm.occurrences.len(), 2);
        assert_eq!(norm.occurrences[0].occ_id, OccId(0));
        assert_eq!(norm.occurrences[1].occ_id, OccId(1));
        assert_eq!(norm.occurrences[0].relation, R);
        assert_eq!(norm.occurrences[1].relation, R);
        assert_ne!(norm.occurrences[0].filters, norm.occurrences[1].filters);
    }

    #[test]
    fn range_comparison_pushes_down_and_cross_atom_comparison_is_residual() {
        // 100 <= R.a (constant on the left: flips to a >= 100); R.a < S.y
        // stays a residual.
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![
                Atom {
                    relation: R,
                    bindings: vec![(FieldId(0), var(2)), (FieldId(1), var(0))],
                },
                Atom {
                    relation: S,
                    bindings: vec![(FieldId(1), var(1))],
                },
            ],
            predicates: vec![
                Comparison {
                    op: CmpOp::Le,
                    lhs: Term::Literal(Value::I64(100)),
                    rhs: var(0),
                },
                Comparison {
                    op: CmpOp::Lt,
                    lhs: var(0),
                    rhs: var(1),
                },
            ],
        };
        let norm = normalized(&query);
        assert_eq!(
            norm.occurrences[0].filters,
            vec![FilterPredicate::Compare {
                field: FieldId(1),
                op: CmpOp::Ge, // flipped
                value: Const::Word(u64::from_be_bytes(encode_i64(100))),
            }]
        );
        assert!(norm.occurrences[1].filters.is_empty());
        assert_eq!(
            norm.residuals,
            vec![PlacedComparison {
                op: CmpOp::Lt,
                lhs: VarId(0),
                rhs: VarId(1),
            }]
        );
    }

    #[test]
    fn occurrence_vars_are_duplicate_free_over_generated_inputs() {
        // A tiny deterministic generator: every subset/multiset of var
        // bindings over R's three fields, with var ids drawn from {0,1}.
        let mut checked = 0;
        for mask in 0..3u16.pow(3) {
            let mut bindings = Vec::new();
            let mut m = mask;
            for field in 0..3u16 {
                let choice = m % 3;
                m /= 3;
                match choice {
                    0 => {}
                    1 => bindings.push((FieldId(field), var(0))),
                    _ => bindings.push((FieldId(field), var(1))),
                }
            }
            if bindings.is_empty() {
                continue;
            }
            // Var 0 must be findable; ensure it is bound.
            if !bindings.iter().any(|(_, t)| *t == var(0)) {
                continue;
            }
            let query = Query {
                finds: vec![FindTerm::Var(VarId(0))],
                atoms: vec![Atom {
                    relation: R,
                    bindings,
                }],
                predicates: vec![],
            };
            // Field types differ (U64 vs I64): only same-typed repeats
            // validate; skip type-conflicting combinations.
            let Ok(witness) = validate(&schema(), &query) else {
                continue;
            };
            let norm = normalize(&witness);
            for occurrence in &norm.occurrences {
                let mut seen = std::collections::BTreeSet::new();
                for (_, v) in &occurrence.vars {
                    assert!(seen.insert(*v), "occurrence vars must be distinct");
                }
            }
            checked += 1;
        }
        assert!(checked > 3, "the sweep exercised real shapes: {checked}");
    }

    #[test]
    fn zero_binding_atom_becomes_an_empty_occurrence() {
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![
                Atom {
                    relation: R,
                    bindings: vec![(FieldId(0), var(0))],
                },
                Atom {
                    relation: S,
                    bindings: vec![],
                },
            ],
            predicates: vec![],
        };
        let norm = normalized(&query);
        assert_eq!(norm.occurrences[1].occ_id, OccId(1));
        assert!(norm.occurrences[1].vars.is_empty());
        assert!(norm.occurrences[1].filters.is_empty());
    }
}
