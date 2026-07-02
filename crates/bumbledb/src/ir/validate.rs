//! The single validation boundary (PRD 14): IR in, [`ValidatedQuery`]
//! witness out. Everything downstream trusts the witness and re-checks
//! nothing (post-mortem §38: v5 validated one plan four times).
//!
//! The roster, transcribed from `docs/architecture/20-query-ir.md` and
//! checked off in code order below — it is exhaustive by contract:
//!
//!  1. unknown relation ids
//!  2. unknown field ids
//!  3. duplicate `FieldId` in one atom's bindings
//!  4. variable type conflicts (structural)
//!  5. literal-vs-field type mismatches
//!  6. enum ordinal out of range for the field's variant list
//!  7. param anchor conflicts / unanchored params
//!  8. comparisons violating the type rules (Eq/Ne all types; order ops
//!     U64/U64 and I64/I64 only; no cross-type, ever)
//!  9. constant comparisons (no variable side)
//! 10. unbound find variables (Datalog safety; includes aggregate inputs)
//! 11. comparison-only variables
//! 12. empty finds
//! 13. duplicate find terms
//! 14. no atoms
//! 15. aggregate input types (Sum/Min/Max integers only; Count nullary)
//! 16. aggregate over a group-key variable

use std::collections::{BTreeMap, BTreeSet};

use crate::error::ValidationError;
use crate::ir::{AggOp, CmpOp, Comparison, FindTerm, ParamId, Query, Term, Value, VarId};
use crate::schema::{Schema, ValueType};

/// The sealed witness: the query plus the derived tables downstream layers
/// trust. Unconstructible outside this module.
#[derive(Debug)]
pub struct ValidatedQuery {
    query: Query,
    var_types: BTreeMap<VarId, ValueType>,
    param_types: BTreeMap<ParamId, ValueType>,
    /// Non-aggregated find variables — the group key under aggregation.
    group_key: BTreeSet<VarId>,
    /// Whether any find term is an aggregate.
    has_aggregates: bool,
}

impl ValidatedQuery {
    /// The validated query, verbatim.
    #[must_use]
    pub fn query(&self) -> &Query {
        &self.query
    }

