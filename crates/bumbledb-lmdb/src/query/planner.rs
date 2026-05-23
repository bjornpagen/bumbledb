fn plan_query(
    schema: &StorageSchema,
    query: &mut NormalizedQuery,
    image: &crate::QueryImage,
    query_image_cache: QueryImageCacheDiagnostics,
) -> Result<ExecutionPlan> {
    let _span = tracing::debug_span!("bumbledb.query.plan").entered();
    let variable_order_ids = {
        let relation_atoms = query.atoms.iter().collect::<Vec<_>>();
        let comparisons = query.predicates.iter().collect::<Vec<_>>();
        let stats = {
            let _span =
                tracing::debug_span!("bumbledb.query.plan.stats", atoms = relation_atoms.len())
                    .entered();
            PlannerStats::collect(schema, image, &relation_atoms)?
        };
        {
            let _span = tracing::debug_span!(
                "bumbledb.query.plan.variable_order",
                variables = query.vars.len()
            )
            .entered();
            choose_variable_order(schema, query, &relation_atoms, &comparisons, &stats)?
        }
    };
    attach_predicate_depths(query, &variable_order_ids);
    let variable_order = variable_order_ids
        .iter()
        .map(|id| query.vars[*id].name.clone())
        .collect::<Vec<_>>();
    let free_join = {
        let _span = tracing::debug_span!(
            "bumbledb.query.plan.free_join",
            atoms = query.atoms.len(),
            variables = variable_order_ids.len()
        )
        .entered();
        build_free_join_plan(query, &variable_order_ids)
    };
    free_join.validate()?;
    let planner_stats = image.planner_stats_diagnostics();

    let execution_plan = ExecutionPlan {
        comparisons: query.predicates.clone(),
        summary: QueryPlan {
            variable_order,
            query_image_cache,
            planner_stats,
            free_join,
            timings: QueryTimings::default(),
            allocations: QueryAllocationStats::default(),
            counters: PlanCounters::default(),
        },
    };
    Ok(execution_plan)
}

fn choose_variable_order(
    schema: &StorageSchema,
    query: &NormalizedQuery,
    atoms: &[&NormAtom],
    comparisons: &[&NormPredicate],
    stats: &PlannerStats,
) -> Result<Vec<usize>> {
    let mut remaining = vec![true; query.vars.len()];
    let mut remaining_count = query.vars.len();
    let mut bound = BTreeSet::new();
    let mut order = Vec::with_capacity(query.vars.len());
    let predecessors = variable_predecessors(schema, query.vars.len(), atoms)?;

    while remaining_count != 0 {
        let mut best = None;
        for (variable, is_remaining) in remaining.iter().copied().enumerate() {
            if !is_remaining {
                continue;
            }
            if predecessors[variable]
                .iter()
                .any(|predecessor| remaining[*predecessor])
            {
                continue;
            }
            let score = variable_order_score(schema, atoms, comparisons, stats, &bound, variable)?;
            if best.as_ref().is_none_or(|best: &VariableOrderScore| {
                variable_order_key(&score, query) < variable_order_key(best, query)
            }) {
                best = Some(score);
            }
        }
        let best = if let Some(best) = best {
            best
        } else {
            let mut fallback = None;
            for (variable, is_remaining) in remaining.iter().copied().enumerate() {
                if !is_remaining {
                    continue;
                }
                let score = variable_order_score(schema, atoms, comparisons, stats, &bound, variable)?;
                if fallback.as_ref().is_none_or(|best: &VariableOrderScore| {
                    variable_order_key(&score, query) < variable_order_key(best, query)
                }) {
                    fallback = Some(score);
                }
            }
            fallback.ok_or_else(|| Error::internal("query has no remaining variables"))?
        };
        remaining[best.variable] = false;
        remaining_count -= 1;
        bound.insert(best.variable);
        order.push(best.variable);
    }

    Ok(order)
}

fn variable_predecessors(
    schema: &StorageSchema,
    variable_count: usize,
    atoms: &[&NormAtom],
) -> Result<Vec<BTreeSet<usize>>> {
    let mut predecessors = vec![BTreeSet::new(); variable_count];
    for atom in atoms {
        let variables = access_order_variables(schema, atom)?;
        for left in 0..variables.len() {
            for right in left + 1..variables.len() {
                let predecessor = variables[left];
                let variable = variables[right];
                if variable < predecessors.len() && predecessor != variable {
                    predecessors[variable].insert(predecessor);
                }
            }
        }
    }
    Ok(predecessors)
}

fn access_order_variables(schema: &StorageSchema, atom: &NormAtom) -> Result<Vec<usize>> {
    let atom_variables = atom_variables(atom);
    let mut best = Vec::new();
    for path in schema.access_paths(&atom.relation_name)? {
        let mut saw_variable = false;
        let mut variables = Vec::new();
        for field_name in &path.leading_fields {
            let Some(field) = atom
                .fields
                .iter()
                .find(|field| &field.field_name == field_name)
            else {
                if saw_variable {
                    continue;
                }
                break;
            };
            match field.term {
                NormTerm::Var(variable) => {
                    saw_variable = true;
                    let variable = variable.0 as usize;
                    if !variables.contains(&variable) {
                        variables.push(variable);
                    }
                }
                NormTerm::Input(_) | NormTerm::Literal(_) => {}
                NormTerm::Wildcard if saw_variable => {}
                NormTerm::Wildcard => break,
            }
        }
        if atom_variables.iter().all(|variable| variables.contains(variable))
            && variables.len() > best.len()
        {
            best = variables;
        }
    }
    if best.is_empty() {
        let mut variables = atom
            .fields
            .iter()
            .filter_map(|field| match field.term {
                NormTerm::Var(variable) => Some((field.field.0, variable.0 as usize)),
                NormTerm::Input(_) | NormTerm::Literal(_) | NormTerm::Wildcard => None,
            })
            .collect::<Vec<_>>();
        variables.sort_unstable();
        variables.dedup_by_key(|(_, variable)| *variable);
        best = variables.into_iter().map(|(_, variable)| variable).collect();
    }
    Ok(best)
}

