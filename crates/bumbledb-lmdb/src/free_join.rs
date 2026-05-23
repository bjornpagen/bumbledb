use crate::{Error, FieldId, RelationId, Result};

/// Free Join physical plan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FreeJoinPlan {
    /// Ordered physical nodes.
    pub nodes: Vec<PlanNode>,
    /// Output/projection/aggregation plan.
    pub output: OutputPlan,
}

impl FreeJoinPlan {
    /// Validates local structural invariants of this plan.
    pub fn validate(&self) -> Result<()> {
        for (expected, node) in self.nodes.iter().enumerate() {
            if node.id.0 as usize != expected {
                return Err(Error::internal(
                    "free join node ids must be dense and ordered",
                ));
            }
            if node.bind_vars.len() != 1 {
                return Err(Error::internal(
                    "free join nodes must bind exactly one variable",
                ));
            }
            for subatom in &node.subatoms {
                for variable in &subatom.vars {
                    if !node.bind_vars.iter().any(|bound| bound == variable) {
                        return Err(Error::internal(format!(
                            "subatom variable {} is not bound by node {}",
                            variable.0, node.id.0
                        )));
                    }
                }
                if subatom.fields.len() != subatom.vars.len() {
                    return Err(Error::internal("subatom fields and vars length mismatch"));
                }
            }
        }
        Ok(())
    }
}

/// One physical Free Join node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanNode {
    /// Dense node ID.
    pub id: NodeId,
    /// Variables bound by this node.
    pub bind_vars: Vec<VarId>,
    /// Subatoms consumed by this node.
    pub subatoms: Vec<SubAtom>,
}

/// Subatom partition inside a Free Join node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubAtom {
    /// Original atom ID in query clause order among relation atoms.
    pub atom_id: AtomId,
    /// Relation ID.
    pub relation: RelationId,
    /// Fields used by this subatom.
    pub fields: Vec<FieldId>,
    /// Variables corresponding to `fields`.
    pub vars: Vec<VarId>,
}

/// Output/projection/aggregation plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutputPlan {
    /// Projection with set semantics.
    Project(ProjectPlan),
}

impl Default for OutputPlan {
    fn default() -> Self {
        OutputPlan::Project(ProjectPlan::default())
    }
}

/// Projection output plan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectPlan {
    /// Projected variables in output order.
    pub vars: Vec<VarId>,
}

/// Dense variable ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VarId(pub u16);

/// Dense node ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u16);

/// Dense atom ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AtomId(pub u16);

/// Dense physical access ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccessId(pub u16);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_manual_lftj_plan() -> Result<()> {
        let plan = FreeJoinPlan {
            nodes: vec![PlanNode {
                id: NodeId(0),
                bind_vars: vec![VarId(0)],
                subatoms: vec![SubAtom {
                    atom_id: AtomId(0),
                    relation: RelationId(0),
                    fields: vec![FieldId(0)],
                    vars: vec![VarId(0)],
                }],
            }],
            output: OutputPlan::Project(ProjectPlan {
                vars: vec![VarId(0)],
            }),
        };

        plan.validate()?;
        Ok(())
    }

    #[test]
    fn rejects_subatom_vars_not_bound_by_node() {
        let plan = FreeJoinPlan {
            nodes: vec![PlanNode {
                id: NodeId(0),
                bind_vars: vec![VarId(0)],
                subatoms: vec![SubAtom {
                    atom_id: AtomId(0),
                    relation: RelationId(0),
                    fields: vec![FieldId(0)],
                    vars: vec![VarId(1)],
                }],
            }],
            output: OutputPlan::default(),
        };

        assert!(plan.validate().is_err());
    }

    #[test]
    fn rejects_multi_variable_nodes() {
        let plan = FreeJoinPlan {
            nodes: vec![PlanNode {
                id: NodeId(0),
                bind_vars: vec![VarId(0), VarId(1)],
                subatoms: Vec::new(),
            }],
            output: OutputPlan::default(),
        };

        assert!(plan.validate().is_err());
    }
}
