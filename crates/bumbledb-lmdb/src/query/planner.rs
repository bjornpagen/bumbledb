fn plan_query(
    schema: &StorageSchema,
    query: &mut NormalizedQuery,
    image: &crate::QueryImage,
    query_image_cache: QueryImageCacheDiagnostics,
    prepared_plan_cache: PreparedPlanCacheDiagnostics,
) -> Result<ExecutionPlan> {
    let _span = tracing::debug_span!("bumbledb.query.plan").entered();
    let (stats, variable_order_ids, variable_costs) = {
        let relation_atoms = query.atoms.iter().collect::<Vec<_>>();
        let comparisons = query.predicates.iter().collect::<Vec<_>>();
        let stats = {
            let _span =
                tracing::debug_span!("bumbledb.query.plan.stats", atoms = relation_atoms.len())
                    .entered();
            PlannerStats::collect(schema, image, &relation_atoms)?
        };
        let (variable_order_ids, variable_costs) = {
            let _span = tracing::debug_span!(
                "bumbledb.query.plan.variable_order",
                variables = query.vars.len()
            )
            .entered();
            choose_variable_order(schema, query, &relation_atoms, &comparisons, &stats)?
        };
        (stats, variable_order_ids, variable_costs)
    };
    attach_predicate_depths(query, &variable_order_ids);
    let relation_atoms = query.atoms.iter().collect::<Vec<_>>();
    let variable_order = variable_order_ids
        .iter()
        .map(|id| query.vars[*id].name.clone())
        .collect::<Vec<_>>();
    let variable_estimates = variable_costs
        .iter()
        .map(|cost| VariableEstimate {
            variable: query.vars[cost.variable].name.clone(),
            estimated_candidates: cost.estimated_candidates,
            static_constraints: cost.static_constraints,
            bound_constraints: cost.bound_constraints,
            relation_constraints: cost.relation_constraints,
            access: cost.access.clone(),
            reason: cost.reason.clone(),
        })
        .collect::<Vec<_>>();
    let node_facts = variable_order_ids
        .iter()
        .enumerate()
        .map(|(node_id, variable)| NodeFactEstimate {
            node: NodeId(node_id as u16),
            variable: query.vars[*variable].name.clone(),
            estimated_facts: variable_costs
                .get(node_id)
                .map_or(1, |cost| cost.estimated_candidates),
            actual_facts: 0,
        })
        .collect::<Vec<_>>();
    let missing_indexes = missing_index_recommendations(schema, query, &relation_atoms)?;
    let (free_join, optimizer) = {
        let _span = tracing::debug_span!(
            "bumbledb.query.plan.optimize_free_join",
            atoms = relation_atoms.len(),
            variables = variable_order_ids.len()
        )
        .entered();
        optimize_free_join_plan(
            schema,
            query,
            &relation_atoms,
            &variable_order_ids,
            &variable_costs,
            &stats,
        )?
    };
    free_join.validate()?;
    let node_timings = query_node_timings(&free_join, &node_facts);
    let planner_stats = image.planner_stats_diagnostics();

    let uses_indexed_multiway_join = relation_atoms.len() > 1;
    let execution_plan = ExecutionPlan {
        variable_order_ids,
        relation_atoms: query.atoms.clone(),
        comparisons: query.predicates.clone(),
        summary: QueryPlan {
            variable_order,
            variable_estimates,
            missing_indexes,
            optimizer,
            query_image_cache,
            planner_stats,
            prepared_plan_cache,
            node_facts,
            node_timings,
            free_join,
            timings: QueryTimings::default(),
            allocations: QueryAllocationStats::default(),
            counters: PlanCounters::default(),
            uses_indexed_multiway_join,
        },
    };
    Ok(execution_plan)
}

