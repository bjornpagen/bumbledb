use super::{
    Context, InteriorSignatures, NonEmpty, ParamKind, RuleTyping, Signature, TypeSlot,
    ValidatedBaseArm, ValidatedInterior, ValidatedMain, ValidatedQuery, ValidatedRec,
    ValidatedRecArm,
};
use crate::error::{Exceeded, FindIndex, Mismatch, RuleIndex, ValidationError};
use crate::ir::normalize::{LoweredRule, collapse, disjunct_count, distribute, nesting_depth};
use crate::ir::{
    FindTerm, InteriorId, MAX_CONDITION_DEPTH, MAX_RULES, ParamId, Query, Rec, RecRule, RecStep,
    VarId,
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
///
/// # Panics
///
/// Never: interior ids are `u32`-checked above before the `expect`.
pub fn validate(schema: &Schema, query: &Query) -> Result<ValidatedQuery, ValidationError> {
    match &query.rec {
        None => validate_cq(schema, &query.interiors, &query.head, &query.rules),
        Some(rec) => validate_reach(schema, &query.interiors, rec, &query.head, &query.rules),
    }
}

fn overflow(derived: usize) -> Result<(), ValidationError> {
    if u32::try_from(derived).is_err() {
        Err(ValidationError::InteriorIdOverflow { count: derived })
    } else {
        Ok(())
    }
}

fn validate_cq(
    schema: &Schema,
    interiors: &[crate::ir::Interior],
    head: &[crate::ir::HeadTerm],
    rules: &[crate::ir::Rule],
) -> Result<ValidatedQuery, ValidationError> {
    let derived = interiors.len();
    overflow(derived)?;
    let mut params = ParamTables::default();
    let (sealed, interiors_out, mut rule_count) = seal_interiors(
        schema,
        interiors,
        |sealed, id| InteriorSignatures::cq(sealed, Some(id), derived),
        &mut params,
    )?;
    let main = type_main(
        schema,
        head,
        rules,
        &InteriorSignatures::cq(&sealed, None, derived),
        &mut params,
        &mut rule_count,
    )?;
    finish_cq(params, interiors_out, main, rule_count)
}

fn validate_reach(
    schema: &Schema,
    interiors: &[crate::ir::Interior],
    rec: &Rec,
    head: &[crate::ir::HeadTerm],
    rules: &[crate::ir::Rule],
) -> Result<ValidatedQuery, ValidationError> {
    let derived = interiors.len() + 1;
    overflow(derived)?;
    let mut params = ParamTables::default();
    let (sealed, interiors_out, mut rule_count) = seal_interiors(
        schema,
        interiors,
        |sealed, id| InteriorSignatures::reach_open(sealed, Some(id), derived),
        &mut params,
    )?;
    let id = InteriorId(u32::try_from(interiors.len()).expect("derived count fits u32"));
    refuse_self_in_base(rec, id)?;
    let rec_head = rec.head();
    let (base, rec_low) = lower_rec_pool(rec, id)?;
    let base_typing = type_rules(
        schema,
        &InteriorSignatures::reach_open(&sealed, None, derived),
        &rec_head,
        &base,
        &mut params,
        true,
    )?;
    rule_count += base_typing.len() as u64;
    let rec_signature = super::Signature::derive(&base[0], &base_typing[0]);
    let rec_typing = type_rules(
        schema,
        &InteriorSignatures::reach_sealed(&sealed, &rec_signature, derived),
        &rec_head,
        &rec_low,
        &mut params,
        true,
    )?;
    let base_row = input_row(&base[0], &base_typing[0]);
    for (rule_idx, (rule, typing)) in rec_low.iter().zip(&rec_typing).enumerate() {
        let row = input_row(rule, typing);
        if let Some(position) = (0..row.len()).find(|&i| row[i] != base_row[i]) {
            return Err(ValidationError::HeadTypeMismatch {
                rule: RuleIndex(rule_idx),
                position: FindIndex(position),
            });
        }
    }
    rule_count += rec_typing.len() as u64;
    let base_arms = NonEmpty::from_vec(
        base.into_iter()
            .zip(base_typing)
            .map(|(rule, typing)| ValidatedBaseArm { rule, typing })
            .collect(),
    )
    .expect("roster/lower refused empty base");
    let rec_arms = NonEmpty::from_vec(
        rec_low
            .into_iter()
            .zip(rec_typing)
            .map(|(rule, typing)| ValidatedRecArm {
                self_occ: crate::ir::normalize::OccId(0),
                rule,
                typing,
            })
            .collect(),
    )
    .expect("roster/lower refused empty rec");
    let rec_out = ValidatedRec {
        base: base_arms,
        rec: rec_arms,
        signature: rec_signature,
    };
    let main = type_main(
        schema,
        head,
        rules,
        &InteriorSignatures::reach_sealed(&sealed, rec_out.signature(), derived),
        &mut params,
        &mut rule_count,
    )?;
    finish_reach(params, interiors_out, rec_out, main, rule_count)
}

/// Seals interiors in declaration order. `sigs` builds the typing
/// surface for interior *id* against already-sealed signatures.
fn seal_interiors(
    schema: &Schema,
    interiors: &[crate::ir::Interior],
    sigs: impl for<'a> Fn(&'a [Signature], InteriorId) -> InteriorSignatures<'a>,
    params: &mut ParamTables,
) -> Result<(Vec<Signature>, Vec<ValidatedInterior>, u64), ValidationError> {
    let mut sealed: Vec<Signature> = Vec::with_capacity(interiors.len());
    let mut interiors_out = Vec::with_capacity(interiors.len());
    let mut rule_count = 0u64;
    let mut seal_span = crate::obs::span(crate::obs::names::VALIDATE_SEAL);
    for (index, interior) in interiors.iter().enumerate() {
        let id = InteriorId(u32::try_from(index).expect("derived count fits u32"));
        if interior.rules.is_empty() {
            return Err(ValidationError::EmptyInterior { interior: id });
        }
        let as_rules: Vec<crate::ir::Rule> = interior
            .rules
            .iter()
            .map(crate::ir::ProjectionRule::to_rule)
            .collect();
        let head = interior.head();
        let lowered = lower_rules(
            &head,
            &as_rules,
            ValidationError::EmptyInterior { interior: id },
            false,
        )?;
        let typings = type_rules(schema, &sigs(&sealed, id), &head, &lowered, params, false)?;
        rule_count += typings.len() as u64;
        let signature = super::Signature::derive(&lowered[0], &typings[0]);
        sealed.push(signature.clone());
        interiors_out.push(ValidatedInterior {
            lowered,
            signature,
            rules: typings,
        });
    }
    seal_span.set_pair(interiors.len() as u64, sealed.len() as u64);
    seal_span.end();
    Ok((sealed, interiors_out, rule_count))
}

