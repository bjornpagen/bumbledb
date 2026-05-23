use std::collections::{BTreeMap, BTreeSet};

use super::*;

pub(in crate::query::tests) struct ReferenceDb {
    facts: BTreeMap<String, Vec<Fact>>,
}

#[derive(Clone, Debug)]
struct ReferenceBinding {
    values: Vec<Option<Value>>,
}

impl ReferenceBinding {
    fn new(variable_count: usize) -> Self {
        Self {
            values: vec![None; variable_count],
        }
    }

    fn get(&self, variable: usize) -> Option<&Value> {
        self.values[variable].as_ref()
    }

    fn bind(&mut self, variable: usize, value: Value) -> bool {
        match &self.values[variable] {
            Some(existing) => existing == &value,
            None => {
                self.values[variable] = Some(value);
                true
            }
        }
    }
}

impl ReferenceDb {
    pub(in crate::query::tests) fn from_facts(facts: Vec<Fact>) -> Self {
        let mut by_relation: BTreeMap<String, Vec<Fact>> = BTreeMap::new();
        for fact in facts {
            by_relation
                .entry(fact.relation().to_owned())
                .or_default()
                .push(fact);
        }
        Self { facts: by_relation }
    }

    pub(in crate::query::tests) fn execute(
        &self,
        query: &TypedQuery,
        inputs: &InputBindings,
    ) -> Result<Vec<Vec<Value>>> {
        let atoms = query
            .clauses
            .iter()
            .filter_map(|clause| match clause {
                TypedClause::Relation(atom) => Some(atom),
                TypedClause::Comparison(_) => None,
            })
            .collect::<Vec<_>>();
        let comparisons = query
            .clauses
            .iter()
            .filter_map(|clause| match clause {
                TypedClause::Comparison(comparison) => Some(comparison),
                TypedClause::Relation(_) => None,
            })
            .collect::<Vec<_>>();
        let mut output = Vec::new();
        let mut counters = PlanCounters::default();
        self.recurse(
            query,
            inputs,
            &atoms,
            &comparisons,
            0,
            ReferenceBinding::new(query.variables.len()),
            &mut output,
            &mut counters,
        )?;
        reference_project_results(query, &output)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "test reference recursion carries explicit evaluator state"
    )]
    fn recurse(
        &self,
        query: &TypedQuery,
        inputs: &InputBindings,
        atoms: &[&TypedRelationAtom],
        comparisons: &[&TypedComparison],
        depth: usize,
        binding: ReferenceBinding,
        output: &mut Vec<ReferenceBinding>,
        counters: &mut PlanCounters,
    ) -> Result<()> {
        if depth == atoms.len() {
            if reference_comparisons_pass(comparisons, query, inputs, &binding, counters)? {
                output.push(binding);
            }
            return Ok(());
        }

        let atom = atoms[depth];
        for fact in self.facts.get(&atom.relation).into_iter().flatten() {
            let Some(next) = reference_match_atom(atom, query, inputs, &binding, fact)? else {
                continue;
            };
            if reference_comparisons_pass(comparisons, query, inputs, &next, counters)? {
                self.recurse(
                    query,
                    inputs,
                    atoms,
                    comparisons,
                    depth + 1,
                    next,
                    output,
                    counters,
                )?;
            }
        }
        Ok(())
    }
}

fn reference_match_atom(
    atom: &TypedRelationAtom,
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &ReferenceBinding,
    fact: &Fact,
) -> Result<Option<ReferenceBinding>> {
    let mut next = binding.clone();
    for field in &atom.fields {
        let Some(fact_value) = fact.value(&field.field) else {
            return Ok(None);
        };
        match &field.term {
            TypedTerm::Variable(variable) => {
                let normalized =
                    reference_value_for_type(fact_value, &query.variables[*variable].value_type);
                if !next.bind(*variable, normalized) {
                    return Ok(None);
                }
            }
            TypedTerm::Input(input) => {
                let input_value = reference_input_value(query, inputs, *input)?;
                let normalized =
                    reference_value_for_type(fact_value, &query.inputs[*input].value_type);
                if input_value != &normalized {
                    return Ok(None);
                }
            }
            TypedTerm::Literal(literal) => {
                let normalized = reference_value_for_type(fact_value, &literal.value_type);
                if literal_to_value(literal)? != normalized {
                    return Ok(None);
                }
            }
            TypedTerm::Wildcard => {}
        }
    }
    Ok(Some(next))
}

fn reference_comparisons_pass(
    comparisons: &[&TypedComparison],
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &ReferenceBinding,
    counters: &mut PlanCounters,
) -> Result<bool> {
    for comparison in comparisons {
        let Some(left) = reference_operand_value(&comparison.left, query, inputs, binding)? else {
            continue;
        };
        let Some(right) = reference_operand_value(&comparison.right, query, inputs, binding)?
        else {
            continue;
        };
        counters.comparisons_evaluated += 1;
        let left = reference_value_for_type(&left, &comparison.value_type);
        let right = reference_value_for_type(&right, &comparison.value_type);
        if !compare_values(&left, comparison.operator, &right) {
            counters.comparisons_failed += 1;
            return Ok(false);
        }
    }
    Ok(true)
}

fn reference_input_value<'a>(
    query: &'a TypedQuery,
    inputs: &'a InputBindings,
    input: usize,
) -> Result<&'a Value> {
    let input = &query.inputs[input];
    inputs
        .get(&input.name)
        .ok_or_else(|| Error::missing_input(&input.name))
}

fn reference_operand_value(
    operand: &TypedOperand,
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &ReferenceBinding,
) -> Result<Option<Value>> {
    Ok(match operand {
        TypedOperand::Variable(variable) => binding.get(*variable).cloned(),
        TypedOperand::Input(input) => Some(reference_input_value(query, inputs, *input)?.clone()),
        TypedOperand::Literal(literal) => Some(literal_to_value(literal)?),
    })
}

fn reference_value_for_type(value: &Value, _value_type: &ValueType) -> Value {
    value.clone()
}

fn reference_project_results(
    query: &TypedQuery,
    bindings: &[ReferenceBinding],
) -> Result<Vec<Vec<Value>>> {
    let mut set = BTreeSet::new();
    for binding in bindings {
        let mut fact = Vec::new();
        for term in &query.find {
            let TypedFindTerm::Variable { variable } = term;
            fact.push(reference_bound_variable(binding, *variable)?.clone());
        }
        set.insert(fact);
    }
    Ok(set.into_iter().collect())
}

fn reference_bound_variable(binding: &ReferenceBinding, variable: usize) -> Result<&Value> {
    binding
        .get(variable)
        .ok_or_else(|| Error::internal(format!("variable {variable} is unbound at projection")))
}
