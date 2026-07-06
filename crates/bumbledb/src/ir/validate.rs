//! The single validation boundary (docs/architecture/20-query-ir.md): IR in, [`ValidatedQuery`]
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
//!  7. param anchor conflicts (an *unanchored* param is unwritable by
//!     construction: every param position is itself an anchor) and
//!     non-dense param ids
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
//! 17. planner caps: more than `MAX_OCCURRENCES` atoms or more than 128
//!     distinct variables (rejected here so downstream id widths and
//!     bitset sizes are true invariants)

use std::collections::{BTreeMap, BTreeSet};

use crate::error::ValidationError;
use crate::ir::{AggOp, CmpOp, Comparison, FindTerm, ParamId, Query, Term, VarId};
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

    /// Every param with its resolved type, in id order (bind-time checking,
    /// The 30-execution doc).
    pub fn param_types(&self) -> impl Iterator<Item = (ParamId, &ValueType)> {
        self.param_types.iter().map(|(p, t)| (*p, t))
    }

    /// The group key: non-aggregated find variables (test observability;
    /// production reads it only through [`Self::sink_vars`]).
    #[cfg(test)]
    #[must_use]
    pub fn group_key(&self) -> &BTreeSet<VarId> {
        &self.group_key
    }

    /// The plan's sink-relevance set (the D2 gating bits' source). For a
    /// pure projection it is the group key — the suffix skip may cross
    /// nodes binding nothing projected. For an aggregate-bearing find
    /// list it is **every** variable: the fold is defined over the
    /// distinct full binding set, so no node's bindings are skippable,
    /// and the `sink_relevant` bits themselves encode the illegality —
    /// any `SkipSuffix` a future sink ever signaled under an aggregate
    /// plan is absorbed at the node that produced it.
    #[must_use]
    pub fn sink_vars(&self) -> BTreeSet<VarId> {
        let has_aggregate = self
            .query
            .finds
            .iter()
            .any(|term| matches!(term, FindTerm::Aggregate { .. }));
        if has_aggregate {
            self.var_types.keys().copied().collect()
        } else {
            self.group_key.clone()
        }
    }
}

