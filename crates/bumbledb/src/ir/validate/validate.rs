use super::{
    Context, InteriorSignatures, ParamKind, Predicate, RuleTyping, TypeSlot, ValidatedInterior,
    ValidatedMain, ValidatedQuery, ValidatedRec,
};
use crate::error::ValidationError;
use crate::ir::normalize::{LoweredRule, collapse, disjunct_count, distribute, nesting_depth};
use crate::ir::{
    AggOp, ConditionTree, FindTerm, InteriorId, MAX_CONDITION_DEPTH, MAX_RULES, ParamId, Query,
    Rec, Term, VarId,
};
use crate::schema::Schema;
use bumbledb_theory::schema::ValueType;
use std::collections::{BTreeMap, BTreeSet};

/// Validates a query against the schema, yielding the sealed witness.
///
/// The roster, in order (`docs/architecture/20-query-ir.md`): derived-table
/// id-width; then each interior, the rec pool, and main independently
/// through the query-shape checks (empty, [`MAX_RULES`], nesting, DNF,
/// head alignment) and the per-rule roster, sealing interiors in
/// declaration order and the rec from base then rec arms; then the rec
/// structural roster; then query-global param unification. First
/// failure wins.
///
/// # Errors
///
/// A distinct [`ValidationError`] per roster item; see the module docs.
/// Rule-local payloads name positions inside the first failing
/// **lowered** rule of the first failing rule-list.
pub fn validate(schema: &Schema, query: &Query) -> Result<ValidatedQuery, ValidationError> {
    let derived = query.interiors.len() + usize::from(query.rec.is_some());
    if u32::try_from(derived).is_err() {
        return Err(ValidationError::InteriorIdOverflow { count: derived });
    }

    let mut params = ParamTables::default();
    let mut arities: Vec<usize> = Vec::with_capacity(derived);
    let mut sealed: Vec<Option<Predicate>> = Vec::with_capacity(derived);
    let mut interiors_out = Vec::with_capacity(query.interiors.len());
    let mut rule_count = 0u64;

    let mut seal_span = crate::obs::span(
        crate::obs::names::VALIDATE_SEAL,
        crate::obs::Category::Prepare,
    );
    for (index, interior) in query.interiors.iter().enumerate() {
        let id = InteriorId(u32::try_from(index).expect("derived count fits u32"));
        if interior.rules.is_empty() {
            return Err(ValidationError::EmptyInterior { interior: id });
        }
        let lowered = lower_rules(
            &interior.head,
            &interior.rules,
            ValidationError::EmptyInterior { interior: id },
            false,
        )?;
        refuse_derived_head(&interior.head, &lowered, id)?;
        let typings = type_rules(
            schema,
            &InteriorSignatures {
                arities: &arities,
                sealed: &sealed,
                reader: Some(id),
                derived_count: derived,
            },
            &interior.head,
            &lowered,
            &mut params,
        )?;
        rule_count += typings.len() as u64;
        let predicate = super::Predicate::derive(&lowered[0], &typings[0]);
        arities.push(interior.head.len());
        sealed.push(Some(predicate.clone()));
        interiors_out.push(ValidatedInterior {
            lowered,
            predicate,
            rules: typings,
        });
    }
    seal_span.set_args(query.interiors.len() as u64, sealed.len() as u64);
    seal_span.end();

    let rec_out = if let Some(rec) = &query.rec {
        let id = InteriorId(u32::try_from(query.interiors.len()).expect("derived count fits u32"));
        rec_roster(rec, id)?;
        let (base, rec_low) = lower_rec_pool(rec)?;
        refuse_derived_head(&rec.head, &base, id)?;
        refuse_derived_head(&rec.head, &rec_low, id)?;
        arities.push(rec.head.len());
        sealed.push(None);
        let base_typing = type_rules(
            schema,
            &InteriorSignatures {
                arities: &arities,
                sealed: &sealed,
                reader: None,
                derived_count: derived,
            },
            &rec.head,
            &base,
            &mut params,
        )?;
        rule_count += base_typing.len() as u64;
        let predicate = super::Predicate::derive(&base[0], &base_typing[0]);
        *sealed.last_mut().expect("rec slot pushed") = Some(predicate.clone());
        let rec_typing = type_rules(
            schema,
            &InteriorSignatures {
                arities: &arities,
                sealed: &sealed,
                reader: None,
                derived_count: derived,
            },
            &rec.head,
            &rec_low,
            &mut params,
        )?;
        let base_row = input_row(&base[0], &base_typing[0]);
        for (rule_idx, (rule, typing)) in rec_low.iter().zip(&rec_typing).enumerate() {
            let row = input_row(rule, typing);
            if let Some(position) = (0..row.len()).find(|&i| row[i] != base_row[i]) {
                return Err(ValidationError::HeadTypeMismatch {
                    rule: rule_idx,
                    position,
                });
            }
        }
        rule_count += rec_typing.len() as u64;
        measure_in_rec(rec)?;
        Some(ValidatedRec {
            base,
            rec: rec_low,
            predicate,
            base_typing,
            rec_typing,
        })
    } else {
        None
    };

    let lowered = lower_rules(
        &query.head,
        &query.rules,
        ValidationError::EmptyRuleSet,
        true,
    )?;
    let typings = type_rules(
        schema,
        &InteriorSignatures {
            arities: &arities,
            sealed: &sealed,
            reader: None,
            derived_count: derived,
        },
        &query.head,
        &lowered,
        &mut params,
    )?;
    rule_count += typings.len() as u64;
    let predicate = super::Predicate::derive(&lowered[0], &typings[0]);

    let mut rules_span = crate::obs::span(
        crate::obs::names::VALIDATE_RULES,
        crate::obs::Category::Prepare,
    );
    rules_span.set_args(rule_count, 0);
    rules_span.end();

    params.check_masks_and_density()?;
    Ok(ValidatedQuery {
        interiors: interiors_out,
        rec: rec_out,
        main: ValidatedMain {
            lowered,
            predicate,
            rules: typings,
        },
        param_types: params.param_types,
        set_params: params
            .param_kinds
            .iter()
            .filter_map(|(param, kind)| matches!(kind, ParamKind::Set).then_some(*param))
            .collect(),
        point_params: params.point_params,
    })
}

