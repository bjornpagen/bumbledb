use super::Context;
use crate::error::ValidationError;
use crate::ir::{AggOp, CmpOp, Comparison, FindTerm, ParamId, Query, Term, VarId};
use crate::schema::{Schema, ValueType};
use std::collections::BTreeSet;

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

    pub(super) fn check_atoms(
        &mut self,
        schema: &Schema,
        query: &Query,
    ) -> Result<(), ValidationError> {
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

    pub(super) fn check_comparisons(&mut self, query: &Query) -> Result<(), ValidationError> {
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

    pub(super) fn check_finds(
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
