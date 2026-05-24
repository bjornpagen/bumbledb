#![allow(dead_code)]

use crate::query::model::{AtomOccurrenceId, NormalizedQuery, NormalizedTerm};

#[path = "free_join/compact.rs"]
mod compact;
pub(crate) use compact::{AtomPartitionCount, CoverList, IdList, IdRange, NodeList, SubatomList};

/// Ordered subset of one atom occurrence in a formal Free Join node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FjSubatom {
    /// Atom occurrence referenced by this subatom.
    pub(crate) atom: AtomOccurrenceId,
    /// Ordered variables in this subatom.
    pub(crate) vars: IdList,
    /// Field IDs corresponding positionally to `vars`.
    pub(crate) field_ids: IdList,
}

/// One formal Free Join node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FjNode {
    /// Dense node ID.
    pub(crate) id: usize,
    /// Subatoms in node order.
    pub(crate) subatoms: SubatomList,
}

/// Formal Free Join plan shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FjPlan {
    /// Nodes in execution order.
    pub(crate) nodes: NodeList,
    /// Number of query variables available to validation.
    pub(crate) query_variables: usize,
}

/// Validated formal plan with derived node metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ValidatedFjPlan {
    pub(crate) nodes: Box<[ValidatedFjNode]>,
    subatoms: Box<[ValidatedFjSubatom]>,
    vars: Box<[usize]>,
    field_ids: Box<[usize]>,
    covers: Box<[FjCoverCandidate]>,
    pub(crate) atom_partitions: AtomPartitionCount,
    pub(crate) query_variables: usize,
}

/// Validated node metadata derived from earlier nodes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ValidatedFjNode {
    pub(crate) id: usize,
    pub(crate) subatoms: IdRange,
    pub(crate) available_vars: IdRange,
    pub(crate) new_vars: IdRange,
    pub(crate) covers: IdRange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ValidatedFjSubatom {
    pub(crate) atom: AtomOccurrenceId,
    pub(crate) vars: IdRange,
    pub(crate) field_ids: IdRange,
}

/// One valid cover candidate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FjCoverCandidate {
    pub(crate) node: usize,
    pub(crate) subatom: usize,
    pub(crate) vars: IdRange,
}

impl ValidatedFjPlan {
    pub(crate) fn node_subatoms(&self, node: &ValidatedFjNode) -> &[ValidatedFjSubatom] {
        range_slice(&self.subatoms, node.subatoms)
    }

    pub(crate) fn node_covers(&self, node: &ValidatedFjNode) -> &[FjCoverCandidate] {
        range_slice(&self.covers, node.covers)
    }

    pub(crate) fn node_available_vars(&self, node: &ValidatedFjNode) -> &[usize] {
        range_slice(&self.vars, node.available_vars)
    }

    pub(crate) fn node_new_vars(&self, node: &ValidatedFjNode) -> &[usize] {
        range_slice(&self.vars, node.new_vars)
    }

    pub(crate) fn subatom_at(
        &self,
        node: &ValidatedFjNode,
        index: usize,
    ) -> Option<&ValidatedFjSubatom> {
        self.node_subatoms(node).get(index)
    }

    pub(crate) fn subatom_vars(&self, subatom: &ValidatedFjSubatom) -> &[usize] {
        range_slice(&self.vars, subatom.vars)
    }

    pub(crate) fn subatom_field_ids(&self, subatom: &ValidatedFjSubatom) -> &[usize] {
        range_slice(&self.field_ids, subatom.field_ids)
    }
}