/// Head-alignment, per-rule roster, and head-type agreement for one
/// already-lowered rule-list. Params unify into `params` as each rule
/// types — query-global, one pass.
fn type_rules(
    schema: &Schema,
    sigs: &InteriorSignatures<'_>,
    head: &[crate::ir::HeadTerm],
    lowered: &[LoweredRule],
    params: &mut ParamTables,
) -> Result<Vec<RuleTyping>, ValidationError> {
    let mut pinned_row: Vec<ValueType> = Vec::new();
    let mut rules = Vec::with_capacity(lowered.len());
    for (rule_idx, rule) in lowered.iter().enumerate() {
        check_head_alignment(head, rule, rule_idx)?;
        let (typing, ctx) = validate_rule(schema, sigs, rule)?;
        let row = input_row(rule, &typing);
        if rule_idx == 0 {
            pinned_row = row;
        } else if let Some(position) = (0..row.len()).find(|i| row[*i] != pinned_row[*i]) {
            return Err(ValidationError::HeadTypeMismatch {
                rule: rule_idx,
                position,
            });
        }
        params.unify(ctx)?;
        rules.push(typing);
    }
    Ok(rules)
}

/// Bound-var law on an interior or rec head: folds and measure finds
/// are [`ValidationError::AggregateInInterior`] /
/// [`ValidationError::MeasureInInterior`].
fn refuse_derived_head(
    head: &[crate::ir::HeadTerm],
    lowered: &[LoweredRule],
    interior: InteriorId,
) -> Result<(), ValidationError> {
    if head
        .iter()
        .any(|term| matches!(term, crate::ir::HeadTerm::Aggregate(_)))
    {
        return Err(ValidationError::AggregateInInterior { interior });
    }
    if lowered.iter().flat_map(|rule| &rule.finds).any(|term| {
        matches!(
            term,
            FindTerm::Measure(_) | FindTerm::AggregateMeasure { .. }
        )
    }) {
        return Err(ValidationError::MeasureInInterior { interior });
    }
    Ok(())
}