fn query_node_timings(
    free_join: &FreeJoinPlan,
    node_facts: &[NodeFactEstimate],
) -> Vec<QueryNodeTiming> {
    free_join
        .nodes
        .iter()
        .map(|node| {
            let facts = node_facts.get(node.id.0 as usize);
            QueryNodeTiming {
                node: node.id,
                implementation: node.implementation,
                bind_vars: node.bind_vars.clone(),
                estimated_facts: facts.map_or(0, |facts| facts.estimated_facts),
                actual_facts: facts.map_or(0, |facts| facts.actual_facts),
                execute_micros: 0,
            }
        })
        .collect()
}

fn choose_variable_order(
    schema: &StorageSchema,
    query: &NormalizedQuery,
    atoms: &[&NormAtom],
    comparisons: &[&NormPredicate],
    stats: &PlannerStats,
) -> Result<(Vec<usize>, Vec<VariableCost>)> {
    let mut remaining = vec![true; query.vars.len()];
    let mut remaining_count = query.vars.len();
    let mut bound = BTreeSet::new();
    let mut order = Vec::with_capacity(query.vars.len());
    let mut costs = Vec::with_capacity(query.vars.len());

    while remaining_count != 0 {
        let mut best = None;
        for (variable, is_remaining) in remaining.iter().copied().enumerate() {
            if !is_remaining {
                continue;
            }
            let cost = estimate_variable_cost(schema, atoms, comparisons, stats, &bound, variable)?;
            if best.as_ref().is_none_or(|best: &VariableCost| {
                variable_cost_order_key(&cost, query) < variable_cost_order_key(best, query)
            }) {
                best = Some(cost);
            }
        }
        let best = best.ok_or_else(|| Error::internal("query has no remaining variables"))?;
        remaining[best.variable] = false;
        remaining_count -= 1;
        bound.insert(best.variable);
        order.push(best.variable);
        costs.push(best);
    }

    Ok((order, costs))
}

type VariableCostOrderKey<'a> = (
    u64,
    std::cmp::Reverse<usize>,
    std::cmp::Reverse<usize>,
    std::cmp::Reverse<usize>,
    std::cmp::Reverse<usize>,
    &'a str,
);

fn variable_cost_order_key<'a>(
    cost: &'a VariableCost,
    query: &'a NormalizedQuery,
) -> VariableCostOrderKey<'a> {
    (
        cost.estimated_candidates,
        std::cmp::Reverse(cost.static_constraints),
        std::cmp::Reverse(cost.bound_constraints),
        std::cmp::Reverse(cost.relation_constraints),
        std::cmp::Reverse(cost.degree),
        query.vars[cost.variable].name.as_str(),
    )
}