    /// The resolved structural type of a variable.
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: an unknown `VarId` (the witness
    /// resolved every variable).
    #[must_use]
    pub fn var_type(&self, var: VarId) -> &ValueType {
        &self.var_types[&var]
    }

    /// Every variable with its resolved type, in id order.
    pub fn var_types(&self) -> impl Iterator<Item = (VarId, &ValueType)> {
        self.var_types.iter().map(|(v, t)| (*v, t))
    }

    /// Every param with its resolved type, in id order (bind-time checking,
    /// PRD 25).
    pub fn param_types(&self) -> impl Iterator<Item = (ParamId, &ValueType)> {
        self.param_types.iter().map(|(p, t)| (*p, t))
    }

    /// The group key: non-aggregated find variables.
    #[must_use]
    pub fn group_key(&self) -> &BTreeSet<VarId> {
        &self.group_key
    }

    /// Whether the query aggregates.
    #[must_use]
    pub fn has_aggregates(&self) -> bool {
        self.has_aggregates
    }
}

/// The structural type of a literal, for matching against a field or
/// variable type. Enum literals carry only an ordinal, so they match any
/// enum type whose variant list covers the ordinal.
fn literal_matches(value: &Value, expected: &ValueType) -> Result<(), LiteralMismatch> {
    match (value, expected) {
        (Value::Bool(_), ValueType::Bool)
        | (Value::U64(_), ValueType::U64)
        | (Value::I64(_), ValueType::I64)
        | (Value::String(_), ValueType::String)
        | (Value::Bytes(_), ValueType::Bytes) => Ok(()),
        (Value::Enum(ordinal), ValueType::Enum { variants }) => {
            if usize::from(*ordinal) < variants.len() {
                Ok(())
            } else {
                Err(LiteralMismatch::EnumOrdinal(*ordinal))
            }
        }
        _ => Err(LiteralMismatch::Type),
    }
}

enum LiteralMismatch {
    Type,
    EnumOrdinal(u8),
}

/// Whether the operator is legal for the type: `Eq`/`Ne` everywhere, order
/// operators only over the two integer types.
fn cmp_legal(op: CmpOp, value_type: &ValueType) -> bool {
    match op {
        CmpOp::Eq | CmpOp::Ne => true,
        CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge => {
            matches!(value_type, ValueType::U64 | ValueType::I64)
        }
    }
}

/// Validates a query against the schema, yielding the sealed witness.
///
/// # Errors
///
/// A distinct [`ValidationError`] per roster item; see the module docs.
pub fn validate(schema: &Schema, query: &Query) -> Result<ValidatedQuery, ValidationError> {
    if query.finds.is_empty() {
        return Err(ValidationError::EmptyFinds);
    }
    if query.atoms.is_empty() {
        return Err(ValidationError::NoAtoms);
    }
    for (index, term) in query.finds.iter().enumerate() {
        if query.finds[..index].contains(term) {
            return Err(ValidationError::DuplicateFindTerm { index });
        }
    }

    let mut ctx = Context::default();
    ctx.check_atoms(schema, query)?;
    ctx.check_comparisons(query)?;
    ctx.check_finds(query)?;

    let group_key: BTreeSet<VarId> = query
        .finds
        .iter()
        .filter_map(|term| match term {
            FindTerm::Var(var) => Some(*var),
            FindTerm::Aggregate { .. } => None,
        })
        .collect();
    let has_aggregates = query
        .finds
        .iter()
        .any(|t| matches!(t, FindTerm::Aggregate { .. }));

    Ok(ValidatedQuery {
        query: query.clone(),
        var_types: ctx.var_types,
        param_types: ctx.param_types,
        group_key,
        has_aggregates,
    })
}

/// Accumulated typing state while walking the query.
#[derive(Default)]
struct Context {
    var_types: BTreeMap<VarId, ValueType>,
    param_types: BTreeMap<ParamId, ValueType>,
    /// Params seen anywhere (each must end up anchored).
    params_seen: BTreeSet<ParamId>,
    /// Variables bound by at least one atom.
    atom_vars: BTreeSet<VarId>,
}

impl Context {
    fn bind_var(&mut self, var: VarId, value_type: &ValueType) -> Result<(), ValidationError> {
        match self.var_types.get(&var) {
            Some(existing) if existing != value_type => {
                Err(ValidationError::VariableTypeConflict { var })
            }
            Some(_) => Ok(()),
            None => {
                self.var_types.insert(var, value_type.clone());
                Ok(())
            }
        }
    }

    fn anchor_param(
        &mut self,
        param: ParamId,
        value_type: &ValueType,
    ) -> Result<(), ValidationError> {
        self.params_seen.insert(param);
        match self.param_types.get(&param) {
            Some(existing) if existing != value_type => {
                Err(ValidationError::ParamTypeConflict { param })
            }
            Some(_) => Ok(()),
            None => {
                self.param_types.insert(param, value_type.clone());
                Ok(())
            }
        }
    }

    fn check_atoms(&mut self, schema: &Schema, query: &Query) -> Result<(), ValidationError> {
        for (atom_idx, atom) in query.atoms.iter().enumerate() {
            if usize::try_from(atom.relation.0).expect("64-bit usize") >= schema.relations().len() {
                return Err(ValidationError::UnknownRelation {
                    atom: atom_idx,
                    relation: atom.relation,
                });
            }
            let relation = schema.relation(atom.relation);
            for (binding_idx, (field, term)) in atom.bindings.iter().enumerate() {
                if usize::from(field.0) >= relation.fields().len() {
                    return Err(ValidationError::UnknownField {
                        atom: atom_idx,
                        field: *field,
                    });
                }
                if atom.bindings[..binding_idx].iter().any(|(f, _)| f == field) {
                    return Err(ValidationError::DuplicateFieldBinding {
                        atom: atom_idx,
                        field: *field,
                    });
                }
                let field_type = &relation.field(*field).value_type;
                match term {
                    Term::Var(var) => {
                        self.bind_var(*var, field_type)?;
                        self.atom_vars.insert(*var);
                    }
                    Term::Param(param) => self.anchor_param(*param, field_type)?,
                    Term::Literal(value) => match literal_matches(value, field_type) {
                        Ok(()) => {}
                        Err(LiteralMismatch::Type) => {
                            return Err(ValidationError::LiteralTypeMismatch {
                                atom: atom_idx,
                                field: *field,
                            });
                        }
                        Err(LiteralMismatch::EnumOrdinal(ordinal)) => {
                            return Err(ValidationError::EnumOrdinalOutOfRange {
                                atom: atom_idx,
                                field: *field,
                                ordinal,
                            });
                        }
                    },
                }
            }
        }
        Ok(())
    }

