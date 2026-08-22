use super::{ClassifiedComparison, Context, ParamKind, SealedConst, TypeSlot};
use crate::error::{AtomIndex, ValidationError};
use crate::image::view::MaskConst;
use crate::ir::normalize::LoweredRule;
use crate::ir::{CmpOp, Comparison, ParamId, Term, Value, VarId};
use crate::schema::Schema;
use bumbledb_theory::allen::AllenMask;
use bumbledb_theory::schema::{FieldId, IntervalElement, ValueType};

use bumbledb_theory::schema::{ValueMismatch as LiteralMismatch, value_matches as literal_matches};

fn element_type(element: IntervalElement) -> ValueType {
    match element {
        IntervalElement::U64 => ValueType::U64,
        IntervalElement::I64 => ValueType::I64,
    }
}

fn bivalent_admits(interval: &ValueType, candidate: &ValueType) -> bool {
    interval
        .interval_element()
        .is_some_and(|element| *candidate == element_type(element) || candidate == interval)
}

fn literal_anchor_type(value: &Value) -> ValueType {
    match value {
        Value::Bool(_) => ValueType::Bool,
        Value::U64(_) => ValueType::U64,
        Value::I64(_) => ValueType::I64,
        Value::String(_) => ValueType::String,

        Value::FixedBytes(raw) => ValueType::FixedBytes {
            len: u16::try_from(raw.len()).unwrap_or(u16::MAX),
        },

        Value::IntervalU64(..) => ValueType::Interval {
            element: IntervalElement::U64,
        },
        Value::IntervalI64(..) => ValueType::Interval {
            element: IntervalElement::I64,
        },
    }
}

fn at_domain_ceiling(value: &Value) -> bool {
    matches!(value, Value::U64(u64::MAX) | Value::I64(i64::MAX))
}

fn check_interval_field_literal(
    atom: usize,
    field: FieldId,
    interval: &ValueType,
    value: &Value,
) -> Result<(), ValidationError> {
    let element = interval.interval_element().expect("interval-field binding");
    match (value, element) {

        (Value::U64(_), IntervalElement::U64) | (Value::I64(_), IntervalElement::I64) => {
            if at_domain_ceiling(value) {
                Err(ValidationError::PointLiteralAtCeiling {
                    atom: AtomIndex(atom),
                    field,
                })
            } else {
                Ok(())
            }
        }

        (Value::IntervalU64(_), IntervalElement::U64)
        | (Value::IntervalI64(_), IntervalElement::I64) => match literal_matches(value, interval) {
            Ok(()) => Ok(()),
            Err(_) => Err(ValidationError::LiteralTypeMismatch {
                atom: AtomIndex(atom),
                field,
            }),
        },
        _ => Err(ValidationError::LiteralTypeMismatch {
            atom: AtomIndex(atom),
            field,
        }),
    }
}

#[derive(Clone, Copy)]
enum OpClass {

    Equality { negated: bool },

    Order {
        op: crate::ir::OrderCmp,
        mirror: crate::ir::OrderCmp,
    },

    Allen { mask: AllenMask },

    PointIn,
}

impl OpClass {
    fn of(op: CmpOp) -> Self {
        match op {
            CmpOp::Eq => Self::Equality { negated: false },
            CmpOp::Ne => Self::Equality { negated: true },
            CmpOp::Lt => Self::Order {
                op: crate::ir::OrderCmp::Lt,
                mirror: crate::ir::OrderCmp::Gt,
            },
            CmpOp::Le => Self::Order {
                op: crate::ir::OrderCmp::Le,
                mirror: crate::ir::OrderCmp::Ge,
            },
            CmpOp::Gt => Self::Order {
                op: crate::ir::OrderCmp::Gt,
                mirror: crate::ir::OrderCmp::Lt,
            },
            CmpOp::Ge => Self::Order {
                op: crate::ir::OrderCmp::Ge,
                mirror: crate::ir::OrderCmp::Le,
            },
            CmpOp::Allen { mask } => Self::Allen { mask },
            CmpOp::PointIn => Self::PointIn,
        }
    }
}

