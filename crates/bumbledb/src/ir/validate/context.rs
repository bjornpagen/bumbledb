use super::{ClassifiedComparison, Context, DurationOperand, ParamKind, SealedConst, TypeSlot};
use crate::allen::AllenMask;
use crate::error::ValidationError;
use crate::image::view::MaskConst;
use crate::ir::normalize::LoweredRule;
use crate::ir::{CmpOp, Comparison, MaskTerm, ParamId, Term, Value, VarId};
use crate::schema::{FieldId, IntervalElement, Schema, ValueType};

/// The structural type of a literal, for matching against a field or
/// variable type — the shared [`crate::schema::value_matches`] check, so a
/// non-UTF-8 `String` literal is a type mismatch here exactly as it is at
/// bind time and on the dynamic write path.
use crate::schema::{ValueMismatch as LiteralMismatch, value_matches as literal_matches};

/// The scalar type of an interval's element domain.
fn element_type(element: IntervalElement) -> ValueType {
    match element {
        IntervalElement::U64 => ValueType::U64,
        IntervalElement::I64 => ValueType::I64,
    }
}

/// Whether `candidate` is one of a bivalent slot's two readings: the
/// element type (membership) or the interval type (value equality).
fn bivalent_admits(element: IntervalElement, candidate: &ValueType) -> bool {
    *candidate == element_type(element) || *candidate == ValueType::Interval { element }
}

/// The structural type a literal contributes as a comparison anchor.
/// `None` for a mask literal: it names no data-model type, so it anchors
/// nothing — it is *checked* against the other side's type instead.
fn literal_anchor_type(value: &Value) -> Option<ValueType> {
    Some(match value {
        Value::Bool(_) => ValueType::Bool,
        Value::U64(_) => ValueType::U64,
        Value::I64(_) => ValueType::I64,
        Value::String(_) => ValueType::String,
        // The length is the type: a bytes<N> literal anchors bytes<N>.
        Value::FixedBytes(raw) => ValueType::FixedBytes {
            len: u16::try_from(raw.len()).unwrap_or(u16::MAX),
        },
        Value::IntervalU64(..) => ValueType::Interval {
            element: IntervalElement::U64,
        },
        Value::IntervalI64(..) => ValueType::Interval {
            element: IntervalElement::I64,
        },
        // A mask literal is no data-model type at all (it is only ever
        // legal inside `CmpOp::Allen`'s mask position, never as a term) —
        // it anchors nothing and is checked against the other side.
        Value::AllenMask(_) => return None,
    })
}

/// Whether an element-typed value sits at its domain ceiling (`MAX`) —
/// outside the point domain `MIN ..= MAX−1`: `MAX` is the ray's ∞, never
/// a point (`docs/architecture/10-data-model.md`, the point-domain law).
fn at_domain_ceiling(value: &Value) -> bool {
    matches!(value, Value::U64(u64::MAX) | Value::I64(i64::MAX))
}

/// A literal in an interval-field binding: element-typed means point
/// membership, while interval-typed (same element) means value equality.
/// Interval literals are nonempty by construction.
fn check_interval_field_literal(
    atom: usize,
    field: FieldId,
    element: IntervalElement,
    value: &Value,
) -> Result<(), ValidationError> {
    match (value, element) {
        // Membership: `start <= t < end` — and the point domain is
        // `MIN ..= MAX−1`, so the ceiling can be inside no interval.
        (Value::U64(_), IntervalElement::U64) | (Value::I64(_), IntervalElement::I64) => {
            if at_domain_ceiling(value) {
                Err(ValidationError::PointLiteralAtCeiling { atom, field })
            } else {
                Ok(())
            }
        }
        // Value equality against the field's intervals.
        (Value::IntervalU64(_), IntervalElement::U64)
        | (Value::IntervalI64(_), IntervalElement::I64) => Ok(()),
        _ => Err(ValidationError::LiteralTypeMismatch { atom, field }),
    }
}

/// The operator's class — the dimension a comparison's operand roster
/// depends on, matched exactly once per comparison. The order operators
/// precompute their mirror here, so no later phase re-matches an
/// operator to flip it.
#[derive(Clone, Copy)]
enum OpClass {
    /// `Eq`/`Ne` (`negated` = `Ne`).
    Equality { negated: bool },
    /// `Lt`/`Le`/`Gt`/`Ge`, with the mirrored operator alongside.
    Order { op: CmpOp, mirror: CmpOp },
    /// `Allen { mask }`.
    Allen { mask: MaskTerm },
    /// `PointIn`.
    PointIn,
}

