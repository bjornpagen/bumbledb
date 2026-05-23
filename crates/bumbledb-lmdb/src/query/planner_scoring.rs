use super::*;

type VariableOrderKey<'a> = (
    usize,
    u64,
    std::cmp::Reverse<usize>,
    std::cmp::Reverse<usize>,
    std::cmp::Reverse<usize>,
    std::cmp::Reverse<usize>,
    &'a str,
);

pub(super) fn variable_order_key<'a>(
    score: &'a VariableOrderScore,
    query: &'a NormalizedQuery,
) -> VariableOrderKey<'a> {
    (
        score.field_position,
        score.candidate_estimate,
        std::cmp::Reverse(score.static_constraints),
        std::cmp::Reverse(score.bound_constraints),
        std::cmp::Reverse(score.relation_constraints),
        std::cmp::Reverse(score.degree),
        query.vars[score.variable].name.as_str(),
    )
}

pub(super) fn variable_order_score(
    schema: &StorageSchema,
    atoms: &[&NormAtom],
    comparisons: &[&NormPredicate],
    stats: &PlannerStats,
    bound: &BTreeSet<usize>,
    variable: usize,
) -> Result<VariableOrderScore> {
    let mut has_constrained_stream = false;
    let mut has_unconstrained_payload_stream = false;
    for atom in atoms
        .iter()
        .copied()
        .filter(|atom| atom_contains_variable(atom, variable))
    {
        let relation_constraints = atom_bound_constraint_count(atom, variable, bound);
        let static_constraints = atom_static_constraint_count(atom, variable)
            + comparison_static_constraint_count(comparisons, variable, bound);
        let has_unbound_other = atom_has_unbound_other_variable_id(atom, variable, bound);
        let strength = relation_constraints + static_constraints;
        has_constrained_stream |= strength > 0;
        has_unconstrained_payload_stream |= strength == 0 && has_unbound_other;
    }
    let mut best_access: Option<VariableAccessScore> = None;
    let mut relation_constraints = 0usize;
    let mut static_constraints = comparison_static_constraint_count(comparisons, variable, bound);
    let mut bound_constraints = comparison_bound_constraint_count(comparisons, variable, bound);

    for atom in atoms
        .iter()
        .copied()
        .filter(|atom| atom_contains_variable(atom, variable))
    {
        let strength = atom_bound_constraint_count(atom, variable, bound)
            + atom_static_constraint_count(atom, variable)
            + comparison_static_constraint_count(comparisons, variable, bound);
        let has_unbound_other = atom_has_unbound_other_variable_id(atom, variable, bound);
        relation_constraints += 1;
        static_constraints += atom_static_constraint_count(atom, variable);
        bound_constraints += atom_bound_constraint_count(atom, variable, bound);
        if has_constrained_stream && strength == 0 && has_unbound_other {
            continue;
        }
        let estimate = variable_access_score(schema, stats, bound, atom, variable)?;
        if best_access.as_ref().is_none_or(|best| {
            (
                estimate.fact_estimate,
                std::cmp::Reverse(estimate.prefix_len),
                std::cmp::Reverse(estimate.current_is_next),
                estimate.access_label(),
            ) < (
                best.fact_estimate,
                std::cmp::Reverse(best.prefix_len),
                std::cmp::Reverse(best.current_is_next),
                best.access_label(),
            )
        }) {
            best_access = Some(estimate);
        }
    }

    let degree = atoms
        .iter()
        .filter(|atom| atom_contains_variable(atom, variable))
        .count();
    let field_position = atoms
        .iter()
        .flat_map(|atom| atom.fields.iter())
        .filter_map(|field| match field.term {
            NormTerm::Var(id) if id.0 as usize == variable => Some(field.field.0 as usize),
            _ => None,
        })
        .min()
        .unwrap_or(usize::MAX);
    let mut candidate_estimate = best_access
        .as_ref()
        .map(|estimate| estimate.fact_estimate)
        .unwrap_or(u64::MAX / 4)
        .max(1);
    if static_constraints == 0
        && bound_constraints == 0
        && degree == 1
        && has_unconstrained_payload_stream
    {
        candidate_estimate = candidate_estimate.max(
            best_access
                .as_ref()
                .map(|estimate| stats.relation_facts(&estimate.relation))
                .unwrap_or(u64::MAX / 8),
        );
    }

    Ok(VariableOrderScore {
        variable,
        field_position,
        candidate_estimate,
        static_constraints,
        bound_constraints,
        relation_constraints,
        degree,
    })
}

