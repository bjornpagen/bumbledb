use super::{Context, ParamKind, TypeSlot};
use crate::error::ValidationError;
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
/// membership, interval-typed (same element) means value equality — and
/// an interval literal with `start >= end` denotes no points.
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
        (Value::IntervalU64(start, end), IntervalElement::U64) => {
            if start < end {
                Ok(())
            } else {
                Err(ValidationError::EmptyIntervalLiteral { atom, field })
            }
        }
        (Value::IntervalI64(start, end), IntervalElement::I64) => {
            if start < end {
                Ok(())
            } else {
                Err(ValidationError::EmptyIntervalLiteral { atom, field })
            }
        }
        _ => Err(ValidationError::LiteralTypeMismatch { atom, field }),
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
    /// [`Context::resolve_bivalents`].
    pub(super) fn resolved_var_type(&self, var: VarId) -> &ValueType {
        match &self.var_slots[&var] {
            TypeSlot::Mono(value_type) => value_type,
            TypeSlot::Bivalent(_) => unreachable!("resolve_bivalents ran"),
        }
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
            Term::Duration(_) => {
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
            Term::Duration(_) => {
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
                // Unreachable for a scalar field (kind is checked
                // first), kept total for the mapping.
                Err(LiteralMismatch::IntervalEmpty) => {
                    return Err(ValidationError::EmptyIntervalLiteral {
                        atom: occ_idx,
                        field,
                    });
                }
            },
        }
        Ok(())
    }

    // --- comparisons ------------------------------------------------------

    pub(super) fn check_comparisons(&mut self, rule: &LoweredRule) -> Result<(), ValidationError> {
        self.comparison_shapes(rule)?;
        self.propagate_comparison_anchors(rule)?;
        self.resolve_bivalents();
        self.comparison_types(rule)?;
        // A param with no anchor is unwritable by construction: every
        // param position is itself an anchor (a field binding types it
        // immediately; a comparison against a variable types it via the
        // variable; param-only comparisons are already
        // `ConstantComparison`) — the roster item is discharged by
        // representation, not by a check. The two whole-program param
        // rules — mask-vs-value conflicts and id density — are checked
        // after every rule contributed (params are query-global;
        // `validate::ParamTables`).
        Ok(())
    }

    /// Shape rules that need no types: self-comparisons, constant
    /// comparisons (no variable side), comparison-only variables, param
    /// roles, and the `ParamSet`-only-under-`Eq` rule.
    fn comparison_shapes(&mut self, rule: &LoweredRule) -> Result<(), ValidationError> {
        for (index, Comparison { op, lhs, rhs }) in rule.predicates.iter().enumerate() {
            // The Allen mask position, both vacuity rules for literals
            // (∅ = "never": write no query; full = "always": write no
            // predicate) and the roster registration for params (their
            // vacuity is checked at bind, where the value exists).
            if let CmpOp::Allen { mask } = op {
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
                        self.note_param_kind(*param, ParamKind::Scalar)?;
                        self.mask_params.insert(*param);
                    }
                }
            }
            // A comparison of a variable with itself is constant-valued —
            // the "write the query you mean" rule applies exactly as it
            // does to literal-vs-literal.
            if let (Term::Var(l), Term::Var(r)) = (lhs, rhs)
                && l == r
            {
                return Err(ValidationError::SelfComparison { index });
            }
            // The measure's comparison discipline (20-query-ir, § the
            // measure): one `Duration` side at most, and only under the
            // order operators — every other operator is a typed
            // rejection here, so the typed phase below never sees a
            // measure outside `check_order`.
            match (lhs, rhs) {
                (Term::Duration(_), Term::Duration(_)) => {
                    return Err(ValidationError::DurationBothSides { index });
                }
                (Term::Duration(_), _) | (_, Term::Duration(_))
                    if !matches!(op, CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge) =>
                {
                    return Err(ValidationError::DurationComparisonOperator { index });
                }
                _ => {}
            }
            // A comparison with no variable side is a constant comparison —
            // write the query you mean (a measure varies with its bound
            // variable, so it is a variable side).
            let varies = |term: &Term| matches!(term, Term::Var(_) | Term::Duration(_));
            if !varies(lhs) && !varies(rhs) {
                return Err(ValidationError::ConstantComparison { index });
            }
            for term in [lhs, rhs] {
                match term {
                    Term::Var(var) | Term::Duration(var) => {
                        if !self.var_slots.contains_key(var) {
                            return Err(ValidationError::ComparisonOnlyVariable { var: *var });
                        }
                    }
                    Term::Param(param) => self.note_param_kind(*param, ParamKind::Scalar)?,
                    Term::ParamSet(param) => {
                        self.note_param_kind(*param, ParamKind::Set)?;
                        if !matches!(op, CmpOp::Eq) {
                            return Err(ValidationError::ParamSetComparison { index });
                        }
                    }
                    Term::Literal(_) => {}
                }
            }
        }
        Ok(())
    }

    /// Monovalent-anchor propagation: under the same-type operators, a
    /// side of known type names the other side's type — collapsing a
    /// bivalent variable and anchoring an unanchored param. Runs to a
    /// fixpoint so comparison order cannot matter. Incompatibilities are
    /// left standing (never overwritten): `comparison_types` diagnoses
    /// them against final types. `Contains` propagates nothing — its
    /// right side is legally either reading of the left (the predicate
    /// form of the membership rule), so neither side names the other.
    fn propagate_comparison_anchors(&mut self, rule: &LoweredRule) -> Result<(), ValidationError> {
        loop {
            let mut changed = false;
            for Comparison { op, lhs, rhs } in &rule.predicates {
                if matches!(op, CmpOp::Contains) {
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
            Term::Duration(_) => Some(ValueType::U64),
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
            Term::ParamSet(_) | Term::Literal(_) | Term::Duration(_) => false,
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
    ///    comparison (predicates).
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
    fn resolve_bivalents(&mut self) {
        for slot in self
            .var_slots
            .values_mut()
            .chain(self.param_slots.values_mut())
        {
            if let TypeSlot::Bivalent(element) = *slot {
                *slot = TypeSlot::Mono(ValueType::Interval { element });
            }
        }
    }

    /// Per-operator type legality over final types, and the param
    /// anchoring those rules imply. Runs after `resolve_bivalents`: every
    /// variable slot is monovalent here.
    fn comparison_types(&mut self, rule: &LoweredRule) -> Result<(), ValidationError> {
        for (index, Comparison { op, lhs, rhs }) in rule.predicates.iter().enumerate() {
            match op {
                CmpOp::Eq | CmpOp::Ne => self.check_equality(index, lhs, rhs)?,
                CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge => {
                    self.check_order(index, lhs, rhs)?;
                }
                CmpOp::Allen { .. } => self.check_allen(index, lhs, rhs)?,
                CmpOp::Contains => self.check_contains(index, lhs, rhs)?,
            }
        }
        Ok(())
    }

    /// `Eq`/`Ne`: same structural type both sides, every type legal.
    /// `ParamSet` reaches here under `Eq` only (`comparison_shapes`) and
    /// anchors at the variable side's type — unless that type is an
    /// interval, the dedicated `IntervalParamSet` rejection.
    fn check_equality(
        &mut self,
        index: usize,
        lhs: &Term,
        rhs: &Term,
    ) -> Result<(), ValidationError> {
        let (var, other) = match (lhs, rhs) {
            (Term::Var(var), other) | (other, Term::Var(var)) => (*var, other),
            _ => unreachable!("comparison_shapes rejected constant comparisons"),
        };
        let var_type = self.resolved_var_type(var).clone();
        match other {
            Term::Var(other_var) => {
                if *self.resolved_var_type(*other_var) != var_type {
                    return Err(ValidationError::IllegalComparison { index });
                }
            }
            Term::Param(param) => self.anchor_param_mono(*param, &var_type)?,
            Term::ParamSet(param) => {
                if matches!(var_type, ValueType::Interval { .. }) {
                    return Err(ValidationError::IntervalParamSet { param: *param });
                }
                self.anchor_param_mono(*param, &var_type)?;
            }
            Term::Literal(value) => self.check_literal_against(index, value, &var_type)?,
            Term::Duration(_) => {
                unreachable!("comparison_shapes admits measures under order operators only")
            }
        }
        Ok(())
    }

    /// `Lt`/`Le`/`Gt`/`Ge`: U64/U64 and I64/I64 only — plus the measure
    /// side, `Duration(v)`, whose variable must have resolved to an
    /// interval and whose value side is u64 (20-query-ir, § the
    /// measure). An interval operand gets the dedicated diagnostic — the
    /// predictable mistake gets the good error.
    fn check_order(&mut self, index: usize, lhs: &Term, rhs: &Term) -> Result<(), ValidationError> {
        for term in [lhs, rhs] {
            match self.term_mono_type(term) {
                Some(ValueType::Interval { .. }) => {
                    return Err(ValidationError::OrderComparisonOnInterval { index });
                }
                // The order-on-bytes refusal: a digest's lexicographic
                // order is an encoding artifact — identity only
                // (docs/architecture/10-data-model.md).
                Some(ValueType::FixedBytes { .. }) => {
                    return Err(ValidationError::OrderComparisonOnFixedBytes { index });
                }
                _ => {}
            }
        }
        // The measure side, if any (comparison_shapes admitted at most
        // one): the measured variable must be an interval; the other
        // side checks against u64 exactly as a u64 variable side would.
        if let (Term::Duration(var), other) | (other, Term::Duration(var)) = (lhs, rhs) {
            if !matches!(self.resolved_var_type(*var), ValueType::Interval { .. }) {
                return Err(ValidationError::DurationOverNonInterval { var: *var });
            }
            return self.check_order_side(index, other, &ValueType::U64);
        }
        let (var, other) = match (lhs, rhs) {
            (Term::Var(var), other) | (other, Term::Var(var)) => (*var, other),
            _ => unreachable!("comparison_shapes rejected constant comparisons"),
        };
        let var_type = self.resolved_var_type(var).clone();
        if !matches!(var_type, ValueType::U64 | ValueType::I64) {
            return Err(ValidationError::IllegalComparison { index });
        }
        self.check_order_side(index, other, &var_type)
    }

    /// One order comparison's non-anchoring side against the anchoring
    /// side's resolved type (a variable's, or u64 for a measure).
    fn check_order_side(
        &mut self,
        index: usize,
        other: &Term,
        expected: &ValueType,
    ) -> Result<(), ValidationError> {
        match other {
            Term::Var(other_var) => {
                if self.resolved_var_type(*other_var) != expected {
                    return Err(ValidationError::IllegalComparison { index });
                }
            }
            Term::Param(param) => self.anchor_param_mono(*param, expected)?,
            Term::ParamSet(_) => unreachable!("comparison_shapes rejected sets under order ops"),
            Term::Literal(value) => self.check_literal_against(index, value, expected)?,
            Term::Duration(_) => {
                unreachable!("comparison_shapes rejected two-measure comparisons")
            }
        }
        Ok(())
    }

    /// `Allen { mask }`: two interval terms of one element type — the one
    /// interval-pair comparison (the mask itself was checked in
    /// `comparison_shapes`; params get the vacuity rules at bind).
    fn check_allen(&mut self, index: usize, lhs: &Term, rhs: &Term) -> Result<(), ValidationError> {
        let (var, other) = match (lhs, rhs) {
            (Term::Var(var), other) | (other, Term::Var(var)) => (*var, other),
            _ => unreachable!("comparison_shapes rejected constant comparisons"),
        };
        let var_type = self.resolved_var_type(var).clone();
        if !matches!(var_type, ValueType::Interval { .. }) {
            return Err(ValidationError::IllegalComparison { index });
        }
        match other {
            Term::Var(other_var) => {
                if *self.resolved_var_type(*other_var) != var_type {
                    return Err(ValidationError::IllegalComparison { index });
                }
            }
            Term::Param(param) => self.anchor_param_mono(*param, &var_type)?,
            Term::ParamSet(_) => unreachable!("comparison_shapes rejected sets under Allen"),
            Term::Literal(value) => self.check_literal_against(index, value, &var_type)?,
            Term::Duration(_) => {
                unreachable!("comparison_shapes admits measures under order operators only")
            }
        }
        Ok(())
    }

    /// `Contains`: point membership as a predicate — an interval left
    /// side, an **element-typed** right side (the predicate form of the
    /// membership binding rule, for terms already bound elsewhere). The
    /// interval⊇interval form is gone: that predicate is `Allen(COVERS)`,
    /// and an interval-typed right side is an illegal comparison.
    fn check_contains(
        &mut self,
        index: usize,
        lhs: &Term,
        rhs: &Term,
    ) -> Result<(), ValidationError> {
        // The element domain comes from the interval side; every shape
        // with a variable somewhere is covered (constant comparisons are
        // already rejected).
        let element = match lhs {
            Term::Var(var) => match self.resolved_var_type(*var) {
                ValueType::Interval { element } => *element,
                _ => return Err(ValidationError::IllegalComparison { index }),
            },
            Term::Param(param) => {
                // The right side is a variable (constant comparisons are
                // gone), and it is the *point*: its element type names the
                // param's interval domain.
                let Term::Var(rhs_var) = rhs else {
                    unreachable!("comparison_shapes rejected constant comparisons")
                };
                let element = match self.resolved_var_type(*rhs_var) {
                    ValueType::U64 => IntervalElement::U64,
                    ValueType::I64 => IntervalElement::I64,
                    _ => return Err(ValidationError::IllegalComparison { index }),
                };
                return self.anchor_param_mono(*param, &ValueType::Interval { element });
            }
            Term::Literal(value) => {
                let Some(ValueType::Interval { element }) = literal_anchor_type(value) else {
                    return Err(ValidationError::IllegalComparison { index });
                };
                if literal_matches(value, &ValueType::Interval { element }).is_err() {
                    return Err(ValidationError::ComparisonEmptyIntervalLiteral { index });
                }
                element
            }
            Term::ParamSet(_) => {
                unreachable!("comparison_shapes rejected sets under Contains")
            }
            Term::Duration(_) => {
                unreachable!("comparison_shapes admits measures under order operators only")
            }
        };
        match rhs {
            Term::Var(var) => {
                if *self.resolved_var_type(*var) != element_type(element) {
                    return Err(ValidationError::IllegalComparison { index });
                }
            }
            Term::Param(param) => {
                // A `Contains` right side is a point at an interval
                // position: the ceiling rule applies at bind, where the
                // value exists (the point-domain law).
                self.interval_position_params.insert(*param);
                self.anchor_param_mono(*param, &element_type(element))?;
            }
            Term::Literal(value) => match (value, element) {
                (Value::U64(_), IntervalElement::U64) | (Value::I64(_), IntervalElement::I64) => {
                    // The point domain is `MIN ..= MAX−1`.
                    if at_domain_ceiling(value) {
                        return Err(ValidationError::ComparisonPointLiteralAtCeiling { index });
                    }
                }
                _ => return Err(ValidationError::IllegalComparison { index }),
            },
            Term::ParamSet(_) => {
                unreachable!("comparison_shapes rejected sets under Contains")
            }
            Term::Duration(_) => {
                unreachable!("comparison_shapes admits measures under order operators only")
            }
        }
        Ok(())
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
            Err(LiteralMismatch::IntervalEmpty) => {
                Err(ValidationError::ComparisonEmptyIntervalLiteral { index })
            }
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
        for (var, slot) in &self.var_slots {
            let TypeSlot::Mono(value_type) = slot else {
                unreachable!("resolve_bivalents ran")
            };
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