enum Shaped<'rule> {

    EqVarVar {
        negated: bool,
        lhs: VarId,
        rhs: VarId,
    },

    EqVarConst {
        negated: bool,
        var: VarId,
        var_on_left: bool,
        constant: ConstSide<'rule>,
    },

    EqVarSet { var: VarId, set: ParamId },

    OrdVarVar {
        op: crate::ir::OrderCmp,
        lhs: VarId,
        rhs: VarId,
    },

    OrdVarConst {
        op: crate::ir::OrderCmp,
        var: VarId,
        var_on_left: bool,
        constant: ConstSide<'rule>,
    },

    AllenVarVar {
        mask: AllenMask,
        lhs: VarId,
        rhs: VarId,
    },

    AllenVarConst {
        mask: AllenMask,
        var: VarId,
        var_on_left: bool,
        constant: ConstSide<'rule>,
    },

    PointInVarVar { lhs: VarId, rhs: VarId },

    PointInVarConst {
        var: VarId,
        constant: ConstSide<'rule>,
    },

    PointInConstVar {
        constant: ConstSide<'rule>,
        var: VarId,
    },
}

enum ConstSide<'rule> {
    Param(ParamId),
    Literal(&'rule Value),
}

fn shaped_var_const(
    class: OpClass,
    var: VarId,
    var_on_left: bool,
    constant: ConstSide<'_>,
) -> Shaped<'_> {
    match class {
        OpClass::Equality { negated } => Shaped::EqVarConst {
            negated,
            var,
            var_on_left,
            constant,
        },
        OpClass::Order { op, mirror } => Shaped::OrdVarConst {
            op: if var_on_left { op } else { mirror },
            var,
            var_on_left,
            constant,
        },
        OpClass::Allen { mask } => Shaped::AllenVarConst {
            mask,
            var,
            var_on_left,
            constant,
        },
        OpClass::PointIn if var_on_left => Shaped::PointInVarConst { var, constant },
        OpClass::PointIn => Shaped::PointInConstVar { constant, var },
    }
}

fn equals_mask(negated: bool) -> AllenMask {
    if negated {
        AllenMask::EQUALS.complement()
    } else {
        AllenMask::EQUALS
    }
}

fn equality_op(negated: bool) -> crate::ir::WordCmp {
    if negated {
        crate::ir::WordCmp::Ne
    } else {
        crate::ir::WordCmp::Eq
    }
}

fn sealed_mask(mask: AllenMask, mirrored: bool) -> MaskConst {
    if mirrored { mask.converse() } else { mask }
}

/// The order operators' operand screen: every equality-only type gets its
/// dedicated diagnostic before accepted comparison classification.
fn screen_order_operand(index: usize, operand: Option<&ValueType>) -> Result<(), ValidationError> {
    match operand {
        Some(ty) if ty.is_interval() => Err(ValidationError::OrderComparisonOnInterval { index }),
        Some(ValueType::FixedBytes { .. }) => {
            Err(ValidationError::OrderComparisonOnFixedBytes { index })
        }
        Some(ValueType::String) => Err(ValidationError::OrderComparisonOnString { index }),
        _ => Ok(()),
    }
}

impl Context {
    /// The closed-reference order wall (ruled 2026-07-23, R4): a

    /// them is refused exactly as the enum's ordinal order was, judged

    fn screen_order_closed(&self, index: usize, var: VarId) -> Result<(), ValidationError> {
        if self.closed_vars.contains_key(&var) {
            return Err(ValidationError::OrderComparisonOnClosedReference { index });
        }
        Ok(())
    }

    fn bind_var_mono(&mut self, var: VarId, value_type: &ValueType) -> Result<(), ValidationError> {
        match self.var_slots.get(&var) {
            Some(TypeSlot::Mono(existing)) if existing != value_type => {
                Err(ValidationError::VariableTypeConflict { var })
            }
            Some(TypeSlot::Mono(_)) => Ok(()),
            Some(TypeSlot::Bivalent { interval }) => {
                if bivalent_admits(interval, value_type) {
                    self.var_slots.insert(var, TypeSlot::Mono(*value_type));
                    Ok(())
                } else {
                    Err(ValidationError::VariableTypeConflict { var })
                }
            }
            None => {
                self.var_slots.insert(var, TypeSlot::Mono(*value_type));
                Ok(())
            }
        }
    }

