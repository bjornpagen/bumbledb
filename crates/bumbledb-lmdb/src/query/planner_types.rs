use super::*;

#[derive(Clone, Debug)]
pub(super) struct PlannerStats {
    relations: BTreeMap<String, Arc<PlannerRelationStats>>,
}

impl PlannerStats {
    pub(super) fn collect(
        schema: &StorageSchema,
        image: &crate::QueryImage,
        atoms: &[&NormAtom],
    ) -> Result<Self> {
        let mut relations = BTreeMap::new();
        for atom in atoms {
            if relations.contains_key(&atom.relation_name) {
                continue;
            }
            let relation = image
                .relation_by_id(atom.relation)
                .ok_or_else(|| Error::unknown_relation(&atom.relation_name))?;
            relations.insert(
                atom.relation_name.clone(),
                image.planner_relation_stats(schema, relation)?,
            );
        }
        Ok(Self { relations })
    }

    pub(super) fn relation_facts(&self, relation: &str) -> u64 {
        self.relations
            .get(relation)
            .map(|stats| stats.facts as u64)
            .unwrap_or(1)
            .max(1)
    }

    pub(super) fn index_stats(&self, relation: &str, index: &str) -> Option<&PlannerIndexStats> {
        self.relations.get(relation)?.indexes.get(index)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct VariableOrderScore {
    pub(super) variable: usize,
    pub(super) field_position: usize,
    pub(super) candidate_estimate: u64,
    pub(super) static_constraints: usize,
    pub(super) bound_constraints: usize,
    pub(super) relation_constraints: usize,
    pub(super) degree: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct VariableAccessScore {
    pub(super) relation: String,
    pub(super) index: String,
    pub(super) fact_estimate: u64,
    pub(super) prefix_len: usize,
    pub(super) current_is_next: bool,
}

impl VariableAccessScore {
    pub(super) fn access_label(&self) -> String {
        format!("{}.{}", self.relation, self.index)
    }
}