/// Rec structural roster, on the written (pre-DNF) rules, in declaration
/// order: empty lists, self in base, missing/nonlinear self, negation.
fn rec_roster(rec: &Rec, rec_id: InteriorId) -> Result<(), ValidationError> {
    if rec.base.is_empty() {
        return Err(ValidationError::EmptyRecursiveBase);
    }
    if rec.rec.is_empty() {
        return Err(ValidationError::EmptyRecursiveStep);
    }
    let is_self = |atom: &crate::ir::Atom| atom.source.interior() == Some(rec_id);
    for rule in &rec.base {
        if rule.atoms.iter().chain(&rule.negated).any(is_self) {
            return Err(ValidationError::SelfInBase);
        }
    }
    for rule in &rec.rec {
        let selves = rule.atoms.iter().filter(|atom| is_self(atom)).count();
        if selves == 0 {
            return Err(ValidationError::RecArmMissingSelf);
        }
        if selves >= 2 {
            return Err(ValidationError::NonlinearRecArm);
        }
    }
    let negated = rec
        .base
        .iter()
        .chain(&rec.rec)
        .any(|rule| !rule.negated.is_empty());
    if negated {
        return Err(ValidationError::NegationInRec);
    }
    Ok(())
}

/// Measure comparisons in rec **bodies** — per-rule may already have
/// refused a binding (`DurationInBinding`); a legal comparison shape
/// is this item.
fn measure_in_rec(rec: &Rec) -> Result<(), ValidationError> {
    let has_measure = |tree: &ConditionTree| -> bool { tree_has_measure(tree) };
    if rec
        .base
        .iter()
        .chain(&rec.rec)
        .flat_map(|rule| &rule.conditions)
        .any(has_measure)
    {
        return Err(ValidationError::MeasureInRec);
    }
    Ok(())
}

fn tree_has_measure(tree: &ConditionTree) -> bool {
    match tree {
        ConditionTree::Leaf(cmp) => {
            matches!(cmp.lhs, Term::Measure(_)) || matches!(cmp.rhs, Term::Measure(_))
        }
        ConditionTree::And(children) | ConditionTree::Or(children) => {
            children.iter().any(tree_has_measure)
        }
    }
}

/// Rec pool lowering: one [`MAX_RULES`] on `base.len() + rec.len()` and
/// on DNF width of both lists together — not 16+16.
fn lower_rec_pool(rec: &Rec) -> Result<(Vec<LoweredRule>, Vec<LoweredRule>), ValidationError> {
    let count = rec.base.len() + rec.rec.len();
    if count > MAX_RULES {
        return Err(ValidationError::TooManyRules { count });
    }
    if rec.head.is_empty() {
        return Err(ValidationError::EmptyFinds);
    }
    for (rule_idx, rule) in rec.base.iter().chain(&rec.rec).enumerate() {
        let depth = nesting_depth(&rule.conditions);
        if depth > MAX_CONDITION_DEPTH {
            return Err(ValidationError::ConditionNestingTooDeep {
                rule: rule_idx,
                depth,
                cap: MAX_CONDITION_DEPTH,
            });
        }
    }
    let produced = rec
        .base
        .iter()
        .chain(&rec.rec)
        .map(disjunct_count)
        .fold(0, usize::saturating_add);
    if produced > MAX_RULES {
        return Err(ValidationError::DnfExceedsRules {
            produced,
            cap: MAX_RULES,
        });
    }
    let base = distribute_list(&rec.base);
    let rec_low = distribute_list(&rec.rec);
    if base.is_empty() {
        return Err(ValidationError::EmptyRecursiveBase);
    }
    if rec_low.is_empty() {
        return Err(ValidationError::EmptyRecursiveStep);
    }
    Ok((base, rec_low))
}

fn distribute_list(rules: &[crate::ir::Rule]) -> Vec<LoweredRule> {
    let distributed = rules
        .iter()
        .enumerate()
        .flat_map(|(written, rule)| {
            let written = u16::try_from(written).expect("rule count capped");
            distribute(rule).into_iter().map(move |mut lowered| {
                lowered.written = Some(written);
                lowered.minted = vec![written];
                lowered
            })
        })
        .collect();
    collapse(distributed)
}

