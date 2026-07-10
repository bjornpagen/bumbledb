use super::{Context, ParamKind, RuleTyping, TypeSlot, ValidatedQuery};
use crate::error::ValidationError;
use crate::ir::normalize::{collapse, disjunct_count, distribute, LoweredRule};
use crate::ir::{AggOp, FindTerm, ParamId, Query, VarId, MAX_RULES};
use crate::schema::{Schema, ValueType};
use std::collections::{BTreeMap, BTreeSet};

/// Validates a query against the schema, yielding the sealed witness.
///
/// The program shape first (empty rule set, the rule cap, empty head);
/// then **DNF distribution** — each rule's predicate trees distribute
/// and each disjunct becomes a rule, with the blowup capped at
/// [`MAX_RULES`] on the structural term count (before materializing)
/// and duplicates collapsed by normalized-form equality; then each
/// lowered rule under the per-rule roster with its own typing fixpoint
/// — a rule validates exactly as a conjunctive query did — and finally
/// the query-global param unification (params are one binding surface
/// across rules; variables never cross them).
///
/// Duplicate and even statically contradictory predicates (`x < 5,
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

    let mut head_types: Vec<ValueType> = Vec::new();
    let mut rules = Vec::with_capacity(lowered.len());
    let mut params = ParamTables::default();
    for (rule_idx, rule) in lowered.iter().enumerate() {
        check_head_alignment(&query.head, rule, rule_idx)?;
        let (typing, ctx) = validate_rule(schema, rule)?;
        // The positional type row: rule 0's pins the head; every later
        // rule must agree position by position.
        let row = head_row(rule, &typing);
        if rule_idx == 0 {
            head_types = row;
        } else if let Some(position) = (0..row.len()).find(|i| row[*i] != head_types[*i]) {
            return Err(ValidationError::HeadTypeMismatch {
                rule: rule_idx,
                position,
            });
        }
        params.unify(ctx)?;
        rules.push(typing);
    }
    params.check_masks_and_density()?;

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
        head_types,
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
    ctx.check_comparisons(rule)?;
    ctx.check_membership_domains()?;
    // The group key (non-aggregated find variables) is computed once and
    // shared between the find checks and the witness.
    let group_key: BTreeSet<VarId> = rule
        .finds
        .iter()
        .filter_map(|term| match term {
            FindTerm::Var(var) => Some(*var),
            FindTerm::Aggregate { .. } => None,
        })
        .collect();
    ctx.check_finds(rule, &group_key)?;
    if ctx.var_slots.len() > crate::plan::planner::MAX_DISTINCT_VARS {
        return Err(ValidationError::TooManyVariables {
            count: ctx.var_slots.len(),
        });
    }

    // Every slot is monovalent past `resolve_bivalents` — the typing
    // carries plain types.
    let var_types = ctx
        .var_slots
        .iter()
        .map(|(var, slot)| match slot {
            TypeSlot::Mono(value_type) => (*var, value_type.clone()),
            TypeSlot::Bivalent(_) => unreachable!("resolve_bivalents ran"),
        })
        .collect();
    Ok((
        RuleTyping {
            var_types,
            group_key,
        },
        ctx,
    ))
}

/// One rule's positional type contribution: a variable position carries
/// the variable's type; an aggregate position its fold input type (the
/// nullary `Count` is `U64`; an Arg position carries the *carried*
/// variable's type — the key is rule-internal).
fn head_row(rule: &LoweredRule, typing: &RuleTyping) -> Vec<ValueType> {
    let var_type = |var: &VarId| typing.var_types[var].clone();
    rule.finds
        .iter()
        .map(|term| match term {
            FindTerm::Var(var) => var_type(var),
            FindTerm::Aggregate { op, over } => match op {
                AggOp::Count => ValueType::U64,
                AggOp::Sum
                | AggOp::Min
                | AggOp::Max
                | AggOp::CountDistinct
                | AggOp::ArgMax { .. }
                | AggOp::ArgMin { .. } => {
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
