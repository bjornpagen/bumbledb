use super::{Context, ParamKind, RuleTyping, TypeSlot, ValidatedQuery};
use crate::error::ValidationError;
use crate::ir::normalize::{LoweredRule, collapse, disjunct_count, distribute, nesting_depth};
use crate::ir::{AggOp, FindTerm, MAX_CONDITION_DEPTH, MAX_RULES, ParamId, Query, VarId};
use crate::schema::{Schema, ValueType};
use std::collections::{BTreeMap, BTreeSet};

/// Validates a query against the schema, yielding the sealed witness.
///
/// The program shape first (empty rule set, the rule cap, empty head);
/// then **DNF distribution** — each rule's condition trees distribute
/// and each disjunct becomes a rule, with the blowup capped at
/// [`MAX_RULES`] on the structural term count (before materializing)
/// and duplicates collapsed by normalized-form equality; then each
/// lowered rule under the per-rule roster with its own typing fixpoint
/// — a rule validates exactly as a conjunctive query did — and finally
/// the query-global param unification (params are one binding surface
/// across rules; variables never cross them).
///
/// Duplicate and even statically contradictory conditions (`x < 5,
/// x > 9`) are accepted deliberately: the semantics are exact (an empty
/// result), and the "write the query you mean" roster rejects only
/// shapes with no meaning at all (constant and self comparisons) — it
/// does not extend to statically false conjunctions. The empty
/// disjunction (`Or([])`) rides the same ruling: it is constant false,
/// its rule lowers to zero rules, and only a program whose *every* rule
/// vanishes is rejected — as the empty union it now is.
///
/// # Errors
///
/// A distinct [`ValidationError`] per roster item; see the module docs.
/// Rule-local payloads name positions inside the first failing
/// **lowered** rule.
pub fn validate(schema: &Schema, query: &Query) -> Result<ValidatedQuery, ValidationError> {
    if query.rules.is_empty() {
        return Err(ValidationError::EmptyRuleSet);
    }
    if query.rules.len() > MAX_RULES {
        return Err(ValidationError::TooManyRules {
            count: query.rules.len(),
        });
    }
    if query.head.is_empty() {
        return Err(ValidationError::EmptyFinds);
    }

    // The nesting boundary check runs before ANY recursive tree walk
    // (the trust-boundary law: `disjunct_count` and `distribute` recurse
    // by depth, so a hostile depth must be a typed rejection here, judged
    // by the iterative `nesting_depth`, never a stack exhaustion there).
    for (rule_idx, rule) in query.rules.iter().enumerate() {
        let depth = nesting_depth(&rule.conditions);
        if depth > MAX_CONDITION_DEPTH {
            return Err(ValidationError::ConditionNestingTooDeep {
                rule: rule_idx,
                depth,
                cap: MAX_CONDITION_DEPTH,
            });
        }
    }

    // DNF distribution: the cap is judged on the structural term count —
    // no disjunct of an exponential case is ever materialized — and the
    // distributed program collapses duplicates (set semantics at the
    // representation level).
    let produced = query
        .rules
        .iter()
        .map(disjunct_count)
        .fold(0, usize::saturating_add);
    if produced > MAX_RULES {
        return Err(ValidationError::DnfExceedsRules {
            produced,
            cap: MAX_RULES,
        });
    }
    let lowered = collapse(query.rules.iter().flat_map(distribute).collect());
    if lowered.is_empty() {
        // Every rule's disjunction was empty: the program lowered to the
        // empty union — no query.
        return Err(ValidationError::EmptyRuleSet);
    }
    // Arg-restriction across rules is undefined — the restriction key is
    // a rule-scoped variable outside the head's vocabulary, and rules
    // need not even agree on its type — so it refuses at the boundary,
    // judged on the LOWERED rule count (a DNF blowup of one Arg rule
    // refuses too). Modeling answer: one Arg query per disjunct,
    // host-merged (20-query-ir § aggregation).
    if lowered.len() > 1
        && query.head.iter().any(|term| {
            matches!(
                term,
                crate::ir::HeadTerm::Aggregate(
                    crate::ir::HeadOp::ArgMax | crate::ir::HeadOp::ArgMin
                )
            )
        })
    {
        return Err(ValidationError::ArgAcrossRules {
            rules: lowered.len(),
        });
    }

    let mut pinned_row: Vec<ValueType> = Vec::new();
    let mut rules = Vec::with_capacity(lowered.len());
    let mut params = ParamTables::default();
    for (rule_idx, rule) in lowered.iter().enumerate() {
        check_head_alignment(&query.head, rule, rule_idx)?;
        let (typing, ctx) = validate_rule(schema, rule)?;
        // Every rule derives the predicate: rule 0's resolved positional
        // input row pins the head, and every later rule must agree
        // position by position (the input row, not the signature — a
        // `CountDistinct` position anchors its *input* type across
        // rules, though its signature column is U64 regardless).
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
    params.check_masks_and_density()?;

    // The predicate, derived ONCE — rule 0 speaks for every rule (the
    // alignment above), and nothing downstream re-derives the signature.
    let predicate = super::Predicate::derive(&lowered[0], &rules[0]);

    let ParamTables {
        param_types,
        param_kinds,
        mask_params,
        point_params,
    } = params;
    let set_params = param_kinds
        .into_iter()
        .filter_map(|(param, kind)| matches!(kind, ParamKind::Set).then_some(param))
        .collect();
    Ok(ValidatedQuery {
        lowered,
        predicate,
        rules,
        param_types,
        set_params,
        point_params,
        mask_params,
    })
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
/// rule's own variable scope and its own bivalent-anchor typing fixpoint.
fn validate_rule(
    schema: &Schema,
    rule: &LoweredRule,
) -> Result<(RuleTyping, Context), ValidationError> {
    if rule.atoms.is_empty() {
        return Err(ValidationError::NoPositiveAtoms);
    }
    // The planner caps are roster items: rejected here, at the boundary,
    // so nothing downstream (normalize's u16 occurrence ids, the DP's
    // bitmask table, the 128-bit variable bitsets) ever sees an
    // over-limit rule. Negated atoms are occurrences too — each one is
    // an anti-probe the DP places — so they count. Per rule: each rule
    // plans alone (the rule cap is counted independently, at the top).
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
    ctx.check_atoms(schema, rule)?;
    let classified = ctx.check_comparisons(rule)?;
    ctx.check_membership_domains()?;
    // The group key (non-aggregated find variables) is computed once and
    // shared between the find checks and the witness. A measure find's
    // variable is a group-key variable: the position projects a value per
    // binding (the measure word), so its variable is grouped-by exactly
    // like a plain projected variable — and an aggregate over it would be
    // constant per group (`AggregateOverGroupKey`).
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

    // The variable types are already resolved (`resolve_bivalents`
    // consumed the inference slots into `var_types` during
    // `check_comparisons`): the typing takes them verbatim.
    let var_types = ctx.var_types.clone();
    Ok((
        RuleTyping {
            var_types,
            group_key,
            classified,
        },
        ctx,
    ))
}

/// One rule's positional INPUT contribution to the alignment check: a
/// variable position carries the variable's type; an aggregate position
/// its fold input type (the nullary `Count` is `U64`; an Arg position
/// carries the *carried* variable's type — the key is rule-internal).
/// Alignment-only — the signature is [`super::Predicate::derive`],
/// never this row (their one divergence: a `CountDistinct` input).
fn input_row(rule: &LoweredRule, typing: &RuleTyping) -> Vec<ValueType> {
    let var_type = |var: &VarId| typing.var_types[var].clone();
    rule.finds
        .iter()
        .map(|term| match term {
            FindTerm::Var(var) => var_type(var),
            // The measure is u64 by definition — projected or folded.
            FindTerm::Measure(_) | FindTerm::AggregateMeasure { .. } => ValueType::U64,
            FindTerm::Aggregate { op, over } => match op {
                AggOp::Count => ValueType::U64,
                // A Pack position's row entry is an interval — the packed
                // segment shares its input's interval type, so the pinned
                // row carries `Interval(E)` there like any interval find.
                AggOp::Sum
                | AggOp::Min
                | AggOp::Max
                | AggOp::CountDistinct
                | AggOp::ArgMax { .. }
                | AggOp::ArgMin { .. }
                | AggOp::Pack => var_type(&over.expect("validated: only Count is nullary")),
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
    mask_params: BTreeSet<ParamId>,
    point_params: BTreeSet<ParamId>,
}

impl ParamTables {
    /// Absorbs one rule's resolved param state, diagnosing cross-rule
    /// disagreements with the same errors the per-rule checks use.
    fn unify(&mut self, ctx: Context) -> Result<(), ValidationError> {
        // Point-position params (the point-domain law): anchored at an
        // interval position and resolved element-typed — their bound
        // values are points, so bind rejects the domain ceiling.
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
                TypeSlot::Bivalent(_) => unreachable!("resolve_bivalents ran"),
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
        self.mask_params.extend(ctx.mask_params);
        Ok(())
    }

    /// The two whole-program param rules, checked after every rule
    /// contributed: a mask param with any value anchor anywhere (a mask
    /// is not a data-model type), and id density — jointly across value
    /// and mask params, across all rules (a gap would be a positional
    /// slot at execution whose supplied value is never type-checked).
    fn check_masks_and_density(&self) -> Result<(), ValidationError> {
        for param in &self.mask_params {
            if self.param_types.contains_key(param) {
                return Err(ValidationError::ParamTypeConflict { param: *param });
            }
        }
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