impl OpClass {
    fn of(op: CmpOp) -> Self {
        match op {
            CmpOp::Eq => Self::Equality { negated: false },
            CmpOp::Ne => Self::Equality { negated: true },
            CmpOp::Lt => Self::Order {
                op: CmpOp::Lt,
                mirror: CmpOp::Gt,
            },
            CmpOp::Le => Self::Order {
                op: CmpOp::Le,
                mirror: CmpOp::Ge,
            },
            CmpOp::Gt => Self::Order {
                op: CmpOp::Gt,
                mirror: CmpOp::Lt,
            },
            CmpOp::Ge => Self::Order {
                op: CmpOp::Ge,
                mirror: CmpOp::Le,
            },
            CmpOp::Allen { mask } => Self::Allen { mask },
            CmpOp::PointIn => Self::PointIn,
        }
    }
}

/// One comparison's operand shape, proven and sealed by the shape pass
/// ([`Context::comparison_shape`]): the operator class fused with exactly
/// the operand roster the shape rules admit under it, so the typed pass
/// ([`Context::classify`]) matches only representable shapes — the
/// rejected ones (constant comparisons, two measures, a measure outside
/// the order operators, a set outside `Eq`) exist as typed errors, never
/// as arms. Literal sides borrow the rule; the typed pass seals owned
/// values into [`ClassifiedComparison`].
enum Shaped<'rule> {
    /// `Eq`/`Ne` over two distinct variables.
    EqVarVar {
        negated: bool,
        lhs: VarId,
        rhs: VarId,
    },
    /// `Eq`/`Ne` against a scalar constant (written order kept for the
    /// canonicalized interval form's mask converse).
    EqVarConst {
        negated: bool,
        var: VarId,
        var_on_left: bool,
        constant: ConstSide<'rule>,
    },
    /// `Eq` against the set marker (legal under `Eq` alone).
    EqVarSet { var: VarId, set: ParamId },
    /// An order comparison over two variables.
    OrdVarVar { op: CmpOp, lhs: VarId, rhs: VarId },
    /// An order comparison against a constant — the operator already
    /// sealed variable-on-left; the written order kept for the operand
    /// screen's diagnostic order.
    OrdVarConst {
        op: CmpOp,
        var: VarId,
        var_on_left: bool,
        constant: ConstSide<'rule>,
    },
    /// The measure against a variable, the operator sealed
    /// measure-on-left.
    OrdMeasureVar {
        op: CmpOp,
        interval: VarId,
        scalar: VarId,
    },
    /// The measure against a constant, the operator sealed
    /// measure-on-left.
    OrdMeasureConst {
        op: CmpOp,
        interval: VarId,
        constant: ConstSide<'rule>,
    },
    /// `Allen` over two variables, the mask as written.
    AllenVarVar {
        mask: MaskTerm,
        lhs: VarId,
        rhs: VarId,
    },
    /// `Allen` against a constant (written order kept for the mask
    /// converse).
    AllenVarConst {
        mask: MaskTerm,
        var: VarId,
        var_on_left: bool,
        constant: ConstSide<'rule>,
    },
    /// `PointIn` over two variables, written order (`lhs ∋ rhs`).
    PointInVarVar { lhs: VarId, rhs: VarId },
    /// `var ∋ constant`.
    PointInVarConst {
        var: VarId,
        constant: ConstSide<'rule>,
    },
    /// `constant ∋ var`.
    PointInConstVar {
        constant: ConstSide<'rule>,
        var: VarId,
    },
}

/// A proven constant comparison side: the param handle or the literal.
enum ConstSide<'rule> {
    Param(ParamId),
    Literal(&'rule Value),
}

/// Seals a proven variable-vs-constant operand pair under its operator
/// class: the order form seals the operator variable-on-left (a
/// constant-first comparison mirrors — the mirror rode in on
/// [`OpClass::Order`]); the containment form seals its direction as a
/// variant.
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

/// Interval equality's canonical mask: interval `Eq`/`Ne` are the derived
/// facts `Allen(EQUALS)` / `Allen(¬EQUALS)` — sealed at classification,
/// so exactly one interval-pair form leaves validation.
fn equals_mask(negated: bool) -> AllenMask {
    if negated {
        AllenMask::EQUALS.complement()
    } else {
        AllenMask::EQUALS
    }
}

/// The scalar operator a sealed `Eq`/`Ne` shape carries (symmetric, so
/// operand order never mirrors it).
fn equality_op(negated: bool) -> CmpOp {
    if negated { CmpOp::Ne } else { CmpOp::Eq }
}

/// The sealed mask side of an interval comparison against a constant,
/// the mirrored form pre-encoded exactly as the filter shape carries it:
/// `Allen(a, b, m) ≡ Allen(b, a, converse(m))`, so a comparison written
/// constant-first seals the field on the left and the mask conversed —
/// immediately for a literal, deferred to bind for a param
/// ([`MaskConst::ConversedParam`]).
fn sealed_mask(mask: MaskTerm, mirrored: bool) -> MaskConst {
    match (mask, mirrored) {
        (MaskTerm::Literal(mask), false) => MaskConst::Mask(mask),
        (MaskTerm::Literal(mask), true) => MaskConst::Mask(mask.converse()),
        (MaskTerm::Param(param), false) => MaskConst::Param(param),
        (MaskTerm::Param(param), true) => MaskConst::ConversedParam(param),
    }
}