type VariableOrderKey<'a> = (
    usize,
    u64,
    std::cmp::Reverse<usize>,
    std::cmp::Reverse<usize>,
    std::cmp::Reverse<usize>,
    std::cmp::Reverse<usize>,
    &'a str,
);

fn variable_order_key<'a>(
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

fn variable_order_score(
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

fn build_free_join_plan(query: &NormalizedQuery, variable_order_ids: &[usize]) -> FreeJoinPlan {
    let mut nodes = Vec::new();
    for (node_id, variable) in variable_order_ids.iter().enumerate() {
        let var_id = VarId(*variable as u16);
        nodes.push(PlanNode {
            id: NodeId(node_id as u16),
            bind_vars: vec![var_id],
        });
    }

    FreeJoinPlan {
        nodes,
        output: output_plan(query),
    }
}

fn output_plan(query: &NormalizedQuery) -> OutputPlan {
    output_plan_from_find(&query.find)
}

fn output_plan_from_find(find: &[NormFindTerm]) -> OutputPlan {
    OutputPlan::Project(ProjectPlan {
        vars: find
            .iter()
            .map(|term| match term {
                NormFindTerm::Variable { variable } => *variable,
            })
            .collect(),
    })
}

fn atom_contains_variable(atom: &NormAtom, variable: usize) -> bool {
    atom.fields
        .iter()
        .any(|field| matches!(field.term, NormTerm::Var(id) if id.0 as usize == variable))
}

fn atom_variables(atom: &NormAtom) -> BTreeSet<usize> {
    atom.fields
        .iter()
        .filter_map(|field| match field.term {
            NormTerm::Var(variable) => Some(variable.0 as usize),
            NormTerm::Input(_) | NormTerm::Literal(_) | NormTerm::Wildcard => None,
        })
        .collect()
}

fn comparisons_ready_pass(
    txn: &ReadTxn<'_>,
    comparisons: &[NormPredicate],
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    binding: &EncodedBinding,
    counters: &mut PlanCounters,
) -> Result<bool> {
    for comparison in comparisons {
        let Some(left_encoded) = operand_encoded_value(
            &comparison.operands[0],
            &comparison.value_type,
            inputs,
            binding,
        ) else {
            continue;
        };
        let Some(right_encoded) = operand_encoded_value(
            &comparison.operands[1],
            &comparison.value_type,
            inputs,
            binding,
        ) else {
            continue;
        };
        if encoded_comparison_supported(comparison.op, &comparison.value_type) {
            counters.comparisons_evaluated += 1;
            counters.encoded_comparisons_evaluated += 1;
            if !compare_encoded_values(
                left_encoded.as_bytes(),
                comparison.op,
                right_encoded.as_bytes(),
            ) {
                counters.comparisons_failed += 1;
                return Ok(false);
            }
            continue;
        }

        let Some(left) = operand_logical_value(
            txn,
            &comparison.operands[0],
            &comparison.value_type,
            query,
            inputs,
            binding,
            counters,
        )?
        else {
            continue;
        };
        let Some(right) = operand_logical_value(
            txn,
            &comparison.operands[1],
            &comparison.value_type,
            query,
            inputs,
            binding,
            counters,
        )?
        else {
            continue;
        };
        counters.comparisons_evaluated += 1;
        counters.decoded_comparisons_evaluated += 1;
        if !compare_values(&left, comparison.op, &right) {
            counters.comparisons_failed += 1;
            return Ok(false);
        }
    }
    Ok(true)
}

fn operand_encoded_value(
    operand: &NormOperand,
    _value_type: &ValueType,
    inputs: &EncodedInputs,
    binding: &EncodedBinding,
) -> Option<EncodedOwned> {
    match operand {
        NormOperand::Var(variable) => binding.get(variable.0 as usize).cloned(),
        NormOperand::Input(input) => inputs.get(*input).cloned(),
        NormOperand::Literal(literal) => Some(literal.clone()),
    }
}

fn encoded_comparison_supported(operator: ComparisonOperator, value_type: &ValueType) -> bool {
    match operator {
        ComparisonOperator::Eq | ComparisonOperator::NotEq => true,
        ComparisonOperator::Lt
        | ComparisonOperator::Lte
        | ComparisonOperator::Gt
        | ComparisonOperator::Gte => !matches!(value_type, ValueType::String | ValueType::Bytes),
    }
}

fn compare_encoded_values(left: &[u8], operator: ComparisonOperator, right: &[u8]) -> bool {
    match operator {
        ComparisonOperator::Eq => left == right,
        ComparisonOperator::NotEq => left != right,
        ComparisonOperator::Lt => left < right,
        ComparisonOperator::Lte => left <= right,
        ComparisonOperator::Gt => left > right,
        ComparisonOperator::Gte => left >= right,
    }
}
