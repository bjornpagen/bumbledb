#![allow(dead_code)]

use std::collections::BTreeSet;

use crate::query::model::{AtomOccurrenceId, NormalizedQuery};

/// Internal binary join plan over atom occurrences.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BinaryPlan {
    /// Base atom occurrence leaf.
    Leaf(AtomOccurrenceId),
    /// Binary join node.
    Join {
        /// Left child.
        left: Box<BinaryPlan>,
        /// Right child.
        right: Box<BinaryPlan>,
        /// Variables shared by both children.
        join_vars: Vec<usize>,
        /// Variables emitted by this binary subplan.
        output_vars: Vec<usize>,
    },
}

/// Ordered source in one decomposed left-deep plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LeftDeepSource {
    /// Atom occurrence source.
    Atom(AtomOccurrenceId),
    /// Materialized output of an earlier decomposed subplan.
    MaterializedSubplan(usize),
}

/// Left-deep plan after bushy decomposition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LeftDeepPlan {
    /// Ordered left-deep sources.
    pub(crate) sources: Vec<LeftDeepSource>,
}

/// Ordered decomposition of a possibly bushy binary plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DecomposedBinaryPlans {
    /// Materialized subplans followed by the root left-deep plan.
    pub(crate) plans: Vec<LeftDeepPlan>,
}

/// Binary plan validation failure.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum BinaryPlanError {
    /// Query had no atom occurrences.
    #[error("cannot plan an empty query")]
    EmptyQuery,
    /// Leaf references an unknown atom occurrence.
    #[error("unknown atom occurrence {atom:?}")]
    UnknownLeaf { atom: AtomOccurrenceId },
    /// The same atom occurrence appears more than once.
    #[error("duplicate atom occurrence leaf {atom:?}")]
    DuplicateLeaf { atom: AtomOccurrenceId },
    /// A query atom occurrence is absent from the plan.
    #[error("missing atom occurrence leaf {atom:?}")]
    MissingLeaf { atom: AtomOccurrenceId },
    /// Join variables are not exactly available from both children.
    #[error("invalid join variable {variable}")]
    InvalidJoinVariable { variable: usize },
    /// Output variable is not provided by either child.
    #[error("disconnected output variable {variable}")]
    DisconnectedOutputVariable { variable: usize },
    /// Output variables do not include all child output variables.
    #[error("binary join output omits variable {variable}")]
    MissingOutputVariable { variable: usize },
}

impl BinaryPlan {
    /// Creates a leaf plan.
    pub(crate) fn leaf(atom: usize) -> Self {
        Self::Leaf(AtomOccurrenceId(atom))
    }

    /// Creates a join plan from child plans and declared variable metadata.
    pub(crate) fn join(
        left: BinaryPlan,
        right: BinaryPlan,
        join_vars: impl IntoIterator<Item = usize>,
        output_vars: impl IntoIterator<Item = usize>,
    ) -> Self {
        Self::Join {
            left: Box::new(left),
            right: Box::new(right),
            join_vars: join_vars.into_iter().collect(),
            output_vars: output_vars.into_iter().collect(),
        }
    }

    /// Validates this binary plan against a normalized query.
    pub(crate) fn validate(&self, query: &NormalizedQuery) -> Result<(), BinaryPlanError> {
        let mut seen = BTreeSet::new();
        self.validate_inner(query, &mut seen)?;
        for atom in &query.atoms {
            if !seen.contains(&atom.id) {
                return Err(BinaryPlanError::MissingLeaf { atom: atom.id });
            }
        }
        Ok(())
    }

    /// Returns the left-deep source sequence when the plan is left-deep.
    pub(crate) fn left_deep_sources(&self) -> Vec<LeftDeepSource> {
        let mut sources = Vec::new();
        self.collect_left_deep_sources(&mut sources);
        sources
    }

    /// Decomposes a possibly bushy binary plan into left-deep plans.
    pub(crate) fn decompose(&self) -> DecomposedBinaryPlans {
        let mut plans = Vec::new();
        let root_sources = self.decompose_sources(&mut plans);
        plans.push(LeftDeepPlan {
            sources: root_sources,
        });
        DecomposedBinaryPlans { plans }
    }

