#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

use crate::query::model::{AtomOccurrenceId, NormalizedQuery, NormalizedTerm};

/// Ordered subset of one atom occurrence in a formal Free Join node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FjSubatom {
    /// Atom occurrence referenced by this subatom.
    pub(crate) atom: AtomOccurrenceId,
    /// Ordered variables in this subatom.
    pub(crate) vars: Vec<usize>,
    /// Field IDs corresponding positionally to `vars`.
    pub(crate) field_ids: Vec<usize>,
}

/// One formal Free Join node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FjNode {
    /// Dense node ID.
    pub(crate) id: usize,
    /// Subatoms in node order.
    pub(crate) subatoms: Vec<FjSubatom>,
}

/// Formal Free Join plan shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FjPlan {
    /// Nodes in execution order.
    pub(crate) nodes: Vec<FjNode>,
    /// Number of query variables available to validation.
    pub(crate) query_variables: usize,
}

/// Validated formal plan with derived node metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ValidatedFjPlan {
    /// Validated nodes with available/new variables and covers.
    pub(crate) nodes: Vec<ValidatedFjNode>,
    /// Per-atom subatom partitions.
    pub(crate) atom_partitions: BTreeMap<AtomOccurrenceId, Vec<FjSubatom>>,
    /// Number of query variables.
    pub(crate) query_variables: usize,
}

/// Validated node metadata derived from earlier nodes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ValidatedFjNode {
    /// Dense node ID.
    pub(crate) id: usize,
    /// Node subatoms.
    pub(crate) subatoms: Vec<FjSubatom>,
    /// Variables available before this node.
    pub(crate) available_vars: Vec<usize>,
    /// Variables introduced by this node.
    pub(crate) new_vars: Vec<usize>,
    /// Cover candidates for this node.
    pub(crate) covers: Vec<FjCoverCandidate>,
}

/// One valid cover candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FjCoverCandidate {
    /// Node ID.
    pub(crate) node: usize,
    /// Subatom index within the node.
    pub(crate) subatom: usize,
    /// Cover variable tuple.
    pub(crate) vars: Vec<usize>,
}

/// Formal Free Join plan validation failure.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum FjPlanError {
    /// Node IDs must be dense in plan order.
    #[error("node id {actual} is not dense; expected {expected}")]
    NodeIdNotDense { expected: usize, actual: usize },
    /// A subatom referenced an unknown atom occurrence.
    #[error("unknown atom occurrence {atom:?}")]
    UnknownAtom { atom: AtomOccurrenceId },
    /// A subatom referenced a variable outside the query.
    #[error("unknown variable {variable} in atom {atom:?}")]
    UnknownVariable {
        atom: AtomOccurrenceId,
        variable: usize,
    },
    /// A subatom referenced a variable not in its atom occurrence.
    #[error("variable {variable} is not in atom {atom:?}")]
    VariableOutsideAtom {
        atom: AtomOccurrenceId,
        variable: usize,
    },
    /// A subatom listed the same variable twice.
    #[error("duplicate variable {variable} in subatom for atom {atom:?}")]
    DuplicateSubatomVariable {
        atom: AtomOccurrenceId,
        variable: usize,
    },
    /// A field ID did not belong to the atom occurrence.
    #[error("field {field_id} is not in atom {atom:?}")]
    FieldOutsideAtom {
        atom: AtomOccurrenceId,
        field_id: usize,
    },
    /// The variable and field tuples did not have equal lengths.
    #[error("subatom for atom {atom:?} has mismatched vars and fields")]
    FieldArityMismatch { atom: AtomOccurrenceId },
    /// A field did not bind the expected variable.
    #[error("field {field_id} in atom {atom:?} does not bind variable {variable}")]
    FieldVariableMismatch {
        atom: AtomOccurrenceId,
        field_id: usize,
        variable: usize,
    },
    /// An atom variable appeared in more than one partition subatom.
    #[error("variable {variable} in atom {atom:?} appears in multiple partition subatoms")]
    DuplicatePartitionVariable {
        atom: AtomOccurrenceId,
        variable: usize,
    },
    /// An atom occurrence did not have a complete partition.
    #[error("atom {atom:?} partition is incomplete")]
    MissingPartition { atom: AtomOccurrenceId },
    /// A node contained the same atom occurrence twice.
    #[error("node {node} contains atom {atom:?} more than once")]
    DuplicateAtomInNode { node: usize, atom: AtomOccurrenceId },
    /// A node has no executable cover.
    #[error("node {node} has no cover")]
    MissingCover { node: usize },
    /// A probe subatom needs a variable unavailable to the current cover.
    #[error("node {node} subatom {subatom} probes unavailable variable {variable}")]
    UnavailableProbeVariable {
        node: usize,
        subatom: usize,
        variable: usize,
    },
}