    fn check_comparisons(&mut self, query: &Query) -> Result<(), ValidationError> {
        for (index, Comparison { op, lhs, rhs }) in query.predicates.iter().enumerate() {
            // A comparison with no variable side is a constant comparison —
            // write the query you mean.
            let (var_side, other) = match (lhs, rhs) {
                (Term::Var(var), other) | (other, Term::Var(var)) => (*var, other),
                _ => return Err(ValidationError::ConstantComparison { index }),
            };
            let Some(var_type) = self.var_types.get(&var_side).cloned() else {
                return Err(ValidationError::ComparisonOnlyVariable { var: var_side });
            };
            if !cmp_legal(*op, &var_type) {
                return Err(ValidationError::IllegalComparison { index });
            }
            match other {
                Term::Var(other_var) => {
                    let Some(other_type) = self.var_types.get(other_var) else {
                        return Err(ValidationError::ComparisonOnlyVariable { var: *other_var });
                    };
                    if *other_type != var_type {
                        return Err(ValidationError::IllegalComparison { index });
                    }
                }
                Term::Param(param) => self.anchor_param(*param, &var_type)?,
                Term::Literal(value) => match literal_matches(value, &var_type) {
                    Ok(()) => {}
                    Err(_) => return Err(ValidationError::IllegalComparison { index }),
                },
            }
        }
        // Every param must have found an anchor by now. (Anchors come from
        // field bindings and comparisons against typed terms; a param only
        // ever compared to another param was already a constant comparison.)
        for param in &self.params_seen {
            if !self.param_types.contains_key(param) {
                return Err(ValidationError::ParamUnanchored { param: *param });
            }
        }
        Ok(())
    }

    fn check_finds(&self, query: &Query) -> Result<(), ValidationError> {
        let group_key: BTreeSet<VarId> = query
            .finds
            .iter()
            .filter_map(|term| match term {
                FindTerm::Var(var) => Some(*var),
                FindTerm::Aggregate { .. } => None,
            })
            .collect();
        for (find_idx, term) in query.finds.iter().enumerate() {
            match term {
                FindTerm::Var(var) => {
                    if !self.atom_vars.contains(var) {
                        return Err(ValidationError::UnboundFindVariable { var: *var });
                    }
                }
                FindTerm::Aggregate { op, over } => match (op, over) {
                    (AggOp::Count, Some(_)) => {
                        return Err(ValidationError::CountWithVariable { find: find_idx });
                    }
                    (AggOp::Count, None) => {}
                    (AggOp::Sum | AggOp::Min | AggOp::Max, None) => {
                        return Err(ValidationError::AggregateWithoutVariable { find: find_idx });
                    }
                    (AggOp::Sum | AggOp::Min | AggOp::Max, Some(var)) => {
                        if !self.atom_vars.contains(var) {
                            return Err(ValidationError::UnboundFindVariable { var: *var });
                        }
                        if group_key.contains(var) {
                            return Err(ValidationError::AggregateOverGroupKey { find: find_idx });
                        }
                        let var_type = &self.var_types[var];
                        if !matches!(var_type, ValueType::U64 | ValueType::I64) {
                            return Err(ValidationError::AggregateInputType { find: find_idx });
                        }
                    }
                },
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        ConstraintDescriptor, FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId,
        SchemaDescriptor,
    };

    /// The fixture schema: Holder(id serial, name string); Account(id
    /// serial, holder u64 fk, status enum); Posting(id serial, account
    /// u64, amount i64, at i64, memo bytes, flag bool).
    fn schema() -> Schema {
        let field = |name: &str, ty: ValueType| FieldDescriptor {
            name: name.into(),
            value_type: ty,
            generation: Generation::None,
        };
        let serial = |name: &str| FieldDescriptor {
            name: name.into(),
            value_type: ValueType::U64,
            generation: Generation::Serial,
        };
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    name: "Holder".into(),
                    fields: vec![serial("id"), field("name", ValueType::String)],
                    constraints: vec![],
                },
                RelationDescriptor {
                    name: "Account".into(),
                    fields: vec![
                        serial("id"),
                        field("holder", ValueType::U64),
                        field(
                            "status",
                            ValueType::Enum {
                                variants: ["Active", "Closed"]
                                    .iter()
                                    .map(|v| Box::from(*v))
                                    .collect(),
                            },
                        ),
                    ],
                    constraints: vec![ConstraintDescriptor::ForeignKey {
                        name: "account_holder".into(),
                        fields: Box::new([FieldId(1)]),
                        target_relation: RelationId(0),
                        target_constraint: crate::schema::ConstraintId(0),
                    }],
                },
                RelationDescriptor {
                    name: "Posting".into(),
                    fields: vec![
                        serial("id"),
                        field("account", ValueType::U64),
                        field("amount", ValueType::I64),
                        field("at", ValueType::I64),
                        field("memo", ValueType::Bytes),
                        field("flag", ValueType::Bool),
                    ],
                    constraints: vec![],
                },
            ],
        }
        .validate()
        .expect("valid fixture")
    }