    fn bind_var_bivalent(
        &mut self,
        var: VarId,
        interval: &ValueType,
    ) -> Result<(), ValidationError> {
        match self.var_slots.get(&var) {
            Some(TypeSlot::Mono(existing)) => {
                if bivalent_admits(interval, existing) {
                    Ok(())
                } else {
                    Err(ValidationError::VariableTypeConflict { var })
                }
            }
            Some(TypeSlot::Bivalent { interval: existing }) => {
                if existing == interval {
                    Ok(())
                } else {
                    Err(ValidationError::VariableTypeConflict { var })
                }
            }
            None => {
                self.var_slots.insert(
                    var,
                    TypeSlot::Bivalent {
                        interval: *interval,
                    },
                );
                Ok(())
            }
        }
    }

    fn anchor_param_mono(
        &mut self,
        param: ParamId,
        value_type: &ValueType,
    ) -> Result<(), ValidationError> {
        match self.param_slots.get(&param) {
            Some(TypeSlot::Mono(existing)) if existing != value_type => {
                Err(ValidationError::ParamTypeConflict { param })
            }
            Some(TypeSlot::Mono(_)) => Ok(()),
            Some(TypeSlot::Bivalent { interval }) => {
                if bivalent_admits(interval, value_type) {
                    self.param_slots.insert(param, TypeSlot::Mono(*value_type));
                    Ok(())
                } else {
                    Err(ValidationError::ParamTypeConflict { param })
                }
            }
            None => {
                self.param_slots.insert(param, TypeSlot::Mono(*value_type));
                Ok(())
            }
        }
    }

    fn anchor_param_bivalent(
        &mut self,
        param: ParamId,
        interval: &ValueType,
    ) -> Result<(), ValidationError> {
        match self.param_slots.get(&param) {
            Some(TypeSlot::Mono(existing)) => {
                if bivalent_admits(interval, existing) {
                    Ok(())
                } else {
                    Err(ValidationError::ParamTypeConflict { param })
                }
            }
            Some(TypeSlot::Bivalent { interval: existing }) => {
                if existing == interval {
                    Ok(())
                } else {
                    Err(ValidationError::ParamTypeConflict { param })
                }
            }
            None => {
                self.param_slots.insert(
                    param,
                    TypeSlot::Bivalent {
                        interval: *interval,
                    },
                );
                Ok(())
            }
        }
    }

    fn note_param_kind(&mut self, param: ParamId, kind: ParamKind) -> Result<(), ValidationError> {
        match self.param_kinds.get(&param) {
            Some(existing) if *existing != kind => {
                Err(ValidationError::ParamScalarAndSet { param })
            }
            Some(_) => Ok(()),
            None => {
                self.param_kinds.insert(param, kind);
                Ok(())
            }
        }
    }

    /// # Panics

    /// On a programmer-invariant violation: an unknown `VarId` (every
    /// comparison variable was checked atom-bound before the typed

    pub(super) fn resolved_var_type(&self, var: VarId) -> &ValueType {
        &self.var_types[&var]
    }