fn type_main(
    schema: &Schema,
    head: &[crate::ir::HeadTerm],
    rules: &[crate::ir::Rule],
    sigs: &InteriorSignatures<'_>,
    params: &mut ParamTables,
    rule_count: &mut u64,
) -> Result<ValidatedMain, ValidationError> {
    let lowered = lower_rules(head, rules, ValidationError::EmptyRuleSet, true)?;
    let typings = type_rules(schema, sigs, head, &lowered, params, false)?;
    *rule_count += typings.len() as u64;
    let signature = super::Signature::derive(&lowered[0], &typings[0]);
    Ok(ValidatedMain {
        lowered,
        signature,
        rules: typings,
    })
}

fn finish_cq(
    params: ParamTables,
    interiors: Vec<ValidatedInterior>,
    main: ValidatedMain,
    rule_count: u64,
) -> Result<ValidatedQuery, ValidationError> {
    let mut rules_span = crate::obs::span(crate::obs::names::VALIDATE_RULES);
    rules_span.set_count(rule_count);
    rules_span.end();
    params.check_masks_and_density()?;
    let set_params = set_params_of(&params);
    Ok(ValidatedQuery {
        interiors,
        main,
        param_types: params.param_types,
        set_params,
        point_params: params.point_params,
        rec: None,
    })
}

fn finish_reach(
    params: ParamTables,
    interiors: Vec<ValidatedInterior>,
    rec: ValidatedRec,
    main: ValidatedMain,
    rule_count: u64,
) -> Result<ValidatedQuery, ValidationError> {
    let mut rules_span = crate::obs::span(crate::obs::names::VALIDATE_RULES);
    rules_span.set_count(rule_count);
    rules_span.end();
    params.check_masks_and_density()?;
    let set_params = set_params_of(&params);
    Ok(ValidatedQuery {
        interiors,
        rec: Some(rec),
        main,
        param_types: params.param_types,
        set_params,
        point_params: params.point_params,
    })
}

fn set_params_of(params: &ParamTables) -> BTreeSet<ParamId> {
    params
        .param_kinds
        .iter()
        .filter_map(|(param, kind)| matches!(kind, ParamKind::Set).then_some(*param))
        .collect()
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
    rec_body: bool,
) -> Result<Vec<RuleTyping>, ValidationError> {
    let mut pinned_row: Vec<ValueType> = Vec::new();
    let mut rules = Vec::with_capacity(lowered.len());
    for (rule_idx, rule) in lowered.iter().enumerate() {
        check_head_alignment(head, rule, rule_idx)?;
        let (typing, ctx) = validate_rule(schema, sigs, rule, rec_body)?;
        let row = input_row(rule, &typing);
        if rule_idx == 0 {
            pinned_row = row;
        } else if let Some(position) = (0..row.len()).find(|i| row[*i] != pinned_row[*i]) {
            return Err(ValidationError::HeadTypeMismatch {
                rule: RuleIndex(rule_idx),
                position: FindIndex(position),
            });
        }
        params.unify(ctx)?;
        rules.push(typing);
    }
    Ok(rules)
}