/// The query-shape half of the roster, per rule-list: empty (the
/// provided error), the rule cap, empty head, the nesting boundary
/// check, DNF distribution under its structural cap, and — on main
/// only — the Count-across-rules refusal.
fn lower_rules(
    head: &[crate::ir::HeadTerm],
    rules: &[crate::ir::Rule],
    empty: ValidationError,
    count_across: bool,
) -> Result<Vec<LoweredRule>, ValidationError> {
    let mut span = crate::obs::span(
        crate::obs::names::VALIDATE_LOWER,
        crate::obs::Category::Prepare,
    );
    if rules.is_empty() {
        return Err(empty);
    }
    if rules.len() > MAX_RULES {
        return Err(ValidationError::TooManyRules { count: rules.len() });
    }
    if head.is_empty() {
        return Err(ValidationError::EmptyFinds);
    }

    for (rule_idx, rule) in rules.iter().enumerate() {
        let depth = nesting_depth(&rule.conditions);
        if depth > MAX_CONDITION_DEPTH {
            return Err(ValidationError::ConditionNestingTooDeep {
                rule: rule_idx,
                depth,
                cap: MAX_CONDITION_DEPTH,
            });
        }
    }

    let produced = rules
        .iter()
        .map(disjunct_count)
        .fold(0, usize::saturating_add);
    if produced > MAX_RULES {
        return Err(ValidationError::DnfExceedsRules {
            produced,
            cap: MAX_RULES,
        });
    }
    let lowered = distribute_list(rules);
    if lowered.is_empty() {
        return Err(empty);
    }
    if count_across
        && lowered.len() > 1
        && dnf_derived(&lowered).is_none()
        && head.iter().any(|term| {
            matches!(
                term,
                crate::ir::HeadTerm::Aggregate(crate::ir::HeadOp::Count)
            )
        })
        && head.iter().all(|term| {
            matches!(
                term,
                crate::ir::HeadTerm::Var | crate::ir::HeadTerm::Aggregate(crate::ir::HeadOp::Count)
            )
        })
    {
        return Err(ValidationError::CountAcrossRules {
            rules: lowered.len(),
        });
    }
    span.set_args(lowered.len() as u64, 0);
    span.end();
    Ok(lowered)
}

/// The provenance judgment (ruled 2026-07-23, R2): `Some(written)` iff
/// every lowered rule carries the ONE shared written-rule index — the
/// set is DNF-derived from that rule and the union dedup re-keys on
/// the shared slot arrays. `None` is a hand-written rule set (or a
/// cross-written collapse), which keys the head projection.
pub(crate) fn dnf_derived(lowered: &[LoweredRule]) -> Option<u16> {
    let first = lowered.first()?.written?;
    lowered
        .iter()
        .all(|rule| rule.written == Some(first))
        .then_some(first)
}

/// Head alignment, the shape half: arity, then var-vs-aggregate-op kind
/// position by position (types are checked against the pinned row after
/// the rule's own typing fixpoint resolves them).
fn check_head_alignment(
    head: &[crate::ir::HeadTerm],
    rule: &LoweredRule,
    rule_idx: usize,
) -> Result<(), ValidationError> {
    if rule.finds.len() != head.len() {
        return Err(ValidationError::HeadArityMismatch {
            rule: rule_idx,
            expected: head.len(),
            found: rule.finds.len(),
        });
    }
    for (position, (term, head_term)) in rule.finds.iter().zip(head).enumerate() {
        if term.head_term() != *head_term {
            return Err(ValidationError::HeadAggregateMismatch {
                rule: rule_idx,
                position,
            });
        }
    }
    Ok(())
}