    pub(super) fn check_atoms(
        &mut self,
        schema: &Schema,
        interiors: &super::InteriorSignatures<'_>,
        rule: &LoweredRule,
    ) -> Result<(), ValidationError> {

        let closed_refs = crate::ir::render::ClosedRefs::build(schema);
        let occurrences = rule
            .atoms
            .iter()
            .map(|atom| (atom, false))
            .chain(rule.negated.iter().map(|atom| (atom, true)));
        for (occ_idx, (atom, negated)) in occurrences.enumerate() {
            match atom.source {
                crate::ir::AtomSource::Edb(relation_id) => {
                    if usize::try_from(relation_id.0).expect("64-bit usize")
                        >= schema.relations().len()
                    {
                        return Err(ValidationError::UnknownRelation {
                            atom: AtomIndex(occ_idx),
                            relation: relation_id,
                        });
                    }
                }

                crate::ir::AtomSource::Interior(interior) => {
                    interiors.screen(occ_idx, interior)?;
                }
            }
            for (binding_idx, (field, term)) in atom.bindings.iter().enumerate() {
                if atom.bindings[..binding_idx].iter().any(|(f, _)| f == field) {
                    return Err(ValidationError::DuplicateFieldBinding {
                        atom: AtomIndex(occ_idx),
                        field: *field,
                    });
                }
                let field_type = match atom.source {
                    crate::ir::AtomSource::Edb(relation_id) => {
                        let relation = schema.relation(relation_id);
                        if usize::from(field.0) >= relation.fields().len() {
                            return Err(ValidationError::UnknownField {
                                atom: AtomIndex(occ_idx),
                                field: *field,
                            });
                        }
                        &relation.field(*field).value_type
                    }
                    crate::ir::AtomSource::Interior(interior) => {
                        interiors.column(occ_idx, interior, *field)?
                    }
                };
                if field_type.is_interval() {
                    self.check_interval_binding(occ_idx, negated, *field, field_type, term)?;
                } else {
                    self.check_scalar_binding(occ_idx, negated, *field, field_type, term)?;

                    if let crate::ir::AtomSource::Edb(relation_id) = atom.source
                        && let Term::Var(var) = term
                        && let Some(closed) = closed_refs.target(relation_id, *field)
                    {
                        let rows = schema
                            .relation(closed)
                            .body()
                            .closed_rows()
                            .map_or(0, <[_]>::len);
                        self.closed_vars
                            .insert(*var, u16::try_from(rows).expect("extensions seal at ≤256"));
                    }
                }
            }
        }
        for var in &self.negated_vars {
            if !self.atom_vars.contains(var) {
                return Err(ValidationError::NegatedVariableUnbound { var: *var });
            }
        }
        Ok(())
    }

    fn check_interval_binding(
        &mut self,
        occ_idx: usize,
        negated: bool,
        field: FieldId,
        interval: &ValueType,
        term: &Term,
    ) -> Result<(), ValidationError> {
        let element = interval.interval_element().expect("interval-field binding");
        match term {
            Term::Var(var) => {
                self.bind_var_bivalent(*var, interval)?;
                if negated {
                    self.negated_vars.insert(*var);
                } else {
                    self.atom_vars.insert(*var);
                }
            }
            Term::Param(param) => {
                self.note_param_kind(*param, ParamKind::Scalar)?;
                self.anchor_param_bivalent(*param, interval)?;
                self.interval_position_params.insert(*param);
            }

            Term::ParamSet(param) => {
                self.note_param_kind(*param, ParamKind::Set)?;
                self.anchor_param_mono(*param, &element_type(element))?;
                self.interval_position_params.insert(*param);
            }
            Term::Literal(value) => {
                check_interval_field_literal(occ_idx, field, interval, value)?;
            }
        }
        Ok(())
    }

    fn check_scalar_binding(
        &mut self,
        occ_idx: usize,
        negated: bool,
        field: FieldId,
        field_type: &ValueType,
        term: &Term,
    ) -> Result<(), ValidationError> {
        match term {
            Term::Var(var) => {
                self.bind_var_mono(*var, field_type)?;
                if negated {
                    self.negated_vars.insert(*var);
                } else {
                    self.atom_vars.insert(*var);
                    self.scalar_bound_vars.insert(*var);
                }
            }
            Term::Param(param) => {
                self.note_param_kind(*param, ParamKind::Scalar)?;
                self.anchor_param_mono(*param, field_type)?;
            }
            Term::ParamSet(param) => {
                self.note_param_kind(*param, ParamKind::Set)?;
                self.anchor_param_mono(*param, field_type)?;
            }
            Term::Literal(value) => match literal_matches(value, field_type) {
                Ok(()) => {}
                Err(LiteralMismatch::Type) => {
                    return Err(ValidationError::LiteralTypeMismatch {
                        atom: AtomIndex(occ_idx),
                        field,
                    });
                }
            },
        }
        Ok(())
    }