fn range_slice<T>(values: &[T], range: IdRange) -> &[T] {
    &values[range.start..range.start + range.len]
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct VarSet {
    words: [u64; 4],
}

impl VarSet {
    fn insert(&mut self, variable: usize) -> bool {
        let word = variable / 64;
        let Some(slot) = self.words.get_mut(word) else {
            return false;
        };
        let mask = 1u64 << (variable % 64);
        let already = *slot & mask != 0;
        *slot |= mask;
        !already
    }

    fn contains(self, variable: usize) -> bool {
        self.words
            .get(variable / 64)
            .is_some_and(|word| word & (1u64 << (variable % 64)) != 0)
    }

    fn union_assign(&mut self, other: Self) {
        for (left, right) in self.words.iter_mut().zip(other.words) {
            *left |= right;
        }
    }

    fn difference(self, other: Self) -> Self {
        let mut out = Self::default();
        for ((out, left), right) in out.words.iter_mut().zip(self.words).zip(other.words) {
            *out = left & !right;
        }
        out
    }

    fn is_empty(self) -> bool {
        self.words.iter().all(|word| *word == 0)
    }

    fn is_subset_of(self, other: Self) -> bool {
        self.words
            .iter()
            .zip(other.words)
            .all(|(left, right)| *left & !right == 0)
    }

    fn iter(self) -> VarSetIter {
        VarSetIter { set: self, next: 0 }
    }
}

struct VarSetIter {
    set: VarSet,
    next: usize,
}

impl Iterator for VarSetIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while self.next < self.set.words.len() * 64 {
            let variable = self.next;
            self.next += 1;
            if self.set.contains(variable) {
                return Some(variable);
            }
        }
        None
    }
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

        let mut assigned = vec![VarSet::default(); query.atoms.len()];
        let mut has_partition = vec![false; query.atoms.len()];
        let mut available = VarSet::default();
        let mut validated_nodes = Vec::new();
        let mut validated_subatoms = Vec::new();
        let mut vars = Vec::new();
        let mut field_ids = Vec::new();
        let mut validated_covers = Vec::new();

        for (expected_id, node) in self.nodes.iter().enumerate() {
            if node.id != expected_id {
                return Err(FjPlanError::NodeIdNotDense {
                    expected: expected_id,
                    actual: node.id,
                });
            }
            let mut atoms_in_node = vec![false; query.atoms.len()];
            let mut node_vars = VarSet::default();
            for subatom in &node.subatoms {
                validate_subatom(query, subatom)?;
                if atoms_in_node[subatom.atom.0] {
                    return Err(FjPlanError::DuplicateAtomInNode {
                        node: node.id,
                        atom: subatom.atom,
                    });
                }
                atoms_in_node[subatom.atom.0] = true;
                has_partition[subatom.atom.0] = true;
                for variable in &subatom.vars {
                    node_vars.insert(*variable);
                    if !assigned[subatom.atom.0].insert(*variable) {
                        return Err(FjPlanError::DuplicatePartitionVariable {
                            atom: subatom.atom,
                            variable: *variable,
                        });
                    }
                }
            }

            let new_vars = node_vars.difference(available);
            let covers = covers_for_node(node, &new_vars);
            if node.subatoms.is_empty() || covers.is_empty() {
                if let Some(error) = unavailable_probe_error(node, &available) {
                    return Err(error);
                }
                return Err(FjPlanError::MissingCover { node: node.id });
            }

            let subatom_start = validated_subatoms.len();
            for subatom in &node.subatoms {
                let vars_start = vars.len();
                vars.extend(subatom.vars.iter().copied());
                let field_start = field_ids.len();
                field_ids.extend(subatom.field_ids.iter().copied());
                validated_subatoms.push(ValidatedFjSubatom {
                    atom: subatom.atom,
                    vars: IdRange::new(vars_start, subatom.vars.len()),
                    field_ids: IdRange::new(field_start, subatom.field_ids.len()),
                });
            }
            let available_start = vars.len();
            vars.extend(available.iter());
            let new_start = vars.len();
            vars.extend(new_vars.iter());
            let cover_start = validated_covers.len();
            for mut cover in covers.iter().copied() {
                let cover_subatom = &node.subatoms[cover.subatom];
                let vars_start = vars.len();
                vars.extend(cover_subatom.vars.iter().copied());
                cover.vars = IdRange::new(vars_start, cover_subatom.vars.len());
                validated_covers.push(cover);
            }

            validated_nodes.push(ValidatedFjNode {
                id: node.id,
                subatoms: IdRange::new(subatom_start, node.subatoms.len()),
                available_vars: IdRange::new(available_start, available.iter().count()),
                new_vars: IdRange::new(new_start, new_vars.iter().count()),
                covers: IdRange::new(cover_start, validated_covers.len() - cover_start),
            });
            available.union_assign(node_vars);
        }

        validate_partitions(query, &assigned, &has_partition)?;

        Ok(ValidatedFjPlan {
            nodes: validated_nodes.into_boxed_slice(),
            subatoms: validated_subatoms.into_boxed_slice(),
            vars: vars.into_boxed_slice(),
            field_ids: field_ids.into_boxed_slice(),
            covers: validated_covers.into_boxed_slice(),
            atom_partitions: AtomPartitionCount(
                has_partition.iter().filter(|value| **value).count(),
            ),
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

    let atom_vars = vars_from_iter(atom.variable_tuple.iter().copied());
    let mut seen_vars = VarSet::default();
    for (variable, field_id) in subatom.vars.iter().zip(&subatom.field_ids) {
        if *variable >= query.variables.len() {
            return Err(FjPlanError::UnknownVariable {
                atom: subatom.atom,
                variable: *variable,
            });
        }
        if !atom_vars.contains(*variable) {
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

fn covers_for_node(node: &FjNode, new_vars: &VarSet) -> CoverList {
    let mut covers = CoverList::new();
    for (index, subatom) in node.subatoms.iter().enumerate() {
        let vars = vars_from_iter(subatom.vars.iter().copied());
        if new_vars.is_subset_of(vars) {
            covers.push(FjCoverCandidate {
                node: node.id,
                subatom: index,
                vars: IdRange::default(),
            });
        }
    }
    covers
}

fn unavailable_probe_error(node: &FjNode, available: &VarSet) -> Option<FjPlanError> {
    let candidate_cover = node
        .subatoms
        .first()
        .map(|subatom| vars_from_iter(subatom.vars.iter().copied()))
        .unwrap_or_default();
    for (index, subatom) in node.subatoms.iter().enumerate().skip(1) {
        for variable in &subatom.vars {
            if !available.contains(*variable) && !candidate_cover.contains(*variable) {
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
    assigned: &[VarSet],
    has_partition: &[bool],
) -> Result<(), FjPlanError> {
    for atom in &query.atoms {
        let expected = vars_from_iter(atom.variable_tuple.iter().copied());
        let actual = assigned.get(atom.id.0).copied().unwrap_or_default();
        let has_subatom = has_partition.get(atom.id.0).copied().unwrap_or_default();
        if expected != actual || (expected.is_empty() && !has_subatom) {
            return Err(FjPlanError::MissingPartition { atom: atom.id });
        }
    }
    Ok(())
}

fn vars_from_iter(vars: impl IntoIterator<Item = usize>) -> VarSet {
    let mut set = VarSet::default();
    for variable in vars {
        set.insert(variable);
    }
    set
}

#[cfg(test)]
#[path = "free_join_tests.rs"]
mod tests;