/// The per-rule roster — exactly the conjunctive query's checks, over one
/// rule's own variable scope and its own bivalent-anchor typing fixpoint;
/// `interiors` is the target-signature surface `Interior` anchors resolve
/// against.
fn validate_rule(
    schema: &Schema,
    interiors: &InteriorSignatures<'_>,
    rule: &LoweredRule,
) -> Result<(RuleTyping, Context), ValidationError> {
    if rule.atoms.is_empty() {
        return Err(ValidationError::NoPositiveAtoms);
    }
    let occurrences = rule.atoms.len() + rule.negated.len();
    if occurrences > crate::plan::planner::MAX_OCCURRENCES {
        return Err(ValidationError::TooManyAtoms { count: occurrences });
    }
    for (index, term) in rule.finds.iter().enumerate() {
        if rule.finds[..index].contains(term) {
            return Err(ValidationError::DuplicateFindTerm { index });
        }
    }

    let mut ctx = Context::default();
    ctx.check_atoms(schema, interiors, rule)?;
    let classified = ctx.check_comparisons(rule)?;
    ctx.check_membership_domains()?;
    let group_key: BTreeSet<VarId> = rule
        .finds
        .iter()
        .filter_map(|term| match term {
            FindTerm::Var(var) | FindTerm::Measure(var) => Some(*var),
            FindTerm::Aggregate { .. } | FindTerm::AggregateMeasure { .. } => None,
        })
        .collect();
    ctx.check_finds(rule, &group_key)?;
    if ctx.var_types.len() > crate::plan::planner::MAX_DISTINCT_VARS {
        return Err(ValidationError::TooManyVariables {
            count: ctx.var_types.len(),
        });
    }

    let var_types = ctx.var_types.clone();
    let closed_vars = ctx.closed_vars.clone();
    Ok((
        RuleTyping {
            var_types,
            group_key,
            classified,
            closed_vars,
        },
        ctx,
    ))
}

/// One rule's positional INPUT contribution to the alignment check: a
/// variable position carries the variable's type; an aggregate position
/// its fold input type (the nullary `Count` is `U64`).
/// Alignment-only — the signature is [`super::Predicate::derive`].
fn input_row(rule: &LoweredRule, typing: &RuleTyping) -> Vec<ValueType> {
    let var_type = |var: &VarId| typing.var_types[var].clone();
    rule.finds
        .iter()
        .map(|term| match term {
            FindTerm::Var(var) => var_type(var),
            FindTerm::Measure(_) | FindTerm::AggregateMeasure { .. } => ValueType::U64,
            FindTerm::Aggregate { op, over } => match op {
                AggOp::Count => ValueType::U64,
                AggOp::Sum | AggOp::Min | AggOp::Max | AggOp::Pack => {
                    var_type(&over.expect("validated: only Count is nullary"))
                }
            },
        })
        .collect()
}

/// The query-global param tables, unified across the rules' independent
/// typing fixpoints: one binding surface, so every rule's resolution of a
/// param must agree — in type, in scalar-vs-set role, and in
/// value-vs-mask role.
#[derive(Default)]
struct ParamTables {
    param_types: BTreeMap<ParamId, ValueType>,
    param_kinds: BTreeMap<ParamId, ParamKind>,
    point_params: BTreeSet<ParamId>,
}

impl ParamTables {
    /// Absorbs one rule's resolved param state, diagnosing cross-rule
    /// disagreements with the same errors the per-rule checks use.
    fn unify(&mut self, ctx: Context) -> Result<(), ValidationError> {
        for param in &ctx.interval_position_params {
            if matches!(
                ctx.param_slots.get(param),
                Some(TypeSlot::Mono(ValueType::U64 | ValueType::I64))
            ) {
                self.point_params.insert(*param);
            }
        }
        for (param, slot) in ctx.param_slots {
            let value_type = match slot {
                TypeSlot::Mono(value_type) => value_type,
                TypeSlot::Bivalent { .. } => unreachable!("resolve_bivalents ran"),
            };
            match self.param_types.get(&param) {
                Some(existing) if *existing != value_type => {
                    return Err(ValidationError::ParamTypeConflict { param });
                }
                Some(_) => {}
                None => {
                    self.param_types.insert(param, value_type);
                }
            }
        }
        for (param, kind) in ctx.param_kinds {
            match self.param_kinds.get(&param) {
                Some(existing) if *existing != kind => {
                    return Err(ValidationError::ParamScalarAndSet { param });
                }
                Some(_) => {}
                None => {
                    self.param_kinds.insert(param, kind);
                }
            }
        }
        Ok(())
    }

    /// Param id density — jointly across all rules (a gap would be a
    /// positional slot at execution whose supplied value is never
    /// type-checked).
    fn check_masks_and_density(&self) -> Result<(), ValidationError> {
        for (position, param) in self.param_kinds.keys().enumerate() {
            if usize::from(param.0) != position {
                return Err(ValidationError::ParamIdGap {
                    param: ParamId(u16::try_from(position).expect("param ids fit u16")),
                });
            }
        }
        Ok(())
    }
}