/// The order operators' operand screen: every equality-only type gets its
/// dedicated diagnostic before accepted comparison classification.
fn screen_order_operand(index: usize, operand: Option<&ValueType>) -> Result<(), ValidationError> {
    match operand {
        Some(ValueType::Interval { .. }) => {
            Err(ValidationError::OrderComparisonOnInterval { index })
        }
        Some(ValueType::FixedBytes { .. }) => {
            Err(ValidationError::OrderComparisonOnFixedBytes { index })
        }
        Some(ValueType::String) => Err(ValidationError::OrderComparisonOnString { index }),
        Some(ValueType::Bool) => Err(ValidationError::OrderComparisonOnBool { index }),
        _ => Ok(()),
    }
}

impl Context {
    // --- anchoring -------------------------------------------------------

    fn bind_var_mono(&mut self, var: VarId, value_type: &ValueType) -> Result<(), ValidationError> {
        match self.var_slots.get(&var) {
            Some(TypeSlot::Mono(existing)) if existing != value_type => {
                Err(ValidationError::VariableTypeConflict { var })
            }
            Some(TypeSlot::Mono(_)) => Ok(()),
            Some(TypeSlot::Bivalent(element)) => {
                if bivalent_admits(*element, value_type) {
                    self.var_slots
                        .insert(var, TypeSlot::Mono(value_type.clone()));
                    Ok(())
                } else {
                    Err(ValidationError::VariableTypeConflict { var })
                }
            }
            None => {
                self.var_slots
                    .insert(var, TypeSlot::Mono(value_type.clone()));
                Ok(())
            }
        }
    }