    pub(super) fn check_comparisons(
        &mut self,
        rule: &LoweredRule,
    ) -> Result<Vec<ClassifiedComparison>, ValidationError> {
        let shaped = self.comparison_shapes(rule)?;
        self.propagate_comparison_anchors(rule)?;
        self.resolve_bivalents();

        // after every rule contributed (params are query-global;

        self.classify_comparisons(&shaped)
    }

    fn comparison_shapes<'rule>(
        &mut self,
        rule: &'rule LoweredRule,
    ) -> Result<Vec<Shaped<'rule>>, ValidationError> {
        rule.conditions
            .iter()
            .enumerate()
            .map(|(index, comparison)| self.comparison_shape(index, comparison))
            .collect()
    }

    fn comparison_shape<'rule>(
        &mut self,
        index: usize,
        comparison: &'rule Comparison,
    ) -> Result<Shaped<'rule>, ValidationError> {
        let Comparison { op, lhs, rhs } = comparison;
        let class = OpClass::of(*op);

        if let OpClass::Allen { mask } = class {
            if mask.is_empty() {
                return Err(ValidationError::EmptyAllenMask { index });
            }
            if mask.is_full() {
                return Err(ValidationError::FullAllenMask { index });
            }
        }
        match (lhs, rhs) {

            (Term::Var(l), Term::Var(r)) if l == r => {
                Err(ValidationError::SelfComparison { index })
            }
            (Term::Var(l), Term::Var(r)) => {
                self.comparison_var(*l)?;
                self.comparison_var(*r)?;
                Ok(match class {
                    OpClass::Equality { negated } => Shaped::EqVarVar {
                        negated,
                        lhs: *l,
                        rhs: *r,
                    },
                    OpClass::Order { op, .. } => Shaped::OrdVarVar {
                        op,
                        lhs: *l,
                        rhs: *r,
                    },
                    OpClass::Allen { mask } => Shaped::AllenVarVar {
                        mask,
                        lhs: *l,
                        rhs: *r,
                    },
                    OpClass::PointIn => Shaped::PointInVarVar { lhs: *l, rhs: *r },
                })
            }
            (Term::Var(var), Term::Param(param)) | (Term::Param(param), Term::Var(var)) => {
                let var_on_left = matches!(lhs, Term::Var(_));
                if var_on_left {
                    self.comparison_var(*var)?;
                    self.note_param_kind(*param, ParamKind::Scalar)?;
                } else {
                    self.note_param_kind(*param, ParamKind::Scalar)?;
                    self.comparison_var(*var)?;
                }
                Ok(shaped_var_const(
                    class,
                    *var,
                    var_on_left,
                    ConstSide::Param(*param),
                ))
            }
            (Term::Var(var), Term::Literal(value)) | (Term::Literal(value), Term::Var(var)) => {
                self.comparison_var(*var)?;
                Ok(shaped_var_const(
                    class,
                    *var,
                    matches!(lhs, Term::Var(_)),
                    ConstSide::Literal(value),
                ))
            }
            (Term::Var(var), Term::ParamSet(param)) | (Term::ParamSet(param), Term::Var(var)) => {
                let var_on_left = matches!(lhs, Term::Var(_));
                if var_on_left {
                    self.comparison_var(*var)?;
                }
                self.note_param_kind(*param, ParamKind::Set)?;
                if !matches!(class, OpClass::Equality { negated: false }) {
                    return Err(ValidationError::ParamSetComparison { index });
                }
                if !var_on_left {
                    self.comparison_var(*var)?;
                }
                Ok(Shaped::EqVarSet {
                    var: *var,
                    set: *param,
                })
            }

            (
                Term::Param(_) | Term::ParamSet(_) | Term::Literal(_),
                Term::Param(_) | Term::ParamSet(_) | Term::Literal(_),
            ) => Err(ValidationError::ConstantComparison { index }),
        }
    }

    fn comparison_var(&self, var: VarId) -> Result<(), ValidationError> {
        if self.var_slots.contains_key(&var) {
            Ok(())
        } else {
            Err(ValidationError::ComparisonOnlyVariable { var })
        }
    }

    fn propagate_comparison_anchors(&mut self, rule: &LoweredRule) -> Result<(), ValidationError> {
        loop {
            let mut changed = false;
            for Comparison { op, lhs, rhs } in &rule.conditions {
                if matches!(op, CmpOp::PointIn) {
                    continue;
                }
                let known_lhs = self.term_mono_type(lhs);
                if let Some(value_type) = known_lhs {
                    changed |= self.collapse_term(rhs, &value_type);
                }
                let known_rhs = self.term_mono_type(rhs);
                if let Some(value_type) = known_rhs {
                    changed |= self.collapse_term(lhs, &value_type);
                }
            }
            if !changed {
                return Ok(());
            }
        }
    }

    fn term_mono_type(&self, term: &Term) -> Option<ValueType> {
        match term {
            Term::Var(var) => match self.var_slots.get(var) {
                Some(TypeSlot::Mono(value_type)) => Some(*value_type),
                _ => None,
            },
            Term::Param(param) | Term::ParamSet(param) => match self.param_slots.get(param) {
                Some(TypeSlot::Mono(value_type)) => Some(*value_type),
                _ => None,
            },
            Term::Literal(value) => Some(literal_anchor_type(value)),
        }
    }

    fn collapse_term(&mut self, term: &Term, value_type: &ValueType) -> bool {
        match term {
            Term::Var(var) => match self.var_slots.get(var) {
                Some(TypeSlot::Bivalent { interval }) if bivalent_admits(interval, value_type) => {
                    self.var_slots.insert(*var, TypeSlot::Mono(*value_type));
                    true
                }
                _ => false,
            },
            Term::Param(param) => match self.param_slots.get(param) {
                None => {
                    self.param_slots.insert(*param, TypeSlot::Mono(*value_type));
                    true
                }
                Some(TypeSlot::Bivalent { interval }) if bivalent_admits(interval, value_type) => {
                    self.param_slots.insert(*param, TypeSlot::Mono(*value_type));
                    true
                }
                _ => false,
            },

            Term::ParamSet(_) | Term::Literal(_) => false,
        }
    }

    /// CONSUMED into [`Context::var_types`], so nothing after this line

    fn resolve_bivalents(&mut self) {
        self.var_types = std::mem::take(&mut self.var_slots)
            .into_iter()
            .map(|(var, slot)| {
                let value_type = match slot {
                    TypeSlot::Mono(value_type) => value_type,
                    TypeSlot::Bivalent { interval } => interval,
                };
                (var, value_type)
            })
            .collect();
        for slot in self.param_slots.values_mut() {
            if let TypeSlot::Bivalent { interval } = slot {
                *slot = TypeSlot::Mono(*interval);
            }
        }
    }

    fn classify_comparisons(
        &mut self,
        shaped: &[Shaped<'_>],
    ) -> Result<Vec<ClassifiedComparison>, ValidationError> {
        shaped
            .iter()
            .enumerate()
            .map(|(index, shape)| self.classify(index, shape))
            .collect()
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )] 
    fn classify(
        &mut self,
        index: usize,
        shape: &Shaped<'_>,
    ) -> Result<ClassifiedComparison, ValidationError> {
        match shape {

            Shaped::EqVarVar { negated, lhs, rhs } => {
                let lhs_type = *self.resolved_var_type(*lhs);
                if *self.resolved_var_type(*rhs) != lhs_type {
                    return Err(ValidationError::IllegalComparison { index });
                }
                Ok(if lhs_type.is_interval() {
                    ClassifiedComparison::AllenVarVar {
                        lhs: *lhs,
                        rhs: *rhs,
                        mask: equals_mask(*negated),
                    }
                } else {
                    ClassifiedComparison::VarVar {
                        op: equality_op(*negated),
                        lhs: *lhs,
                        rhs: *rhs,
                    }
                })
            }
            Shaped::EqVarConst {
                negated,
                var,
                var_on_left,
                constant,
            } => {
                let var_type = *self.resolved_var_type(*var);
                let value = self.check_const(index, constant, &var_type)?;
                Ok(if var_type.is_interval() {
                    ClassifiedComparison::AllenVarConst {
                        var: *var,
                        other: value,
                        mask: sealed_mask(equals_mask(*negated), !var_on_left),
                    }
                } else {
                    ClassifiedComparison::VarConst {
                        op: equality_op(*negated),
                        var: *var,
                        value,
                    }
                })
            }

            Shaped::EqVarSet { var, set } => {
                let var_type = *self.resolved_var_type(*var);
                if var_type.is_interval() {
                    return Err(ValidationError::IntervalParamSet { param: *set });
                }
                self.anchor_param_mono(*set, &var_type)?;
                Ok(ClassifiedComparison::VarInSet {
                    var: *var,
                    set: *set,
                })
            }

            // (ordering a declaration-order accident, refused — R4).
            Shaped::OrdVarVar { op, lhs, rhs } => {
                for var in [lhs, rhs] {
                    screen_order_operand(index, Some(self.resolved_var_type(*var)))?;
                    self.screen_order_closed(index, *var)?;
                }
                let lhs_type = *self.resolved_var_type(*lhs);
                if !matches!(lhs_type, ValueType::U64 | ValueType::I64 | ValueType::Bool) {
                    return Err(ValidationError::IllegalComparison { index });
                }
                if *self.resolved_var_type(*rhs) != lhs_type {
                    return Err(ValidationError::IllegalComparison { index });
                }
                Ok(ClassifiedComparison::VarVar {
                    op: (*op).into(),
                    lhs: *lhs,
                    rhs: *rhs,
                })
            }
            Shaped::OrdVarConst {
                op,
                var,
                var_on_left,
                constant,
            } => {
                let var_screen = Some(*self.resolved_var_type(*var));
                let const_screen = self.constant_screen(constant);
                let screens = if *var_on_left {
                    [var_screen, const_screen]
                } else {
                    [const_screen, var_screen]
                };
                for operand in &screens {
                    screen_order_operand(index, operand.as_ref())?;
                }
                self.screen_order_closed(index, *var)?;
                let var_type = *self.resolved_var_type(*var);
                if !matches!(var_type, ValueType::U64 | ValueType::I64 | ValueType::Bool) {
                    return Err(ValidationError::IllegalComparison { index });
                }
                let value = self.check_const(index, constant, &var_type)?;
                Ok(ClassifiedComparison::VarConst {
                    op: (*op).into(),
                    var: *var,
                    value,
                })
            }

            Shaped::AllenVarVar { mask, lhs, rhs } => {
                let Some(lhs_element) = self.resolved_var_type(*lhs).interval_element() else {
                    return Err(ValidationError::IllegalComparison { index });
                };
                let Some(rhs_element) = self.resolved_var_type(*rhs).interval_element() else {
                    return Err(ValidationError::IllegalComparison { index });
                };
                if lhs_element != rhs_element {
                    return Err(ValidationError::IllegalComparison { index });
                }
                Ok(ClassifiedComparison::AllenVarVar {
                    lhs: *lhs,
                    rhs: *rhs,
                    mask: *mask,
                })
            }
            Shaped::AllenVarConst {
                mask,
                var,
                var_on_left,
                constant,
            } => {
                let Some(element) = self.resolved_var_type(*var).interval_element() else {
                    return Err(ValidationError::IllegalComparison { index });
                };

                let other = self.check_const(index, constant, &ValueType::Interval { element })?;
                Ok(ClassifiedComparison::AllenVarConst {
                    var: *var,
                    other,
                    mask: sealed_mask(*mask, !var_on_left),
                })
            }

            Shaped::PointInVarVar { lhs, rhs } => {
                self.screen_order_closed(index, *rhs)?;
                let Some(element) = self.resolved_var_type(*lhs).interval_element() else {
                    return Err(ValidationError::IllegalComparison { index });
                };
                if *self.resolved_var_type(*rhs) != element_type(element) {
                    return Err(ValidationError::IllegalComparison { index });
                }
                Ok(ClassifiedComparison::PointInVarVar {
                    interval: *lhs,
                    point: *rhs,
                })
            }
            Shaped::PointInVarConst { var, constant } => {
                let Some(element) = self.resolved_var_type(*var).interval_element() else {
                    return Err(ValidationError::IllegalComparison { index });
                };
                match constant {
                    ConstSide::Param(param) => {

                        self.interval_position_params.insert(*param);
                        self.anchor_param_mono(*param, &element_type(element))?;
                        Ok(ClassifiedComparison::PointInVarPoint {
                            interval: *var,
                            point: SealedConst::Param(*param),
                        })
                    }
                    ConstSide::Literal(value) => match (value, element) {
                        (Value::U64(_), IntervalElement::U64)
                        | (Value::I64(_), IntervalElement::I64) => {

                            if at_domain_ceiling(value) {
                                return Err(ValidationError::ComparisonPointLiteralAtCeiling {
                                    index,
                                });
                            }
                            Ok(ClassifiedComparison::PointInVarPoint {
                                interval: *var,
                                point: SealedConst::Literal((*value).clone()),
                            })
                        }
                        _ => Err(ValidationError::IllegalComparison { index }),
                    },
                }
            }
            Shaped::PointInConstVar { constant, var } => match constant {
                ConstSide::Param(param) => {

                    let element = match self.resolved_var_type(*var) {
                        ValueType::U64 => IntervalElement::U64,
                        ValueType::I64 => IntervalElement::I64,
                        _ => return Err(ValidationError::IllegalComparison { index }),
                    };

                    self.anchor_param_mono(*param, &ValueType::Interval { element })?;
                    Ok(ClassifiedComparison::VarWithin {
                        var: *var,
                        outer: SealedConst::Param(*param),
                    })
                }
                ConstSide::Literal(value) => {
                    let Some(element) = literal_anchor_type(value).interval_element() else {
                        return Err(ValidationError::IllegalComparison { index });
                    };
                    if *self.resolved_var_type(*var) != element_type(element) {
                        return Err(ValidationError::IllegalComparison { index });
                    }
                    Ok(ClassifiedComparison::VarWithin {
                        var: *var,
                        outer: SealedConst::Literal((*value).clone()),
                    })
                }
            },
        }
    }

    fn check_const(
        &mut self,
        index: usize,
        constant: &ConstSide<'_>,
        expected: &ValueType,
    ) -> Result<SealedConst, ValidationError> {
        match constant {
            ConstSide::Param(param) => {
                self.anchor_param_mono(*param, expected)?;
                Ok(SealedConst::Param(*param))
            }
            ConstSide::Literal(value) => {
                self.check_literal_against(index, value, expected)?;
                Ok(SealedConst::Literal((*value).clone()))
            }
        }
    }

    fn constant_screen(&self, constant: &ConstSide<'_>) -> Option<ValueType> {
        match constant {
            ConstSide::Param(param) => match self.param_slots.get(param) {
                Some(TypeSlot::Mono(value_type)) => Some(*value_type),
                _ => None,
            },
            ConstSide::Literal(value) => Some(literal_anchor_type(value)),
        }
    }

    #[expect(
        clippy::unused_self,
        reason = "the receiver keeps this checker API shape-parallel"
    )] 
    fn check_literal_against(
        &self,
        index: usize,
        value: &Value,
        expected: &ValueType,
    ) -> Result<(), ValidationError> {
        match literal_matches(value, expected) {
            Ok(()) => Ok(()),
            Err(LiteralMismatch::Type) => Err(ValidationError::IllegalComparison { index }),
        }
    }

    pub(super) fn check_membership_domains(&self) -> Result<(), ValidationError> {
        for (var, value_type) in &self.var_types {
            if value_type.is_interval() {
                continue;
            }
            if self.atom_vars.contains(var) && !self.scalar_bound_vars.contains(var) {
                return Err(ValidationError::MembershipOnlyVariable { var: *var });
            }
        }
        Ok(())
    }
}
