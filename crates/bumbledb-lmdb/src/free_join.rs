use bumbledb_core::datalog::AggregateFunction;
use bumbledb_core::schema::ValueType;

use crate::{Error, FieldId, RelationId, Result};

/// Free Join physical plan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FreeJoinPlan {
    /// Ordered physical nodes.
    pub nodes: Vec<PlanNode>,
    /// Output/projection/aggregation plan.
    pub output: OutputPlan,
    /// Planner estimates for this plan.
    pub estimates: PlanEstimates,
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

    /// Returns true when this plan is expressible as pure single-variable LFTJ nodes.
    pub fn is_pure_lftj(&self) -> bool {
        self.nodes.iter().all(|node| {
            node.implementation == NodeImpl::SortedLeapfrog && node.bind_vars.len() == 1
        })
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
    /// Node implementation strategy.
    pub implementation: NodeImpl,
    /// Payload carried by this node.
    pub payload: PayloadDemand,
}

/// Node implementation strategy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeImpl {
    /// Sorted trie leapfrog node.
    SortedLeapfrog,
    /// Hash probe node.
    HashProbe,
    /// Hybrid sorted/hash node.
    Hybrid,
    /// Vector/range loop node.
    VectorLoop,
    /// Existence-only predicate node.
    ExistenceCheck,
    /// Cartesian/product node.
    Product,
    /// Aggregate sink node.
    AggregateSink,
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
    /// Physical access ID. This is index-layout ID for now.
    pub access: AccessId,
}

/// Payload required from a physical node.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PayloadDemand {
    /// Variables needed by final projection.
    pub projected_vars: Vec<VarId>,
    /// Variables needed by aggregate terms.
    pub aggregate_vars: Vec<VarId>,
    /// Relations used only for existence checks.
    pub existence_only_relations: Vec<RelationId>,
    /// Relations whose row IDs are needed by later nodes/output.
    pub row_id_demands: Vec<RelationId>,
}

/// Output/projection/aggregation plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutputPlan {
    /// Projection with set semantics.
    Project(ProjectPlan),
    /// Aggregate output.
    Aggregate(AggregatePlan),
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
    /// True for Datalog set semantics.
    pub set_semantics: bool,
}

/// Aggregate output plan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AggregatePlan {
    /// Group variables in output order.
    pub group_vars: Vec<VarId>,
    /// Aggregate terms in output order.
    pub aggregates: Vec<AggregateTerm>,
}

/// Aggregate term metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AggregateTerm {
    /// Aggregate function.
    pub function: AggregateFunction,
    /// Variable being aggregated.
    pub var: VarId,
    /// Logical aggregate operand type.
    pub value_type: ValueType,
}

/// Planner estimates for one Free Join plan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlanEstimates {
    /// Estimated output rows.
    pub output_rows: u64,
    /// Estimated iterator operations.
    pub iterator_ops: u64,
    /// Estimated hash build rows.
    pub hash_build_rows: u64,
    /// Estimated hash probe rows.
    pub hash_probe_rows: u64,
    /// Estimated materialized logical values.
    pub materialized_values: u64,
    /// Estimated memory bytes.
    pub memory_bytes: usize,
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
    fn validates_manual_lftj_plan() {
        let plan = FreeJoinPlan {
            nodes: vec![PlanNode {
                id: NodeId(0),
                bind_vars: vec![VarId(0)],
                subatoms: vec![SubAtom {
                    atom_id: AtomId(0),
                    relation: RelationId(0),
                    fields: vec![FieldId(0)],
                    vars: vec![VarId(0)],
                    access: AccessId(0),
                }],
                implementation: NodeImpl::SortedLeapfrog,
                payload: PayloadDemand::default(),
            }],
            output: OutputPlan::Project(ProjectPlan {
                vars: vec![VarId(0)],
                set_semantics: true,
            }),
            estimates: PlanEstimates::default(),
        };

        plan.validate().unwrap();
        assert!(plan.is_pure_lftj());
    }

    #[test]
    fn validates_manual_probe_plan_shape() {
        let plan = FreeJoinPlan {
            nodes: vec![PlanNode {
                id: NodeId(0),
                bind_vars: vec![VarId(0), VarId(1)],
                subatoms: vec![SubAtom {
                    atom_id: AtomId(0),
                    relation: RelationId(0),
                    fields: vec![FieldId(0), FieldId(1)],
                    vars: vec![VarId(0), VarId(1)],
                    access: AccessId(0),
                }],
                implementation: NodeImpl::HashProbe,
                payload: PayloadDemand {
                    projected_vars: vec![VarId(0), VarId(1)],
                    ..PayloadDemand::default()
                },
            }],
            output: OutputPlan::Project(ProjectPlan {
                vars: vec![VarId(0), VarId(1)],
                set_semantics: true,
            }),
            estimates: PlanEstimates::default(),
        };

        plan.validate().unwrap();
        assert!(!plan.is_pure_lftj());
    }

    #[test]
    fn validates_manual_hybrid_plan_shape() {
        let plan = FreeJoinPlan {
            nodes: vec![PlanNode {
                id: NodeId(0),
                bind_vars: vec![VarId(0)],
                subatoms: vec![SubAtom {
                    atom_id: AtomId(0),
                    relation: RelationId(0),
                    fields: vec![FieldId(0)],
                    vars: vec![VarId(0)],
                    access: AccessId(0),
                }],
                implementation: NodeImpl::Hybrid,
                payload: PayloadDemand {
                    existence_only_relations: vec![RelationId(1)],
                    row_id_demands: vec![RelationId(0)],
                    ..PayloadDemand::default()
                },
            }],
            output: OutputPlan::Project(ProjectPlan {
                vars: vec![VarId(0)],
                set_semantics: true,
            }),
            estimates: PlanEstimates::default(),
        };

        plan.validate().unwrap();
        assert!(!plan.is_pure_lftj());
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
                    access: AccessId(0),
                }],
                implementation: NodeImpl::SortedLeapfrog,
                payload: PayloadDemand::default(),
            }],
            output: OutputPlan::default(),
            estimates: PlanEstimates::default(),
        };

        assert!(plan.validate().is_err());
    }
}