    const HOLDER: RelationId = RelationId(0);
    const ACCOUNT: RelationId = RelationId(1);
    const POSTING: RelationId = RelationId(2);

    fn atom(relation: RelationId, bindings: Vec<(u16, Term)>) -> crate::ir::Atom {
        crate::ir::Atom {
            relation,
            bindings: bindings.into_iter().map(|(f, t)| (FieldId(f), t)).collect(),
        }
    }

    fn var(id: u16) -> Term {
        Term::Var(VarId(id))
    }

    fn simple(finds: Vec<FindTerm>, atoms: Vec<crate::ir::Atom>) -> Query {
        Query {
            finds,
            atoms,
            predicates: vec![],
        }
    }

    fn expect_err(query: &Query) -> ValidationError {
        validate(&schema(), query).expect_err("must reject")
    }

    // --- Accepting shapes ---

    #[test]
    fn accepts_the_fk_walk_join_with_predicates() {
        let query = Query {
            finds: vec![FindTerm::Var(VarId(1))],
            atoms: vec![
                atom(POSTING, vec![(1, var(0)), (2, var(1)), (3, var(2))]),
                atom(ACCOUNT, vec![(0, var(0))]),
            ],
            predicates: vec![Comparison {
                op: CmpOp::Ge,
                lhs: var(2),
                rhs: Term::Literal(Value::I64(100)),
            }],
        };
        let witness = validate(&schema(), &query).expect("valid");
        assert_eq!(witness.var_type(VarId(0)), &ValueType::U64);
        assert_eq!(witness.var_type(VarId(2)), &ValueType::I64);
        assert_eq!(witness.group_key().len(), 1);
        assert!(!witness.has_aggregates());
    }