    fn bind_var_bivalent(
        &mut self,
        var: VarId,
        element: IntervalElement,
    ) -> Result<(), ValidationError> {
        match self.var_slots.get(&var) {
            Some(TypeSlot::Mono(existing)) => {
                if bivalent_admits(element, existing) {
                    Ok(())
                } else {
                    Err(ValidationError::VariableTypeConflict { var })
                }
            }
            Some(TypeSlot::Bivalent(existing)) => {
                if *existing == element {
                    Ok(())
                } else {
                    Err(ValidationError::VariableTypeConflict { var })
                }
            }
            None => {
                self.var_slots.insert(var, TypeSlot::Bivalent(element));
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
            Some(TypeSlot::Bivalent(element)) => {
                if bivalent_admits(*element, value_type) {
                    self.param_slots
                        .insert(param, TypeSlot::Mono(value_type.clone()));
                    Ok(())
                } else {
                    Err(ValidationError::ParamTypeConflict { param })
                }
            }
            None => {
                self.param_slots
                    .insert(param, TypeSlot::Mono(value_type.clone()));
                Ok(())
            }
        }
    }

    fn anchor_param_bivalent(
        &mut self,
        param: ParamId,
        element: IntervalElement,
    ) -> Result<(), ValidationError> {
        match self.param_slots.get(&param) {
            Some(TypeSlot::Mono(existing)) => {
                if bivalent_admits(element, existing) {
                    Ok(())
                } else {
                    Err(ValidationError::ParamTypeConflict { param })
                }
            }
            Some(TypeSlot::Bivalent(existing)) => {
                if *existing == element {
                    Ok(())
                } else {
                    Err(ValidationError::ParamTypeConflict { param })
                }
            }
            None => {
                self.param_slots.insert(param, TypeSlot::Bivalent(element));
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

    /// The resolved type of a variable. Callable only after
    /// [`Context::resolve_bivalents`] — the map it reads is the
    /// resolution's product, so no unresolved slot is representable
    /// here.
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: an unknown `VarId` (every
    /// comparison variable was checked atom-bound before the typed
    /// pass).
    pub(super) fn resolved_var_type(&self, var: VarId) -> &ValueType {
        &self.var_types[&var]
    }

    // --- atoms ------------------------------------------------------------

    /// Walks positive and negated atoms under one set of per-atom rules —
    /// negation is a position, not a kind of atom, so the occurrence
    /// numbering (positives first, then negated) is the only difference a
    /// diagnostic shows. Ends with the negation safety rule: a negated
    /// atom binds nothing, so its variables must come from positive atoms.
    pub(super) fn check_atoms(
        &mut self,
        schema: &Schema,
        rule: &LoweredRule,
    ) -> Result<(), ValidationError> {
        let occurrences = rule
            .atoms
            .iter()
            .map(|atom| (atom, false))
            .chain(rule.negated.iter().map(|atom| (atom, true)));
        for (occ_idx, (atom, negated)) in occurrences.enumerate() {
            if usize::try_from(atom.relation.0).expect("64-bit usize") >= schema.relations().len() {
                return Err(ValidationError::UnknownRelation {
                    atom: occ_idx,
                    relation: atom.relation,
                });
            }
            let relation = schema.relation(atom.relation);
            for (binding_idx, (field, term)) in atom.bindings.iter().enumerate() {
                if usize::from(field.0) >= relation.fields().len() {
                    return Err(ValidationError::UnknownField {
                        atom: occ_idx,
                        field: *field,
                    });
                }
                if atom.bindings[..binding_idx].iter().any(|(f, _)| f == field) {
                    return Err(ValidationError::DuplicateFieldBinding {
                        atom: occ_idx,
                        field: *field,
                    });
                }
                let field_type = &relation.field(*field).value_type;
                if let ValueType::Interval { element } = field_type {
                    self.check_interval_binding(occ_idx, negated, *field, *element, term)?;
                } else {
                    self.check_scalar_binding(occ_idx, negated, *field, field_type, term)?;
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

    /// One binding on an interval field — the membership rule: the
    /// position types its term bivalently, `Interval(element)` (value
    /// equality) or `element` (membership). Resolution:
    /// [`Context::resolve_bivalents`].
    fn check_interval_binding(
        &mut self,
        occ_idx: usize,
        negated: bool,
        field: FieldId,
        element: IntervalElement,
        term: &Term,
    ) -> Result<(), ValidationError> {
        match term {
            Term::Var(var) => {
                self.bind_var_bivalent(*var, element)?;
                if negated {
                    self.negated_vars.insert(*var);
                } else {
                    self.atom_vars.insert(*var);
                }
            }
            Term::Param(param) => {
                self.note_param_kind(*param, ParamKind::Scalar)?;
                self.anchor_param_bivalent(*param, element)?;
                self.interval_position_params.insert(*param);
            }
            // A set holds points, so an interval-field position anchors
            // it at the element type — membership per element, never
            // interval equality.
            Term::ParamSet(param) => {
                self.note_param_kind(*param, ParamKind::Set)?;
                self.anchor_param_mono(*param, &element_type(element))?;
                self.interval_position_params.insert(*param);
            }
            Term::Literal(value) => {
                check_interval_field_literal(occ_idx, field, element, value)?;
            }
            // The measure is a computation over a bound variable, not a
            // bindable value (docs/architecture/20-query-ir.md, § the
            // measure).
            Term::Measure(_) => {
                return Err(ValidationError::DurationInBinding {
                    atom: occ_idx,
                    field,
                });
            }
        }
        Ok(())
    }

    /// One binding on a scalar field: a monovalent anchor for every term
    /// kind, with the literal precisely diagnosed.
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
            Term::Measure(_) => {
                return Err(ValidationError::DurationInBinding {
                    atom: occ_idx,
                    field,
                });
            }
            Term::Literal(value) => match literal_matches(value, field_type) {
                Ok(()) => {}
                // A non-UTF-8 String literal is a type mismatch:
                // `Value::String` documents the UTF-8 contract.
                Err(LiteralMismatch::Type | LiteralMismatch::Utf8) => {
                    return Err(ValidationError::LiteralTypeMismatch {
                        atom: occ_idx,
                        field,
                    });
                }
            },
        }
        Ok(())
    }

    // --- comparisons ------------------------------------------------------

    /// The three comparison phases, ending in the seal: the shape pass
    /// proves the operand forms ([`Shaped`]), the anchor fixpoint and
    /// bivalent resolution fix every type, and the typed pass proves
    /// per-operator legality — constructing the [`ClassifiedComparison`]
    /// each proof establishes, on the same lines.
    pub(super) fn check_comparisons(
        &mut self,
        rule: &LoweredRule,
    ) -> Result<Vec<ClassifiedComparison>, ValidationError> {
        let shaped = self.comparison_shapes(rule)?;
        self.propagate_comparison_anchors(rule)?;
        self.resolve_bivalents();
        // A param with no anchor is unwritable by construction: every
        // param position is itself an anchor (a field binding types it
        // immediately; a comparison against a variable types it via the
        // variable; param-only comparisons are already
        // `ConstantComparison`) — the roster item is discharged by
        // representation, not by a check. The two whole-program param
        // rules — mask-vs-value conflicts and id density — are checked
        // after every rule contributed (params are query-global;
        // `validate::ParamTables`).
        self.classify_comparisons(&shaped)
    }

    /// Shape rules that need no types: self-comparisons, constant
    /// comparisons (no variable side), comparison-only variables, param
    /// roles, the measure discipline, and the `ParamSet`-only-under-`Eq`
    /// rule — one sealed [`Shaped`] per condition, in condition order.
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

    /// One comparison's shape judgment: a single exhaustive match over
    /// the operand pair whose arms either reject — the same typed errors
    /// as ever, in the same diagnostic priority (mask vacuity, self
    /// comparison, the measure discipline, constant comparisons, then
    /// the per-side rules in written order) — or seal the proven
    /// [`Shaped`] form. The proof and the seal are the same lines, so
    /// no rejected shape is ever represented.
    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )] // one arm per operand-pair shape, in diagnostic priority order
    fn comparison_shape<'rule>(
        &mut self,
        index: usize,
        comparison: &'rule Comparison,
    ) -> Result<Shaped<'rule>, ValidationError> {
        let Comparison { op, lhs, rhs } = comparison;
        let class = OpClass::of(*op);
        // The Allen mask position first: both vacuity rules for literals
        // (∅ = "never": write no query; full = "always": write no
        // condition) and the roster registration for params (their
        // vacuity is checked at bind, where the value exists).
        if let OpClass::Allen { mask } = class {
            match mask {
                MaskTerm::Literal(mask) => {
                    if mask.is_empty() {
                        return Err(ValidationError::EmptyAllenMask { index });
                    }
                    if mask.is_full() {
                        return Err(ValidationError::FullAllenMask { index });
                    }
                }
                MaskTerm::Param(param) => {
                    self.note_param_kind(param, ParamKind::Scalar)?;
                    self.mask_params.insert(param);
                }
            }
        }
        match (lhs, rhs) {
            // A comparison of a variable with itself is constant-valued —
            // the "write the query you mean" rule applies exactly as it
            // does to literal-vs-literal.
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
            // The measure's comparison discipline (20-query-ir, § the
            // measure): one `Duration` side at most, and only under the
            // order operators — sealed measure-on-left (a comparison
            // written measure-second mirrors its operator).
            (Term::Measure(_), Term::Measure(_)) => {
                Err(ValidationError::DurationBothSides { index })
            }
            (Term::Measure(interval), Term::Var(scalar))
            | (Term::Var(scalar), Term::Measure(interval)) => {
                let OpClass::Order { op, mirror } = class else {
                    return Err(ValidationError::DurationComparisonOperator { index });
                };
                let measure_on_left = matches!(lhs, Term::Measure(_));
                if measure_on_left {
                    self.comparison_var(*interval)?;
                    self.comparison_var(*scalar)?;
                } else {
                    self.comparison_var(*scalar)?;
                    self.comparison_var(*interval)?;
                }
                Ok(Shaped::OrdMeasureVar {
                    op: if measure_on_left { op } else { mirror },
                    interval: *interval,
                    scalar: *scalar,
                })
            }
            (Term::Measure(interval), Term::Param(param))
            | (Term::Param(param), Term::Measure(interval)) => {
                let OpClass::Order { op, mirror } = class else {
                    return Err(ValidationError::DurationComparisonOperator { index });
                };
                let measure_on_left = matches!(lhs, Term::Measure(_));
                if measure_on_left {
                    self.comparison_var(*interval)?;
                    self.note_param_kind(*param, ParamKind::Scalar)?;
                } else {
                    self.note_param_kind(*param, ParamKind::Scalar)?;
                    self.comparison_var(*interval)?;
                }
                Ok(Shaped::OrdMeasureConst {
                    op: if measure_on_left { op } else { mirror },
                    interval: *interval,
                    constant: ConstSide::Param(*param),
                })
            }
            (Term::Measure(interval), Term::Literal(value))
            | (Term::Literal(value), Term::Measure(interval)) => {
                let OpClass::Order { op, mirror } = class else {
                    return Err(ValidationError::DurationComparisonOperator { index });
                };
                self.comparison_var(*interval)?;
                Ok(Shaped::OrdMeasureConst {
                    op: if matches!(lhs, Term::Measure(_)) {
                        op
                    } else {
                        mirror
                    },
                    interval: *interval,
                    constant: ConstSide::Literal(value),
                })
            }
            (Term::Measure(interval), Term::ParamSet(param))
            | (Term::ParamSet(param), Term::Measure(interval)) => {
                if !matches!(class, OpClass::Order { .. }) {
                    return Err(ValidationError::DurationComparisonOperator { index });
                }
                // An order operator is never `Eq`, so the set side is
                // illegal whichever side it was written on — after the
                // written-order checks that outrank it.
                if matches!(lhs, Term::Measure(_)) {
                    self.comparison_var(*interval)?;
                }
                self.note_param_kind(*param, ParamKind::Set)?;
                Err(ValidationError::ParamSetComparison { index })
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
            // No variable side and no measure side: a constant comparison
            // — write the query you mean.
            (
                Term::Param(_) | Term::ParamSet(_) | Term::Literal(_),
                Term::Param(_) | Term::ParamSet(_) | Term::Literal(_),
            ) => Err(ValidationError::ConstantComparison { index }),
        }
    }

    /// A comparison-position variable must already be atom-bound —
    /// comparisons bind nothing (the comparison-only rejection).
    fn comparison_var(&self, var: VarId) -> Result<(), ValidationError> {
        if self.var_slots.contains_key(&var) {
            Ok(())
        } else {
            Err(ValidationError::ComparisonOnlyVariable { var })
        }
    }

    /// Monovalent-anchor propagation: under the same-type operators, a
    /// side of known type names the other side's type — collapsing a
    /// bivalent variable and anchoring an unanchored param. Runs to a
    /// fixpoint so comparison order cannot matter. Incompatibilities are
    /// left standing (never overwritten): `comparison_types` diagnoses
    /// them against final types. `PointIn` propagates nothing — its
    /// right side is legally either reading of the left (the predicate
    /// form of the membership rule), so neither side names the other.
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

    /// The monovalent type a term contributes right now, if any.
    fn term_mono_type(&self, term: &Term) -> Option<ValueType> {
        match term {
            Term::Var(var) => match self.var_slots.get(var) {
                Some(TypeSlot::Mono(value_type)) => Some(value_type.clone()),
                _ => None,
            },
            Term::Param(param) | Term::ParamSet(param) => match self.param_slots.get(param) {
                Some(TypeSlot::Mono(value_type)) => Some(value_type.clone()),
                _ => None,
            },
            Term::Literal(value) => literal_anchor_type(value),
            // The measure is u64-valued by definition, whatever its
            // variable resolves to (the interval requirement is checked
            // in `check_order` against final types).
            Term::Measure(_) => Some(ValueType::U64),
        }
    }

    /// Collapses a bivalent variable slot or fills an empty param slot
    /// with `value_type`, when compatible; anything else is left for
    /// `comparison_types`. Returns whether a slot changed.
    fn collapse_term(&mut self, term: &Term, value_type: &ValueType) -> bool {
        match term {
            Term::Var(var) => match self.var_slots.get(var) {
                Some(TypeSlot::Bivalent(element)) if bivalent_admits(*element, value_type) => {
                    self.var_slots
                        .insert(*var, TypeSlot::Mono(value_type.clone()));
                    true
                }
                _ => false,
            },
            Term::Param(param) => match self.param_slots.get(param) {
                None => {
                    self.param_slots
                        .insert(*param, TypeSlot::Mono(value_type.clone()));
                    true
                }
                Some(TypeSlot::Bivalent(element)) if bivalent_admits(*element, value_type) => {
                    self.param_slots
                        .insert(*param, TypeSlot::Mono(value_type.clone()));
                    true
                }
                _ => false,
            },
            // A set never takes an interval type; its collapse would be
            // its own error, diagnosed in `comparison_types` — and a
            // measure names its own type (u64), never its variable's.
            Term::ParamSet(_) | Term::Literal(_) | Term::Measure(_) => false,
        }
    }

    /// Bivalent-anchor resolution — the one subtle typing rule
    /// (`docs/architecture/20-query-ir.md` § membership),
    /// implemented exactly once, here.
    ///
    /// A binding `(field: Interval(E), term)` does not fix the term's
    /// type: an interval-typed term means value equality, an element-typed
    /// term means point membership. Inference therefore records such a
    /// position as a *bivalent* anchor `{Interval(E) | E}`
    /// ([`TypeSlot::Bivalent`]). Resolution order:
    ///
    /// 1. Monovalent anchors — scalar field bindings, comparisons against
    ///    a term of known type, typed literals — collapse a bivalent slot
    ///    to whichever of its two candidates they name; an anchor naming
    ///    neither candidate is a type conflict (atoms) or an illegal
    ///    comparison (conditions).
    /// 2. A slot still bivalent here — every anchor was an interval-field
    ///    position — resolves to `Interval(E)`: the term is interval-typed
    ///    and each such binding is value equality. This step is why
    ///    "bound only by membership" can never arise from bindings alone:
    ///    membership needs an element-typed term, and element typing needs
    ///    a monovalent anchor.
    /// 3. Consequently the membership-only rejection
    ///    ([`Context::check_membership_domains`]) fires exactly when a
    ///    comparison collapsed a variable to the element type while all
    ///    its positive atom bindings are interval fields: element-typed,
    ///    membership-bound, no enumerable domain.
    ///
    /// The phase change is a type change: the variable slots are
    /// CONSUMED into [`Context::var_types`], so nothing after this line
    /// can see — or defensively re-match — an unresolved variable slot.
    /// Params stay slots: the typed pass still anchors them
    /// ([`Context::check_const`]).
    fn resolve_bivalents(&mut self) {
        self.var_types = std::mem::take(&mut self.var_slots)
            .into_iter()
            .map(|(var, slot)| {
                let value_type = match slot {
                    TypeSlot::Mono(value_type) => value_type,
                    TypeSlot::Bivalent(element) => ValueType::Interval { element },
                };
                (var, value_type)
            })
            .collect();
        for slot in self.param_slots.values_mut() {
            if let TypeSlot::Bivalent(element) = *slot {
                *slot = TypeSlot::Mono(ValueType::Interval { element });
            }
        }
    }

    /// The typed pass: per-operator type legality over final types, and
    /// the param anchoring those rules imply. Runs after
    /// [`Context::resolve_bivalents`], so every variable type is plain —
    /// and consumes the shape pass's seal, so no operand form is
    /// re-derived. Each proof constructs its [`ClassifiedComparison`] on
    /// the lines that establish it.
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

    /// One comparison's typed judgment and seal.
    #[expect(
        clippy::too_many_lines,
        reason = "the linear table or protocol is clearer kept together"
    )] // one arm per proven shape, each ending in its seal
    fn classify(
        &mut self,
        index: usize,
        shape: &Shaped<'_>,
    ) -> Result<ClassifiedComparison, ValidationError> {
        match shape {
            // `Eq`/`Ne`: same structural type both sides, every type
            // legal — and interval equality canonicalizes to its derived
            // `Allen` fact (`EQUALS` / its complement), so exactly one
            // interval-pair form leaves validation.
            Shaped::EqVarVar { negated, lhs, rhs } => {
                let lhs_type = self.resolved_var_type(*lhs).clone();
                if *self.resolved_var_type(*rhs) != lhs_type {
                    return Err(ValidationError::IllegalComparison { index });
                }
                Ok(if matches!(lhs_type, ValueType::Interval { .. }) {
                    ClassifiedComparison::AllenVarVar {
                        lhs: *lhs,
                        rhs: *rhs,
                        mask: MaskTerm::Literal(equals_mask(*negated)),
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
                let var_type = self.resolved_var_type(*var).clone();
                let value = self.check_const(index, constant, &var_type)?;
                Ok(if matches!(var_type, ValueType::Interval { .. }) {
                    ClassifiedComparison::AllenVarConst {
                        var: *var,
                        other: value,
                        mask: sealed_mask(MaskTerm::Literal(equals_mask(*negated)), !var_on_left),
                    }
                } else {
                    ClassifiedComparison::VarConst {
                        op: equality_op(*negated),
                        var: *var,
                        value,
                    }
                })
            }
            // The set marker anchors at the variable side's type — unless
            // that type is an interval, the dedicated `IntervalParamSet`
            // rejection.
            Shaped::EqVarSet { var, set } => {
                let var_type = self.resolved_var_type(*var).clone();
                if matches!(var_type, ValueType::Interval { .. }) {
                    return Err(ValidationError::IntervalParamSet { param: *set });
                }
                self.anchor_param_mono(*set, &var_type)?;
                Ok(ClassifiedComparison::VarInSet {
                    var: *var,
                    set: *set,
                })
            }
            // `Lt`/`Le`/`Gt`/`Ge`: U64/U64 and I64/I64 only — the operand
            // screen first, in written order.
            Shaped::OrdVarVar { op, lhs, rhs } => {
                for var in [lhs, rhs] {
                    screen_order_operand(index, Some(self.resolved_var_type(*var)))?;
                }
                let lhs_type = self.resolved_var_type(*lhs).clone();
                if !matches!(lhs_type, ValueType::U64 | ValueType::I64) {
                    return Err(ValidationError::IllegalComparison { index });
                }
                if *self.resolved_var_type(*rhs) != lhs_type {
                    return Err(ValidationError::IllegalComparison { index });
                }
                Ok(ClassifiedComparison::VarVar {
                    op: *op,
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
                let var_screen = Some(self.resolved_var_type(*var).clone());
                let const_screen = self.constant_screen(constant);
                let screens = if *var_on_left {
                    [var_screen, const_screen]
                } else {
                    [const_screen, var_screen]
                };
                for operand in &screens {
                    screen_order_operand(index, operand.as_ref())?;
                }
                let var_type = self.resolved_var_type(*var).clone();
                if !matches!(var_type, ValueType::U64 | ValueType::I64) {
                    return Err(ValidationError::IllegalComparison { index });
                }
                let value = self.check_const(index, constant, &var_type)?;
                Ok(ClassifiedComparison::VarConst {
                    op: *op,
                    var: *var,
                    value,
                })
            }
            // The measure side (20-query-ir, § the measure): the measured
            // variable must have resolved to an interval, and the value
            // side checks against u64 exactly as a u64 variable side
            // would (the measure itself is u64 by definition and never
            // screens).
            Shaped::OrdMeasureVar {
                op,
                interval,
                scalar,
            } => {
                screen_order_operand(index, Some(self.resolved_var_type(*scalar)))?;
                if !matches!(
                    self.resolved_var_type(*interval),
                    ValueType::Interval { .. }
                ) {
                    return Err(ValidationError::DurationOverNonInterval { var: *interval });
                }
                if *self.resolved_var_type(*scalar) != ValueType::U64 {
                    return Err(ValidationError::IllegalComparison { index });
                }
                Ok(ClassifiedComparison::Duration {
                    interval: *interval,
                    op: *op,
                    other: DurationOperand::Var(*scalar),
                })
            }
            Shaped::OrdMeasureConst {
                op,
                interval,
                constant,
            } => {
                screen_order_operand(index, self.constant_screen(constant).as_ref())?;
                if !matches!(
                    self.resolved_var_type(*interval),
                    ValueType::Interval { .. }
                ) {
                    return Err(ValidationError::DurationOverNonInterval { var: *interval });
                }
                let value = self.check_const(index, constant, &ValueType::U64)?;
                Ok(ClassifiedComparison::Duration {
                    interval: *interval,
                    op: *op,
                    other: DurationOperand::Const(value),
                })
            }
            // `Allen { mask }`: two interval terms of one element type —
            // the one interval-pair comparison (the mask itself was
            // checked at the shape pass; params get the vacuity rules at
            // bind).
            Shaped::AllenVarVar { mask, lhs, rhs } => {
                let lhs_type = self.resolved_var_type(*lhs).clone();
                if !matches!(lhs_type, ValueType::Interval { .. }) {
                    return Err(ValidationError::IllegalComparison { index });
                }
                if *self.resolved_var_type(*rhs) != lhs_type {
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
                let var_type = self.resolved_var_type(*var).clone();
                if !matches!(var_type, ValueType::Interval { .. }) {
                    return Err(ValidationError::IllegalComparison { index });
                }
                let other = self.check_const(index, constant, &var_type)?;
                Ok(ClassifiedComparison::AllenVarConst {
                    var: *var,
                    other,
                    mask: sealed_mask(*mask, !var_on_left),
                })
            }
            // `PointIn`: point membership as a predicate — an interval
            // side, an **element-typed** point side (the predicate form
            // of the membership binding rule, for terms already bound
            // elsewhere). The interval⊇interval form is gone: that
            // predicate is `Allen(COVERS)`, and an interval-typed point
            // side is an illegal comparison.
            Shaped::PointInVarVar { lhs, rhs } => {
                let ValueType::Interval { element } = *self.resolved_var_type(*lhs) else {
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
                let ValueType::Interval { element } = *self.resolved_var_type(*var) else {
                    return Err(ValidationError::IllegalComparison { index });
                };
                match constant {
                    ConstSide::Param(param) => {
                        // A `PointIn` point side is a point at an
                        // interval position: the ceiling rule applies at
                        // bind, where the value exists (the point-domain
                        // law).
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
                            // The point domain is `MIN ..= MAX−1`.
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
                    // The point side is the variable: its element type
                    // names the param's interval domain.
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
                    let Some(ValueType::Interval { element }) = literal_anchor_type(value) else {
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

    /// One sealed constant side against the anchoring side's resolved
    /// type: a param anchors there ([`Context::anchor_param_mono`]); a
    /// literal is checked precisely, exactly as the atom-binding path
    /// reports it — and the passing proof seals the side.
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

    /// The type a constant side contributes to the order screen right
    /// now, if any: the param's anchored type, or the literal's own.
    fn constant_screen(&self, constant: &ConstSide<'_>) -> Option<ValueType> {
        match constant {
            ConstSide::Param(param) => match self.param_slots.get(param) {
                Some(TypeSlot::Mono(value_type)) => Some(value_type.clone()),
                _ => None,
            },
            ConstSide::Literal(value) => literal_anchor_type(value),
        }
    }

    /// A literal against a comparison side's resolved type — the precise
    /// diagnosis, exactly as the atom-binding path reports it (the ordinal
    /// names the comparison instead of an atom).
    #[expect(
        clippy::unused_self,
        reason = "the receiver keeps this checker API shape-parallel"
    )] // shape-parallel with the sibling checkers
    fn check_literal_against(
        &self,
        index: usize,
        value: &Value,
        expected: &ValueType,
    ) -> Result<(), ValidationError> {
        match literal_matches(value, expected) {
            Ok(()) => Ok(()),
            Err(LiteralMismatch::Type | LiteralMismatch::Utf8) => {
                Err(ValidationError::IllegalComparison { index })
            }
        }
    }

    // --- membership domains -------------------------------------------

    /// The membership-only rejection (step 3 of the resolution order on
    /// [`Context::resolve_bivalents`]): an element-typed variable whose
    /// positive atom bindings are all interval fields is bound only by
    /// membership — no enumerable domain.
    pub(super) fn check_membership_domains(&self) -> Result<(), ValidationError> {
        for (var, value_type) in &self.var_types {
            if matches!(value_type, ValueType::Interval { .. }) {
                continue;
            }
            if self.atom_vars.contains(var) && !self.scalar_bound_vars.contains(var) {
                return Err(ValidationError::MembershipOnlyVariable { var: *var });
            }
        }
        Ok(())
    }
}