impl FjPlan {
    /// Validates this formal plan against a normalized query.
    pub(crate) fn validate(
        &self,
        query: &NormalizedQuery,
    ) -> std::result::Result<ValidatedFjPlan, FjPlanError> {
        if self.query_variables != query.variables.len() {
            return Err(FjPlanError::UnknownVariable {
                atom: AtomOccurrenceId(0),
                variable: self.query_variables,
            });
        }

        let mut assigned: BTreeMap<AtomOccurrenceId, BTreeSet<usize>> = BTreeMap::new();
        let mut partitions: BTreeMap<AtomOccurrenceId, Vec<FjSubatom>> = BTreeMap::new();
        let mut available = BTreeSet::new();
        let mut validated_nodes = Vec::new();

        for (expected_id, node) in self.nodes.iter().enumerate() {
            if node.id != expected_id {
                return Err(FjPlanError::NodeIdNotDense {
                    expected: expected_id,
                    actual: node.id,
                });
            }
            let mut atoms_in_node = BTreeSet::new();
            let mut node_vars = BTreeSet::new();
            for subatom in &node.subatoms {
                validate_subatom(query, subatom)?;
                if !atoms_in_node.insert(subatom.atom) {
                    return Err(FjPlanError::DuplicateAtomInNode {
                        node: node.id,
                        atom: subatom.atom,
                    });
                }
                for variable in &subatom.vars {
                    node_vars.insert(*variable);
                    let atom_vars = assigned.entry(subatom.atom).or_default();
                    if !atom_vars.insert(*variable) {
                        return Err(FjPlanError::DuplicatePartitionVariable {
                            atom: subatom.atom,
                            variable: *variable,
                        });
                    }
                }
                partitions
                    .entry(subatom.atom)
                    .or_default()
                    .push(subatom.clone());
            }

            let new_vars: BTreeSet<_> = node_vars.difference(&available).copied().collect();
            let covers = covers_for_node(node, &new_vars);
            if node.subatoms.is_empty() || covers.is_empty() {
                if let Some(error) = unavailable_probe_error(node, &available) {
                    return Err(error);
                }
                return Err(FjPlanError::MissingCover { node: node.id });
            }

            validated_nodes.push(ValidatedFjNode {
                id: node.id,
                subatoms: node.subatoms.clone(),
                available_vars: available.iter().copied().collect(),
                new_vars: new_vars.iter().copied().collect(),
                covers,
            });
            available.extend(node_vars);
        }

        validate_partitions(query, &assigned, &partitions)?;

        Ok(ValidatedFjPlan {
            nodes: validated_nodes,
            atom_partitions: partitions,
            query_variables: self.query_variables,
        })
    }
}

fn validate_subatom(query: &NormalizedQuery, subatom: &FjSubatom) -> Result<(), FjPlanError> {
    let atom = query
        .atoms
        .get(subatom.atom.0)
        .ok_or(FjPlanError::UnknownAtom { atom: subatom.atom })?;
    if subatom.vars.len() != subatom.field_ids.len() {
        return Err(FjPlanError::FieldArityMismatch { atom: subatom.atom });
    }

    let atom_vars: BTreeSet<_> = atom.variable_tuple.iter().copied().collect();
    let mut seen_vars = BTreeSet::new();
    for (variable, field_id) in subatom.vars.iter().zip(&subatom.field_ids) {
        if *variable >= query.variables.len() {
            return Err(FjPlanError::UnknownVariable {
                atom: subatom.atom,
                variable: *variable,
            });
        }
        if !atom_vars.contains(variable) {
            return Err(FjPlanError::VariableOutsideAtom {
                atom: subatom.atom,
                variable: *variable,
            });
        }
        if !seen_vars.insert(*variable) {
            return Err(FjPlanError::DuplicateSubatomVariable {
                atom: subatom.atom,
                variable: *variable,
            });
        }
        let field = atom
            .fields
            .get(*field_id)
            .ok_or(FjPlanError::FieldOutsideAtom {
                atom: subatom.atom,
                field_id: *field_id,
            })?;
        if field.term != NormalizedTerm::Variable(*variable) {
            return Err(FjPlanError::FieldVariableMismatch {
                atom: subatom.atom,
                field_id: *field_id,
                variable: *variable,
            });
        }
    }
    Ok(())
}

fn covers_for_node(node: &FjNode, new_vars: &BTreeSet<usize>) -> Vec<FjCoverCandidate> {
    node.subatoms
        .iter()
        .enumerate()
        .filter_map(|(index, subatom)| {
            let vars: BTreeSet<_> = subatom.vars.iter().copied().collect();
            if new_vars.is_subset(&vars) {
                Some(FjCoverCandidate {
                    node: node.id,
                    subatom: index,
                    vars: subatom.vars.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn unavailable_probe_error(node: &FjNode, available: &BTreeSet<usize>) -> Option<FjPlanError> {
    let candidate_cover: BTreeSet<_> = node
        .subatoms
        .first()
        .map(|subatom| subatom.vars.iter().copied().collect())
        .unwrap_or_default();
    for (index, subatom) in node.subatoms.iter().enumerate().skip(1) {
        for variable in &subatom.vars {
            if !available.contains(variable) && !candidate_cover.contains(variable) {
                return Some(FjPlanError::UnavailableProbeVariable {
                    node: node.id,
                    subatom: index,
                    variable: *variable,
                });
            }
        }
    }
    None
}

fn validate_partitions(
    query: &NormalizedQuery,
    assigned: &BTreeMap<AtomOccurrenceId, BTreeSet<usize>>,
    partitions: &BTreeMap<AtomOccurrenceId, Vec<FjSubatom>>,
) -> Result<(), FjPlanError> {
    for atom in &query.atoms {
        let expected: BTreeSet<_> = atom.variable_tuple.iter().copied().collect();
        let actual = assigned.get(&atom.id).cloned().unwrap_or_default();
        let has_subatom = partitions
            .get(&atom.id)
            .is_some_and(|parts| !parts.is_empty());
        if expected != actual || (expected.is_empty() && !has_subatom) {
            return Err(FjPlanError::MissingPartition { atom: atom.id });
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "free_join_tests.rs"]
mod tests;