fn estimate_variable_cost(
    schema: &StorageSchema,
    atoms: &[&NormAtom],
    comparisons: &[&NormPredicate],
    stats: &PlannerStats,
    bound: &BTreeSet<usize>,
    variable: usize,
) -> Result<VariableCost> {
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
    let mut best_access: Option<AccessEstimate> = None;
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
        let estimate = estimate_atom_variable_access(schema, stats, bound, atom, variable)?;
        if best_access.as_ref().is_none_or(|best| {
            (
                estimate.estimated_facts,
                std::cmp::Reverse(estimate.prefix_len),
                std::cmp::Reverse(estimate.current_is_next),
                estimate.access_label(),
            ) < (
                best.estimated_facts,
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
    let mut estimated_candidates = best_access
        .as_ref()
        .map(|estimate| estimate.estimated_facts)
        .unwrap_or(u64::MAX / 4)
        .max(1);
    if static_constraints == 0
        && bound_constraints == 0
        && degree == 1
        && has_unconstrained_payload_stream
    {
        estimated_candidates = estimated_candidates.max(
            best_access
                .as_ref()
                .map(|estimate| stats.relation_facts(&estimate.relation))
                .unwrap_or(u64::MAX / 8),
        );
    }
    let access = best_access
        .as_ref()
        .map(AccessEstimate::access_label)
        .unwrap_or_else(|| "unindexed".to_owned());
    let reason = best_access
        .as_ref()
        .map(AccessEstimate::reason)
        .unwrap_or_else(|| "no relation stats for variable".to_owned());

    Ok(VariableCost {
        variable,
        estimated_candidates,
        static_constraints,
        bound_constraints,
        relation_constraints,
        degree,
        access,
        reason,
    })
}

fn estimate_atom_variable_access(
    schema: &StorageSchema,
    stats: &PlannerStats,
    bound: &BTreeSet<usize>,
    atom: &NormAtom,
    variable: usize,
) -> Result<AccessEstimate> {
    let paths = schema.access_paths(&atom.relation_name)?;
    let relation_facts = stats.relation_facts(&atom.relation_name);
    let mut best: Option<AccessEstimate> = None;

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
        let variable_field_stats = atom
            .fields
            .iter()
            .find(|field| matches!(field.term, NormTerm::Var(id) if id.0 as usize == variable))
            .and_then(|field| stats.field_stats(&atom.relation_name, &field.field_name));
        let distinct = index_stats
            .distinct_by_depth
            .get(prefix_len.saturating_sub(1))
            .copied()
            .unwrap_or(1);
        let candidate = AccessEstimate {
            relation: atom.relation_name.clone(),
            index: path.index_name,
            access: index_stats.index,
            estimated_facts: estimate.max(1),
            prefix_len,
            current_is_next,
            distinct,
            avg_fanout: index_stats.fanout_after_prefix(prefix_len),
            max_fanout: index_stats.max_fanout_after_prefix(prefix_len),
            variable_distinct: variable_field_stats.map_or(1, |stats| stats.distinct),
            has_min: variable_field_stats.is_some_and(|stats| stats.min.is_some()),
            has_max: variable_field_stats.is_some_and(|stats| stats.max.is_some()),
            heavy_hitters: variable_field_stats.map_or(0, |stats| stats.heavy_hitters.len()),
        };
        if best.as_ref().is_none_or(|best| {
            (
                candidate.estimated_facts,
                std::cmp::Reverse(candidate.prefix_len),
                std::cmp::Reverse(candidate.current_is_next),
                candidate.access_label(),
            ) < (
                best.estimated_facts,
                std::cmp::Reverse(best.prefix_len),
                std::cmp::Reverse(best.current_is_next),
                best.access_label(),
            )
        }) {
            best = Some(candidate);
        }
    }

    Ok(best.unwrap_or_else(|| AccessEstimate {
        relation: atom.relation_name.clone(),
        index: "full_scan".to_owned(),
        access: AccessId(0),
        estimated_facts: relation_facts.saturating_mul(4).max(1),
        prefix_len: 0,
        current_is_next: false,
        distinct: 1,
        avg_fanout: relation_facts.max(1),
        max_fanout: relation_facts as usize,
        variable_distinct: 1,
        has_min: false,
        has_max: false,
        heavy_hitters: 0,
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

fn missing_index_recommendations(
    schema: &StorageSchema,
    query: &NormalizedQuery,
    atoms: &[&NormAtom],
) -> Result<Vec<MissingIndexRecommendation>> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let mut variable_degree = vec![0usize; query.vars.len()];
    for atom in atoms {
        for variable in atom_variables(atom) {
            variable_degree[variable] += 1;
        }
    }
    for atom in atoms {
        let (_, relation) = schema.relation(&atom.relation_name)?;
        for field in &atom.fields {
            match field.term {
                NormTerm::Input(_) | NormTerm::Literal(_) => {
                    if has_leading_index(schema, &atom.relation_name, &field.field_name)? {
                        continue;
                    }
                    let fields = recommended_index_fields(relation, &field.field_name);
                    if seen.insert((atom.relation_name.clone(), fields.clone())) {
                        out.push(MissingIndexRecommendation {
                            relation: atom.relation_name.clone(),
                            fields,
                            reason: "StaticPredicate: chosen prefix has no leading index"
                                .to_owned(),
                        });
                    }
                }
                NormTerm::Var(variable) if variable_degree[variable.0 as usize] > 1 => {
                    if has_leading_index(schema, &atom.relation_name, &field.field_name)? {
                        continue;
                    }
                    let fields = recommended_index_fields(relation, &field.field_name);
                    if seen.insert((atom.relation_name.clone(), fields.clone())) {
                        out.push(MissingIndexRecommendation {
                            relation: atom.relation_name.clone(),
                            fields,
                            reason: "JoinPrefix: joined variable has no leading index".to_owned(),
                        });
                    }
                }
                NormTerm::Var(_) | NormTerm::Wildcard => {}
            }
        }
    }
    Ok(out)
}

fn has_leading_index(schema: &StorageSchema, relation: &str, field: &str) -> Result<bool> {
    Ok(schema.access_paths(relation)?.iter().any(|path| {
        path.leading_fields
            .first()
            .is_some_and(|leading| leading == field)
    }))
}

fn recommended_index_fields(
    relation: &bumbledb_core::schema::RelationDescriptor,
    field: &str,
) -> Vec<String> {
    let mut fields = vec![field.to_owned()];
    for primary in first_unique_fields(relation) {
        if !fields.iter().any(|field| field == primary) {
            fields.push(primary.clone());
        }
    }
    fields
}

fn first_unique_fields(relation: &bumbledb_core::schema::RelationDescriptor) -> &[String] {
    relation
        .constraints
        .iter()
        .find_map(|constraint| match constraint {
            bumbledb_core::schema::ConstraintDescriptor::Unique { fields, .. } => {
                Some(fields.as_slice())
            }
            bumbledb_core::schema::ConstraintDescriptor::ForeignKey { .. } => None,
        })
        .unwrap_or(&[])
}

fn optimize_free_join_plan(
    schema: &StorageSchema,
    query: &NormalizedQuery,
    atoms: &[&NormAtom],
    variable_order_ids: &[usize],
    variable_costs: &[VariableCost],
    stats: &PlannerStats,
) -> Result<(FreeJoinPlan, OptimizerTrace)> {
    let cyclic = is_cyclic_multiway_query(query, atoms);
    let mut candidates = Vec::new();

    let lftj_impls = vec![NodeImpl::SortedLeapfrog; variable_order_ids.len()];
    candidates.push(build_plan_candidate(
        "free_join_sorted_leapfrog",
        query,
        atoms,
        variable_costs,
        stats,
        lftj_impls,
        cyclic,
    )?);

    candidates.sort_by_key(|candidate| candidate.cost.clone());
    let chosen = candidates
        .first()
        .ok_or_else(|| Error::internal("no optimizer plan candidates"))?
        .name
        .clone();
    let chosen_candidate = candidates
        .iter()
        .find(|candidate| candidate.name == chosen)
        .ok_or_else(|| Error::internal("chosen optimizer candidate missing"))?;
    let plan = build_free_join_plan(
        schema,
        query,
        atoms,
        variable_order_ids,
        &chosen_candidate.implementations,
        stats,
        chosen_candidate.estimates.clone(),
    )?;
    let trace_candidates = candidates
        .into_iter()
        .map(|candidate| PlanCandidate {
            selected: candidate.name == chosen,
            rejected_reason: if candidate.name == chosen {
                "selected minimum stable cost".to_owned()
            } else {
                "higher stable cost".to_owned()
            },
            name: candidate.name,
            implementations: candidate.implementations,
            cost: candidate.cost,
        })
        .collect::<Vec<_>>();

    Ok((
        plan,
        OptimizerTrace {
            chosen,
            candidates: trace_candidates,
        },
    ))
}

#[derive(Clone, Debug)]
struct OptimizerCandidate {
    name: String,
    implementations: Vec<NodeImpl>,
    cost: CostKey,
    estimates: PlanEstimates,
}

fn build_plan_candidate(
    name: &str,
    query: &NormalizedQuery,
    atoms: &[&NormAtom],
    variable_costs: &[VariableCost],
    stats: &PlannerStats,
    implementations: Vec<NodeImpl>,
    cyclic: bool,
) -> Result<OptimizerCandidate> {
    let estimates = estimate_free_join_plan(name, query, atoms, variable_costs, stats, cyclic);
    let cost = CostKey {
        estimated_micros: estimates
            .iterator_ops
            .saturating_add(estimates.hash_build_facts / HASH_BUILD_ROWS_PER_MICRO)
            .saturating_add(estimates.materialized_values),
        setup_micros: estimated_setup_micros(name, &estimates),
        memory_bytes: estimates.memory_bytes,
        materialization_penalty: estimates.materialized_values,
        candidate_rank: candidate_rank(name),
        implementation_mask: implementation_mask(&implementations),
    };
    Ok(OptimizerCandidate {
        name: name.to_owned(),
        implementations,
        cost,
        estimates,
    })
}

fn candidate_rank(name: &str) -> u8 {
    match name {
        "free_join_sorted_leapfrog" => 0,
        _ => u8::MAX,
    }
}

fn implementation_mask(implementations: &[NodeImpl]) -> u64 {
    implementations
        .iter()
        .take(16)
        .enumerate()
        .fold(0u64, |mask, (index, implementation)| {
            let code = match implementation {
                NodeImpl::SortedLeapfrog => 1,
            };
            mask | ((code as u64) << (index * 4))
        })
}

fn estimated_setup_micros(name: &str, estimates: &PlanEstimates) -> u64 {
    let query_image_cost = estimates.output_facts.clamp(1, 1_000);
    let hash_cost = estimates.hash_build_facts / HASH_BUILD_ROWS_PER_MICRO;
    let sorted_cost = if name == "free_join_sorted_leapfrog" {
        estimates.iterator_ops / 10
    } else {
        0
    };
    query_image_cost
        .saturating_add(hash_cost)
        .saturating_add(sorted_cost)
}

fn build_free_join_plan(
    schema: &StorageSchema,
    query: &NormalizedQuery,
    atoms: &[&NormAtom],
    variable_order_ids: &[usize],
    implementations: &[NodeImpl],
    stats: &PlannerStats,
    estimates: PlanEstimates,
) -> Result<FreeJoinPlan> {
    let mut nodes = Vec::new();
    let mut bound = BTreeSet::new();
    for (node_id, variable) in variable_order_ids.iter().enumerate() {
        let var_id = VarId(*variable as u16);
        let subatoms = atoms
            .iter()
            .enumerate()
            .map(|(atom_id, atom)| {
                let fields = atom
                    .fields
                    .iter()
                    .filter(
                        |field| matches!(field.term, NormTerm::Var(id) if id.0 as usize == *variable),
                    )
                    .map(|field| field.field)
                    .collect::<Vec<_>>();
                if fields.is_empty() {
                    return Ok(None);
                }
                let access =
                    estimate_atom_variable_access(schema, stats, &bound, atom, *variable)?.access;
                Ok(Some(SubAtom {
                    atom_id: AtomId(atom_id as u16),
                    relation: atom.relation,
                    vars: vec![var_id; fields.len()],
                    fields,
                    access,
                }))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        nodes.push(PlanNode {
            id: NodeId(node_id as u16),
            bind_vars: vec![var_id],
            subatoms,
            implementation: implementations
                .get(node_id)
                .copied()
                .unwrap_or(NodeImpl::SortedLeapfrog),
        });
        bound.insert(*variable);
    }

    Ok(FreeJoinPlan {
        nodes,
        output: output_plan(query),
        estimates,
    })
}

fn estimate_free_join_plan(
    name: &str,
    query: &NormalizedQuery,
    atoms: &[&NormAtom],
    variable_costs: &[VariableCost],
    stats: &PlannerStats,
    cyclic: bool,
) -> PlanEstimates {
    let mut iterator_ops = 0u64;
    let mut hash_build_facts = 0u64;
    for cost in variable_costs {
        let variable_ops =
            cost.estimated_candidates
                .max(1)
                .saturating_mul(if cyclic { 1 } else { 3 });
        iterator_ops = iterator_ops.saturating_add(variable_ops);
    }
    for atom in atoms {
        if atom_variables(atom).is_empty() {
            hash_build_facts =
                hash_build_facts.saturating_add(stats.relation_facts(&atom.relation_name));
        }
    }

    if cyclic && name != "free_join_sorted_leapfrog" {
        iterator_ops = iterator_ops.saturating_mul(8);
    }

    let output_facts = estimate_output_facts(query, variable_costs);
    let materialized_values = estimate_materialized_values(query, output_facts);
    let memory_bytes = (hash_build_facts as usize)
        .saturating_mul(32)
        .saturating_add(materialized_values as usize * 16);

    PlanEstimates {
        output_facts,
        iterator_ops,
        hash_build_facts,
        materialized_values,
        memory_bytes,
    }
}

fn estimate_output_facts(query: &NormalizedQuery, variable_costs: &[VariableCost]) -> u64 {
    let has_aggregate = has_aggregate(query);
    let group_vars = query
        .find
        .iter()
        .filter(|term| matches!(term, NormFindTerm::Variable { .. }))
        .count() as u64;
    if has_aggregate && group_vars == 0 {
        return 1;
    }
    variable_costs
        .iter()
        .map(|cost| cost.estimated_candidates)
        .min()
        .unwrap_or(1)
        .max(1)
}

fn estimate_materialized_values(query: &NormalizedQuery, output_facts: u64) -> u64 {
    let projected_values = query.find.len() as u64;
    output_facts
        .saturating_mul(projected_values)
        .max(projected_values)
}

fn is_cyclic_multiway_query(query: &NormalizedQuery, atoms: &[&NormAtom]) -> bool {
    if atoms.len() < 3 {
        return false;
    }
    let mut degree = vec![0usize; query.vars.len()];
    for atom in atoms {
        for variable in atom_variables(atom) {
            degree[variable] += 1;
        }
    }
    degree
        .into_iter()
        .filter(|count| *count > 0)
        .all(|count| count >= 2)
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

fn output_plan(query: &NormalizedQuery) -> OutputPlan {
    output_plan_from_find(&query.find)
}

fn output_plan_from_find(find: &[NormFindTerm]) -> OutputPlan {
    if find
        .iter()
        .any(|term| matches!(term, NormFindTerm::Aggregate { .. }))
    {
        let mut group_vars = Vec::new();
        let mut aggregates = Vec::new();
        for term in find {
            match term {
                NormFindTerm::Variable { variable } => group_vars.push(*variable),
                NormFindTerm::Aggregate {
                    function,
                    variable,
                    domain,
                    value_type,
                } => aggregates.push(AggregateTerm {
                    function: *function,
                    var: *variable,
                    domain_vars: domain.clone(),
                    value_type: value_type.clone(),
                }),
            }
        }
        OutputPlan::Aggregate(AggregatePlan {
            group_vars,
            aggregates,
        })
    } else {
        OutputPlan::Project(ProjectPlan {
            vars: find
                .iter()
                .filter_map(|term| match term {
                    NormFindTerm::Variable { variable } => Some(*variable),
                    NormFindTerm::Aggregate { .. } => None,
                })
                .collect(),
        })
    }
}

fn atom_contains_variable(atom: &NormAtom, variable: usize) -> bool {
    atom.fields
        .iter()
        .any(|field| matches!(field.term, NormTerm::Var(id) if id.0 as usize == variable))
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