/// Bound-var law is in the type: interior/rec finds are [`VarId`].
/// A `RecRule` atom that names the rec is still a positional coincidence
/// ([`InteriorId`] is `interiors.len()`), parsed here once. A `RecStep`'s
/// `self_bindings` is the unique self-atom; a leftover Interior(rec)
/// among `atoms` is a second self-read.
fn refuse_self_in_base(rec: &Rec, rec_id: InteriorId) -> Result<(), ValidationError> {
    let is_self = |atom: &crate::ir::Atom| atom.source.interior() == Some(rec_id);
    if rec.base.iter().any(|rule| rule.atoms.iter().any(is_self)) {
        return Err(ValidationError::SelfInBase);
    }
    if rec.rec.iter().any(|step| step.atoms.iter().any(is_self)) {
        return Err(ValidationError::NonlinearRecArm);
    }
    Ok(())
}

/// Rec pool lowering: one [`MAX_RULES`] on `base.len() + rec.len()` and
/// on DNF width of both lists together — not 16+16. Step arms reconstruct
/// the unique self-atom as the first positive atom, so `self_occ` is 0.
fn lower_rec_pool(
    rec: &Rec,
    rec_id: InteriorId,
) -> Result<(Vec<LoweredRule>, Vec<LoweredRule>), ValidationError> {
    let count = rec.base.len() + rec.rec.len();
    if count > MAX_RULES {
        return Err(ValidationError::TooManyRules { count });
    }
    let head = rec.head();
    if head.is_empty() {
        return Err(ValidationError::EmptyFinds);
    }
    let base_rules: Vec<crate::ir::Rule> = rec.base.iter().map(RecRule::to_rule).collect();
    let rec_rules: Vec<crate::ir::Rule> = rec
        .rec
        .iter()
        .map(|step| RecStep::to_rule(step, rec_id))
        .collect();
    for (rule_idx, rule) in base_rules.iter().chain(&rec_rules).enumerate() {
        let depth = nesting_depth(&rule.conditions);
        if depth > MAX_CONDITION_DEPTH {
            return Err(ValidationError::ConditionNestingTooDeep {
                rule: RuleIndex(rule_idx),
                exceeded: Exceeded {
                    observed: depth,
                    ceiling: MAX_CONDITION_DEPTH,
                },
            });
        }
    }
    let produced = base_rules
        .iter()
        .chain(&rec_rules)
        .map(disjunct_count)
        .fold(0, usize::saturating_add);
    if produced > MAX_RULES {
        return Err(ValidationError::DnfExceedsRules {
            exceeded: Exceeded {
                observed: produced,
                ceiling: MAX_RULES,
            },
        });
    }
    let base = distribute_list(&base_rules);
    let rec_low = distribute_list(&rec_rules);
    // Written-empty arms are unrepresentable (`NonEmpty`). A nonempty
    // written arm can still DNF to nothing — `Or([])` is false — and
    // that is a distinct fact, observed here.
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
    let mut span = crate::obs::span(crate::obs::names::VALIDATE_LOWER);
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
                rule: RuleIndex(rule_idx),
                exceeded: Exceeded {
                    observed: depth,
                    ceiling: MAX_CONDITION_DEPTH,
                },
            });
        }
    }

    let produced = rules
        .iter()
        .map(disjunct_count)
        .fold(0, usize::saturating_add);
    if produced > MAX_RULES {
        return Err(ValidationError::DnfExceedsRules {
            exceeded: Exceeded {
                observed: produced,
                ceiling: MAX_RULES,
            },
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
    span.set_count(lowered.len() as u64);
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
            rule: RuleIndex(rule_idx),
            mismatch: Mismatch {
                witnessed: rule.finds.len(),
                required: head.len(),
            },
        });
    }
    for (position, (term, head_term)) in rule.finds.iter().zip(head).enumerate() {
        if term.head_term() != *head_term {
            return Err(ValidationError::HeadAggregateMismatch {
                rule: RuleIndex(rule_idx),
                position: FindIndex(position),
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
    _rec_body: bool,
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
            FindTerm::Var(var) => Some(*var),
            FindTerm::Count | FindTerm::Aggregate { .. } | FindTerm::Pack { .. } => None,
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
/// Alignment-only — the signature is [`super::Signature::derive`].
fn input_row(rule: &LoweredRule, typing: &RuleTyping) -> Vec<ValueType> {
    let var_type = |var: &VarId| typing.var_types.get(var).copied().expect("typed var");
    rule.finds
        .iter()
        .map(|term| match term {
            FindTerm::Var(var) => var_type(var),
            FindTerm::Count => ValueType::U64,
            FindTerm::Aggregate { over, .. } | FindTerm::Pack { over } => var_type(over),
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