/// The structural type of a literal, for matching against a field or
/// variable type — the shared [`crate::ir::value_matches`] check, so a
/// non-UTF-8 `String` literal is a type mismatch here exactly as it is at
/// bind time and on the dynamic write path.
use crate::ir::{value_matches as literal_matches, ValueMismatch as LiteralMismatch};

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
    // The planner caps are roster items: rejected here, at the boundary,
    // so nothing downstream (normalize's u16 occurrence ids, the DP's
    // bitmask table, the 128-bit variable bitsets) ever sees an
    // over-limit query.
    if query.atoms.len() > crate::plan::planner::MAX_OCCURRENCES {
        return Err(ValidationError::TooManyAtoms {
            count: query.atoms.len(),
        });
    }
    for (index, term) in query.finds.iter().enumerate() {
        if query.finds[..index].contains(term) {
            return Err(ValidationError::DuplicateFindTerm { index });
        }
    }

    let mut ctx = Context::default();
    ctx.check_atoms(schema, query)?;
    ctx.check_comparisons(query)?;
    // The group key (non-aggregated find variables) is computed once and
    // shared between the find checks and the witness.
    let group_key: BTreeSet<VarId> = query
        .finds
        .iter()
        .filter_map(|term| match term {
            FindTerm::Var(var) => Some(*var),
            FindTerm::Aggregate { .. } => None,
        })
        .collect();
    ctx.check_finds(query, &group_key)?;
    if ctx.var_types.len() > crate::plan::planner::MAX_DISTINCT_VARS {
        return Err(ValidationError::TooManyVariables {
            count: ctx.var_types.len(),
        });
    }

    Ok(ValidatedQuery {
        query: query.clone(),
        var_types: ctx.var_types,
        param_types: ctx.param_types,
        group_key,
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
                        // A non-UTF-8 String literal is a type mismatch:
                        // `Value::String` documents the UTF-8 contract.
                        Err(LiteralMismatch::Type | LiteralMismatch::Utf8) => {
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
            // A comparison of a variable with itself is constant-valued —
            // the "write the query you mean" rule applies exactly as it
            // does to literal-vs-literal.
            if let (Term::Var(l), Term::Var(r)) = (lhs, rhs) {
                if l == r {
                    return Err(ValidationError::SelfComparison { index });
                }
            }
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
                    // The precise diagnosis, exactly as the atom-binding
                    // path reports it (atom = usize::MAX is unavailable
                    // here: the ordinal names the comparison instead).
                    Err(LiteralMismatch::EnumOrdinal(ordinal)) => {
                        return Err(ValidationError::ComparisonEnumOrdinalOutOfRange {
                            index,
                            ordinal,
                        });
                    }
                    Err(LiteralMismatch::Type | LiteralMismatch::Utf8) => {
                        return Err(ValidationError::IllegalComparison { index });
                    }
                },
            }
        }
        // A param with no anchor is unwritable by construction: every
        // param position is itself an anchor (a field binding types it
        // immediately; a comparison against a variable types it via the
        // variable; param-only comparisons are already
        // `ConstantComparison`) — the roster item is discharged by
        // representation, not by a check.
        //
        // Param ids must be dense: a gap would be a positional slot at
        // execution whose supplied value is never type-checked.
        for (position, param) in self.params_seen.iter().enumerate() {
            if usize::from(param.0) != position {
                return Err(ValidationError::ParamIdGap {
                    param: ParamId(u16::try_from(position).expect("param ids fit u16")),
                });
            }
        }
        Ok(())
    }

    fn check_finds(
        &self,
        query: &Query,
        group_key: &BTreeSet<VarId>,
    ) -> Result<(), ValidationError> {
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
    use crate::ir::Value;
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
    fn rejects_self_comparison() {
        // x < x is constant-valued: write the query you mean.
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![atom(HOLDER, vec![(0, var(0))])],
            predicates: vec![Comparison {
                op: CmpOp::Lt,
                lhs: var(0),
                rhs: var(0),
            }],
        };
        let err = validate(&schema(), &query).unwrap_err();
        assert!(matches!(err, ValidationError::SelfComparison { index: 0 }));
    }

    #[test]
    fn rejects_order_operators_on_bool_and_enum() {
        // Posting.flag is Bool (field 5); Account.status is Enum (field 2).
        for (rel, field) in [(POSTING, 5u16), (ACCOUNT, 2u16)] {
            let query = Query {
                finds: vec![FindTerm::Var(VarId(0))],
                atoms: vec![
                    atom(rel, vec![(field, var(0)), (0, var(1))]),
                    atom(rel, vec![(field, var(2)), (0, var(3))]),
                ],
                predicates: vec![Comparison {
                    op: CmpOp::Lt,
                    lhs: var(0),
                    rhs: var(2),
                }],
            };
            let err = validate(&schema(), &query).unwrap_err();
            assert!(
                matches!(err, ValidationError::IllegalComparison { index: 0 }),
                "order ops are integer-only; got {err:?}"
            );
        }
    }

    #[test]
    fn enum_ordinal_in_a_comparison_reports_the_precise_variant() {
        // Account.status has 3 variants; ordinal 9 is out of range.
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![atom(ACCOUNT, vec![(2, var(0))])],
            predicates: vec![Comparison {
                op: CmpOp::Eq,
                lhs: var(0),
                rhs: Term::Literal(Value::Enum(9)),
            }],
        };
        let err = validate(&schema(), &query).unwrap_err();
        assert!(matches!(
            err,
            ValidationError::ComparisonEnumOrdinalOutOfRange {
                index: 0,
                ordinal: 9
            }
        ));
    }

    #[test]
    fn rejects_duplicate_aggregate_find_terms() {
        let query = Query {
            finds: vec![
                FindTerm::Aggregate {
                    op: AggOp::Count,
                    over: None,
                },
                FindTerm::Aggregate {
                    op: AggOp::Count,
                    over: None,
                },
            ],
            atoms: vec![atom(HOLDER, vec![(0, var(0))])],
            predicates: vec![],
        };
        let err = validate(&schema(), &query).unwrap_err();
        assert!(matches!(
            err,
            ValidationError::DuplicateFindTerm { index: 1 }
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
    fn param_anchoring_is_total_by_construction() {
        // An unanchored param is unwritable: a param in an atom binding is
        // typed by its field; a param in a comparison is typed by the
        // variable side (a variable-free comparison is already
        // `ConstantComparison`). This pins the anchored case; the roster
        // item is discharged by representation, not by a check.
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![atom(HOLDER, vec![(0, var(0))])],
            predicates: vec![Comparison {
                op: CmpOp::Eq,
                lhs: var(0),
                rhs: Term::Param(ParamId(0)),
            }],
        };
        let witness = validate(&schema(), &query).expect("valid");
        assert_eq!(
            witness.param_types().next(),
            Some((ParamId(0), &ValueType::U64))
        );
    }

    #[test]
    fn rejects_sparse_param_ids() {
        // ?1 without ?0: the gap would be an unchecked positional slot.
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![atom(
                HOLDER,
                vec![(0, var(0)), (1, Term::Param(ParamId(1)))],
            )],
            predicates: vec![],
        };
        let err = validate(&schema(), &query).unwrap_err();
        assert!(matches!(err, ValidationError::ParamIdGap { param } if param.0 == 0));
    }

    #[test]
    fn rejects_more_atoms_than_the_planner_cap_at_the_boundary() {
        let over = crate::plan::planner::MAX_OCCURRENCES + 1;
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: (0..over).map(|_| atom(HOLDER, vec![(0, var(0))])).collect(),
            predicates: vec![],
        };
        let err = validate(&schema(), &query).unwrap_err();
        assert!(matches!(err, ValidationError::TooManyAtoms { count } if count == over));
    }

    #[test]
    fn rejects_more_distinct_variables_than_the_bitset_at_the_boundary() {
        // One 129-field relation binds 129 fresh variables in a single
        // atom — past the executor's 128-bit variable bitsets.
        let wide = SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "Wide".into(),
                fields: (0..129)
                    .map(|i| FieldDescriptor {
                        name: format!("f{i}").into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    })
                    .collect(),
                constraints: vec![],
            }],
        }
        .validate()
        .expect("wide fixture");
        let query = Query {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![crate::ir::Atom {
                relation: RelationId(0),
                bindings: (0..129u16).map(|i| (FieldId(i), var(i))).collect(),
            }],
            predicates: vec![],
        };
        let err = validate(&wide, &query).unwrap_err();
        assert!(matches!(
            err,
            ValidationError::TooManyVariables { count: 129 }
        ));
    }
}