    fn validate_inner(
        &self,
        query: &NormalizedQuery,
        seen: &mut BTreeSet<AtomOccurrenceId>,
    ) -> Result<BTreeSet<usize>, BinaryPlanError> {
        match self {
            BinaryPlan::Leaf(atom) => {
                let occurrence = query
                    .atoms
                    .get(atom.0)
                    .ok_or(BinaryPlanError::UnknownLeaf { atom: *atom })?;
                if !seen.insert(*atom) {
                    return Err(BinaryPlanError::DuplicateLeaf { atom: *atom });
                }
                Ok(occurrence.variable_tuple.iter().copied().collect())
            }
            BinaryPlan::Join {
                left,
                right,
                join_vars,
                output_vars,
            } => {
                let left_vars = left.validate_inner(query, seen)?;
                let right_vars = right.validate_inner(query, seen)?;
                let child_vars: BTreeSet<_> = left_vars.union(&right_vars).copied().collect();
                for variable in join_vars {
                    if !left_vars.contains(variable) || !right_vars.contains(variable) {
                        return Err(BinaryPlanError::InvalidJoinVariable {
                            variable: *variable,
                        });
                    }
                }
                let declared_output: BTreeSet<_> = output_vars.iter().copied().collect();
                for variable in &declared_output {
                    if !child_vars.contains(variable) {
                        return Err(BinaryPlanError::DisconnectedOutputVariable {
                            variable: *variable,
                        });
                    }
                }
                for variable in &child_vars {
                    if !declared_output.contains(variable) {
                        return Err(BinaryPlanError::MissingOutputVariable {
                            variable: *variable,
                        });
                    }
                }
                Ok(declared_output)
            }
        }
    }

    fn collect_left_deep_sources(&self, sources: &mut Vec<LeftDeepSource>) {
        match self {
            BinaryPlan::Leaf(atom) => sources.push(LeftDeepSource::Atom(*atom)),
            BinaryPlan::Join { left, right, .. } => {
                left.collect_left_deep_sources(sources);
                right.collect_left_deep_sources(sources);
            }
        }
    }

    fn decompose_sources(&self, plans: &mut Vec<LeftDeepPlan>) -> Vec<LeftDeepSource> {
        match self {
            BinaryPlan::Leaf(atom) => vec![LeftDeepSource::Atom(*atom)],
            BinaryPlan::Join { left, right, .. } => {
                let mut sources = left.decompose_sources(plans);
                match right.as_ref() {
                    BinaryPlan::Leaf(atom) => sources.push(LeftDeepSource::Atom(*atom)),
                    _ => {
                        let subplan_sources = right.decompose_sources(plans);
                        let subplan_id = plans.len();
                        plans.push(LeftDeepPlan {
                            sources: subplan_sources,
                        });
                        sources.push(LeftDeepSource::MaterializedSubplan(subplan_id));
                    }
                }
                sources
            }
        }
    }
}

/// Builds a deterministic atom-order left-deep binary plan.
pub(crate) fn deterministic_binary_plan(
    query: &NormalizedQuery,
) -> Result<BinaryPlan, BinaryPlanError> {
    let mut atoms = query.atoms.iter();
    let first = atoms.next().ok_or(BinaryPlanError::EmptyQuery)?;
    let mut plan = BinaryPlan::Leaf(first.id);
    let mut output_vars: BTreeSet<_> = first.variable_tuple.iter().copied().collect();

    for atom in atoms {
        let right_vars: BTreeSet<_> = atom.variable_tuple.iter().copied().collect();
        let join_vars: Vec<_> = output_vars.intersection(&right_vars).copied().collect();
        output_vars.extend(right_vars.iter().copied());
        plan = BinaryPlan::join(
            plan,
            BinaryPlan::Leaf(atom.id),
            join_vars,
            output_vars.iter().copied(),
        );
    }

    Ok(plan)
}

#[cfg(test)]
#[path = "planner_tests.rs"]
mod tests;
