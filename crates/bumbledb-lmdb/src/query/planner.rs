use super::*;

pub(super) fn plan_query(
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
                let score =
                    variable_order_score(schema, atoms, comparisons, stats, &bound, variable)?;
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
        if atom_variables
            .iter()
            .all(|variable| variables.contains(variable))
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
        best = variables
            .into_iter()
            .map(|(_, variable)| variable)
            .collect();
    }
    Ok(best)
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

pub(in crate::query) fn output_plan_from_find(find: &[NormFindTerm]) -> OutputPlan {
    OutputPlan::Project(ProjectPlan {
        vars: find
            .iter()
            .map(|term| match term {
                NormFindTerm::Variable { variable } => *variable,
            })
            .collect(),
    })
}

pub(super) fn atom_contains_variable(atom: &NormAtom, variable: usize) -> bool {
    atom.fields
        .iter()
        .any(|field| matches!(field.term, NormTerm::Var(id) if id.0 as usize == variable))
}

pub(super) fn atom_variables(atom: &NormAtom) -> BTreeSet<usize> {
    atom.fields
        .iter()
        .filter_map(|field| match field.term {
            NormTerm::Var(variable) => Some(variable.0 as usize),
            NormTerm::Input(_) | NormTerm::Literal(_) | NormTerm::Wildcard => None,
        })
        .collect()
}