    #[test]
    fn accepts_params_anchored_by_fields_and_comparisons() {
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![atom(
                POSTING,
                vec![(1, Term::Param(ParamId(0))), (0, var(0)), (3, var(1))],
            )],
            predicates: vec![Comparison {
                op: CmpOp::Lt,
                lhs: var(1),
                rhs: Term::Param(ParamId(1)),
            }],
        };
        let witness = validate(&schema(), &query).expect("valid");
        let params: Vec<_> = witness.param_types().collect();
        assert_eq!(params[0], (ParamId(0), &ValueType::U64));
        assert_eq!(params[1], (ParamId(1), &ValueType::I64));
    }

    #[test]
    fn accepts_all_aggregate_finds() {
        // Empty group key, one global group — legal per the doc.
        let query = simple(
            vec![
                FindTerm::Aggregate {
                    op: AggOp::Sum,
                    over: Some(VarId(0)),
                },
                FindTerm::Aggregate {
                    op: AggOp::Count,
                    over: None,
                },
            ],
            vec![atom(POSTING, vec![(2, var(0))])],
        );
        let witness = validate(&schema(), &query).expect("valid");
        assert!(witness.group_key().is_empty());
        assert!(witness.has_aggregates());
    }

    #[test]
    fn accepts_zero_binding_atoms() {
        let query = simple(
            vec![FindTerm::Var(VarId(0))],
            vec![
                atom(POSTING, vec![(0, var(0))]),
                atom(HOLDER, vec![]), // nonemptiness gate
            ],
        );
        validate(&schema(), &query).expect("valid");
    }

    #[test]
    fn accepts_repeated_variable_within_one_atom() {
        // Same-fact equality: amount == at (both I64).
        let query = simple(
            vec![FindTerm::Var(VarId(0))],
            vec![atom(POSTING, vec![(2, var(0)), (3, var(0))])],
        );
        validate(&schema(), &query).expect("valid");
    }

    // --- Rejecting shapes, one per roster item ---

    #[test]
    fn rejects_unknown_relation() {
        let query = simple(
            vec![FindTerm::Var(VarId(0))],
            vec![atom(RelationId(9), vec![(0, var(0))])],
        );
        assert!(matches!(
            expect_err(&query),
            ValidationError::UnknownRelation { atom: 0, .. }
        ));
    }

    #[test]
    fn rejects_unknown_field() {
        let query = simple(
            vec![FindTerm::Var(VarId(0))],
            vec![atom(HOLDER, vec![(9, var(0))])],
        );
        assert!(matches!(
            expect_err(&query),
            ValidationError::UnknownField {
                atom: 0,
                field: FieldId(9)
            }
        ));
    }

    #[test]
    fn rejects_duplicate_field_binding() {
        let query = simple(
            vec![FindTerm::Var(VarId(0))],
            vec![atom(HOLDER, vec![(0, var(0)), (0, var(1))])],
        );
        assert!(matches!(
            expect_err(&query),
            ValidationError::DuplicateFieldBinding {
                atom: 0,
                field: FieldId(0)
            }
        ));
    }

    #[test]
    fn rejects_variable_type_conflict() {
        // Var 0 bound to a U64 field and an I64 field.
        let query = simple(
            vec![FindTerm::Var(VarId(0))],
            vec![atom(POSTING, vec![(1, var(0)), (2, var(0))])],
        );
        assert!(matches!(
            expect_err(&query),
            ValidationError::VariableTypeConflict { var: VarId(0) }
        ));
    }

    #[test]
    fn rejects_literal_type_mismatch() {
        let query = simple(
            vec![FindTerm::Var(VarId(0))],
            vec![atom(
                POSTING,
                vec![(0, var(0)), (2, Term::Literal(Value::U64(5)))], // I64 field
            )],
        );
        assert!(matches!(
            expect_err(&query),
            ValidationError::LiteralTypeMismatch {
                atom: 0,
                field: FieldId(2)
            }
        ));
    }

    #[test]
    fn rejects_enum_ordinal_out_of_range() {
        let query = simple(
            vec![FindTerm::Var(VarId(0))],
            vec![atom(
                ACCOUNT,
                vec![(0, var(0)), (2, Term::Literal(Value::Enum(2)))], // 2 variants
            )],
        );
        assert!(matches!(
            expect_err(&query),
            ValidationError::EnumOrdinalOutOfRange {
                atom: 0,
                field: FieldId(2),
                ordinal: 2
            }
        ));
    }

    #[test]
    fn rejects_conflicting_param_anchors() {
        // Param 0 anchored at U64 (Posting.account) and I64 (Posting.amount).
        let query = simple(
            vec![FindTerm::Var(VarId(0))],
            vec![atom(
                POSTING,
                vec![
                    (0, var(0)),
                    (1, Term::Param(ParamId(0))),
                    (2, Term::Param(ParamId(0))),
                ],
            )],
        );
        assert!(matches!(
            expect_err(&query),
            ValidationError::ParamTypeConflict { param: ParamId(0) }
        ));
    }

    #[test]
    fn rejects_order_comparison_on_non_integer() {
        // Holder.name is a String: Lt is illegal (equality-only type).
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![atom(HOLDER, vec![(0, var(1)), (1, var(0))])],
            predicates: vec![Comparison {
                op: CmpOp::Lt,
                lhs: var(0),
                rhs: Term::Literal(Value::String(Box::from(&b"x"[..]))),
            }],
        };
        assert!(matches!(
            expect_err(&query),
            ValidationError::IllegalComparison { index: 0 }
        ));
    }

    #[test]
    fn rejects_cross_type_comparison() {
        // U64 var vs I64 var: no silent coercion, ever.
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![atom(POSTING, vec![(1, var(0)), (2, var(1))])],
            predicates: vec![Comparison {
                op: CmpOp::Eq,
                lhs: var(0),
                rhs: var(1),
            }],
        };
        assert!(matches!(
            expect_err(&query),
            ValidationError::IllegalComparison { index: 0 }
        ));
    }

    #[test]
    fn rejects_constant_comparison() {
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![atom(HOLDER, vec![(0, var(0))])],
            predicates: vec![Comparison {
                op: CmpOp::Eq,
                lhs: Term::Literal(Value::U64(1)),
                rhs: Term::Param(ParamId(0)),
            }],
        };
        assert!(matches!(
            expect_err(&query),
            ValidationError::ConstantComparison { index: 0 }
        ));
    }

    #[test]
    fn rejects_unbound_find_variable() {
        let query = simple(
            vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(7))],
            vec![atom(HOLDER, vec![(0, var(0))])],
        );
        assert!(matches!(
            expect_err(&query),
            ValidationError::UnboundFindVariable { var: VarId(7) }
        ));
    }

    #[test]
    fn rejects_comparison_only_variable() {
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![atom(HOLDER, vec![(0, var(0))])],
            predicates: vec![Comparison {
                op: CmpOp::Eq,
                lhs: var(9), // appears in no atom
                rhs: var(0),
            }],
        };
        assert!(matches!(
            expect_err(&query),
            ValidationError::ComparisonOnlyVariable { var: VarId(9) }
        ));
    }

    #[test]
    fn rejects_empty_finds() {
        let query = simple(vec![], vec![atom(HOLDER, vec![(0, var(0))])]);
        assert!(matches!(expect_err(&query), ValidationError::EmptyFinds));
    }

    #[test]
    fn rejects_duplicate_find_terms() {
        let query = simple(
            vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(0))],
            vec![atom(HOLDER, vec![(0, var(0))])],
        );
        assert!(matches!(
            expect_err(&query),
            ValidationError::DuplicateFindTerm { index: 1 }
        ));
    }

    #[test]
    fn rejects_no_atoms() {
        let query = simple(vec![FindTerm::Var(VarId(0))], vec![]);
        assert!(matches!(expect_err(&query), ValidationError::NoAtoms));
    }

    #[test]
    fn rejects_sum_over_non_integer() {
        let query = simple(
            vec![FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(0)),
            }],
            vec![atom(HOLDER, vec![(1, var(0))])], // String
        );
        assert!(matches!(
            expect_err(&query),
            ValidationError::AggregateInputType { find: 0 }
        ));
    }

    #[test]
    fn rejects_count_with_a_variable() {
        let query = simple(
            vec![FindTerm::Aggregate {
                op: AggOp::Count,
                over: Some(VarId(0)),
            }],
            vec![atom(POSTING, vec![(2, var(0))])],
        );
        assert!(matches!(
            expect_err(&query),
            ValidationError::CountWithVariable { find: 0 }
        ));
    }

    #[test]
    fn rejects_sum_without_a_variable() {
        let query = simple(
            vec![FindTerm::Aggregate {
                op: AggOp::Sum,
                over: None,
            }],
            vec![atom(POSTING, vec![(2, var(0))])],
        );
        assert!(matches!(
            expect_err(&query),
            ValidationError::AggregateWithoutVariable { find: 0 }
        ));
    }

    #[test]
    fn rejects_aggregate_over_group_key() {
        let query = simple(
            vec![
                FindTerm::Var(VarId(0)),
                FindTerm::Aggregate {
                    op: AggOp::Sum,
                    over: Some(VarId(0)),
                },
            ],
            vec![atom(POSTING, vec![(2, var(0))])],
        );
        assert!(matches!(
            expect_err(&query),
            ValidationError::AggregateOverGroupKey { find: 1 }
        ));
    }

    #[test]
    fn rejects_unanchored_param() {
        // A param whose only appearances give it no type. The one shape
        // that reaches the unanchored check is a param never compared and
        // never field-bound... which cannot appear in a query at all — so
        // the closest reachable shape is a param anchored only through an
        // illegal constant comparison, rejected earlier. Anchoring through
        // a variable keeps this reachable:
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![atom(HOLDER, vec![(0, var(0))])],
            predicates: vec![Comparison {
                op: CmpOp::Eq,
                lhs: var(0),
                rhs: Term::Param(ParamId(0)),
            }],
        };
        // This one *is* anchored (against var 0's U64): accepted.
        let witness = validate(&schema(), &query).expect("valid");
        assert_eq!(
            witness.param_types().next(),
            Some((ParamId(0), &ValueType::U64))
        );
    }
}