fn variable_access_score(
    schema: &StorageSchema,
    stats: &PlannerStats,
    bound: &BTreeSet<usize>,
    atom: &NormAtom,
    variable: usize,
) -> Result<VariableAccessScore> {
    let paths = schema.access_paths(&atom.relation_name)?;
    let relation_facts = stats.relation_facts(&atom.relation_name);
    let mut best: Option<VariableAccessScore> = None;

    for path in paths {
        if !path.components.iter().any(|component| {
            atom.fields.iter().any(|field| {
                field.field_name == component.field_name
                    && matches!(field.term, NormTerm::Var(id) if id.0 as usize == variable)
            })
        }) {
            continue;
        }

        let mut prefix_len = 0usize;
        let mut current_is_next = false;
        for field_name in &path.leading_fields {
            let Some(field) = atom
                .fields
                .iter()
                .find(|field| &field.field_name == field_name)
            else {
                break;
            };
            if matches!(field.term, NormTerm::Var(id) if id.0 as usize == variable) {
                current_is_next = true;
                break;
            }
            if field_is_bound_for_estimate(field, bound) {
                prefix_len += 1;
            } else {
                break;
            }
        }

        let Some(index_stats) = stats.index_stats(&atom.relation_name, &path.index_name) else {
            continue;
        };
        let mut estimate = if current_is_next {
            if prefix_len == 0 {
                if path.kind == IndexKind::Range {
                    relation_facts.max(1).div_ceil(4)
                } else {
                    index_stats
                        .distinct_by_depth
                        .first()
                        .copied()
                        .unwrap_or(index_stats.facts)
                        .max(1) as u64
                }
            } else {
                index_stats.fanout_after_prefix(prefix_len)
            }
        } else {
            index_stats.estimated_facts_for_prefix(prefix_len)
        };
        if matches!(path.kind, IndexKind::FactSet | IndexKind::Unique)
            && current_is_next
            && prefix_len + 1 == path.leading_fields.len()
        {
            estimate = estimate.min(1);
        }
        let candidate = VariableAccessScore {
            relation: atom.relation_name.clone(),
            index: path.index_name,
            fact_estimate: estimate.max(1),
            prefix_len,
            current_is_next,
        };
        if best.as_ref().is_none_or(|best| {
            (
                candidate.fact_estimate,
                std::cmp::Reverse(candidate.prefix_len),
                std::cmp::Reverse(candidate.current_is_next),
                candidate.access_label(),
            ) < (
                best.fact_estimate,
                std::cmp::Reverse(best.prefix_len),
                std::cmp::Reverse(best.current_is_next),
                best.access_label(),
            )
        }) {
            best = Some(candidate);
        }
    }

    Ok(best.unwrap_or_else(|| VariableAccessScore {
        relation: atom.relation_name.clone(),
        index: "full_scan".to_owned(),
        fact_estimate: relation_facts.saturating_mul(4).max(1),
        prefix_len: 0,
        current_is_next: false,
    }))
}

fn field_is_bound_for_estimate(field: &NormAtomField, bound: &BTreeSet<usize>) -> bool {
    match field.term {
        NormTerm::Var(variable) => bound.contains(&(variable.0 as usize)),
        NormTerm::Input(_) | NormTerm::Literal(_) => true,
        NormTerm::Wildcard => false,
    }
}

fn atom_static_constraint_count(atom: &NormAtom, variable: usize) -> usize {
    atom.fields
        .iter()
        .filter(|field| {
            !matches!(field.term, NormTerm::Var(id) if id.0 as usize == variable)
                && matches!(field.term, NormTerm::Input(_) | NormTerm::Literal(_))
        })
        .count()
}

fn atom_bound_constraint_count(atom: &NormAtom, variable: usize, bound: &BTreeSet<usize>) -> usize {
    atom.fields
        .iter()
        .filter(|field| {
            matches!(field.term, NormTerm::Var(id) if id.0 as usize != variable && bound.contains(&(id.0 as usize)))
        })
        .count()
}

fn atom_has_unbound_other_variable_id(
    atom: &NormAtom,
    variable: usize,
    bound: &BTreeSet<usize>,
) -> bool {
    atom.fields.iter().any(|field| {
        matches!(field.term, NormTerm::Var(id) if id.0 as usize != variable && !bound.contains(&(id.0 as usize)))
    })
}

fn comparison_static_constraint_count(
    comparisons: &[&NormPredicate],
    variable: usize,
    bound: &BTreeSet<usize>,
) -> usize {
    comparisons
        .iter()
        .filter(|comparison| comparison_constrains_variable(comparison, variable, bound, true))
        .count()
}

fn comparison_bound_constraint_count(
    comparisons: &[&NormPredicate],
    variable: usize,
    bound: &BTreeSet<usize>,
) -> usize {
    comparisons
        .iter()
        .filter(|comparison| comparison_constrains_variable(comparison, variable, bound, false))
        .count()
}

fn comparison_constrains_variable(
    comparison: &NormPredicate,
    variable: usize,
    bound: &BTreeSet<usize>,
    static_only: bool,
) -> bool {
    let left_is_var =
        matches!(comparison.operands[0], NormOperand::Var(id) if id.0 as usize == variable);
    let right_is_var =
        matches!(comparison.operands[1], NormOperand::Var(id) if id.0 as usize == variable);
    if left_is_var {
        operand_constrains_for_estimate(&comparison.operands[1], bound, static_only)
    } else if right_is_var {
        operand_constrains_for_estimate(&comparison.operands[0], bound, static_only)
    } else {
        false
    }
}

fn operand_constrains_for_estimate(
    operand: &NormOperand,
    bound: &BTreeSet<usize>,
    static_only: bool,
) -> bool {
    match operand {
        NormOperand::Var(variable) => !static_only && bound.contains(&(variable.0 as usize)),
        NormOperand::Input(_) | NormOperand::Literal(_) => static_only,
    }
}
