//! Free Join plan lowering (PRD 17): `binary2fj` (paper Fig. 7), the
//! conservative `factor()` hoist (Fig. 8), cover enumeration (§4.4),
//! residual placement, trie schemas (§3.3), and the sealed
//! [`ValidatedPlan`] witness (`docs/architecture/30-execution.md`).
//!
//! Plain `Vec`s everywhere — no fixed-capacity silent-drop containers
//! (post-mortem §35: capacity bugs must be impossible, not silent).

use std::collections::BTreeSet;

use crate::image::view::{Const, FilterPredicate};
use crate::ir::normalize::{NormalizedQuery, OccId, PlacedComparison};
use crate::ir::VarId;
use crate::plan::planner::JoinOrder;
use crate::schema::{RelationId, Schema};

/// A subatom: one occurrence with a subset of its variables. The plan
/// partitions every occurrence's variables across its subatoms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subatom {
    pub occ: OccId,
    pub vars: Vec<VarId>,
}

/// One plan node: a list of subatoms. Executed as: iterate the chosen
/// cover, probe the rest in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub subatoms: Vec<Subatom>,
}

/// A Free Join plan: a list of nodes partitioning the query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FjPlan {
    pub nodes: Vec<Node>,
}

/// Converts a left-deep binary join order into an equivalent Free Join
/// plan — the paper's Fig. 7, transcribed: the first occurrence
/// contributes its full atom; each subsequent occurrence contributes a
/// probe subatom on its available variables, then opens a node with its
/// remaining variables.
///
/// # Panics
///
/// Only on programmer-invariant violations: `order` referencing an
/// occurrence the normalized query lacks.
#[must_use]
pub fn binary2fj(normalized: &NormalizedQuery, order: &JoinOrder) -> FjPlan {
    let occurrence = |occ: OccId| {
        normalized
            .occurrences
            .iter()
            .find(|o| o.occ_id == occ)
            .expect("join order references known occurrences")
    };
    let vars_of =
        |occ: OccId| -> Vec<VarId> { occurrence(occ).vars.iter().map(|(_, v)| *v).collect() };

    let mut nodes: Vec<Node> = Vec::new();
    let first = order.order[0];
    let mut available: BTreeSet<VarId> = vars_of(first).iter().copied().collect();
    let mut current = Node {
        subatoms: vec![Subatom {
            occ: first,
            vars: vars_of(first),
        }],
    };
    for &next in &order.order[1..] {
        let vars = vars_of(next);
        let probe: Vec<VarId> = vars
            .iter()
            .copied()
            .filter(|v| available.contains(v))
            .collect();
        let remaining: Vec<VarId> = vars
            .iter()
            .copied()
            .filter(|v| !available.contains(v))
            .collect();
        current.subatoms.push(Subatom {
            occ: next,
            vars: probe,
        });
        nodes.push(current);
        available.extend(vars);
        current = Node {
            subatoms: vec![Subatom {
                occ: next,
                vars: remaining,
            }],
        };
    }
    nodes.push(current);
    FjPlan { nodes }
}

/// The paper's Fig. 8 conservative hoist: traverse nodes in reverse; move a
/// *lookup* subatom (never a node's first subatom — that is its opened
/// iterate) to the previous node iff its variables are all available
/// before this node and the previous node lacks that occurrence, stopping
/// per node at the first non-hoistable lookup (preserving the probe order
/// the cost-based order implies).
pub fn factor(plan: &mut FjPlan) {
    for i in (1..plan.nodes.len()).rev() {
        let available: BTreeSet<VarId> = plan.nodes[..i]
            .iter()
            .flat_map(|n| n.subatoms.iter())
            .flat_map(|s| s.vars.iter().copied())
            .collect();
        // Lookups start at index 1; hoisting shifts the next lookup into
        // index 1, so the loop re-examines that slot.
        while plan.nodes[i].subatoms.len() > 1 {
            let candidate = &plan.nodes[i].subatoms[1];
            let hoistable = candidate.vars.iter().all(|v| available.contains(v))
                && !plan.nodes[i - 1]
                    .subatoms
                    .iter()
                    .any(|s| s.occ == candidate.occ);
            if !hoistable {
                break;
            }
            let moved = plan.nodes[i].subatoms.remove(1);
            plan.nodes[i - 1].subatoms.push(moved);
        }
    }
}

/// A plan-validation failure. Plans built by `binary2fj` + `factor` are
/// valid by construction; this boundary exists because [`FjPlan`] is plain
/// data anyone can construct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// An occurrence's subatoms do not partition its variable set.
    BrokenPartition { occ: OccId },
    /// Two subatoms of one node share an occurrence.
    DuplicateOccurrenceInNode { node: usize, occ: OccId },
    /// A node has no cover: no subatom contains all its new variables.
    NoCover { node: usize },
    /// A residual comparison's variables are never both bound.
    UnplacedResidual { residual: usize },
}

/// One occurrence's execution-facing description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanOccurrence {
    pub occ_id: OccId,
    pub relation: RelationId,
    /// The field each variable reads from (field index = column index).
    pub vars: Vec<(crate::schema::FieldId, VarId)>,
    /// Per-occurrence filters (evaluated at the source view).
    pub filters: Vec<FilterPredicate>,
    /// The trie schema: this occurrence's subatom var-lists in node order
    /// (§3.3). Under COLT laziness there is no trailing `[]` level — the
    /// build-phase question dissolves (30-execution).
    pub trie_schema: Vec<Vec<VarId>>,
}

/// One validated node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanNode {
    pub subatoms: Vec<Subatom>,
    /// Indices into `subatoms` of the valid covers (every subatom
    /// containing all variables new to this node) — the runtime chooses
    /// among them by key count (§4.4).
    pub covers: Vec<u8>,
    /// Residual comparisons evaluated at this node (both sides bound here
    /// for the first time).
    pub residuals: Vec<PlacedComparison>,
    /// Variables first bound by this node.
    pub new_vars: Vec<VarId>,
    /// Whether this node binds any sink-relevant (projected) variable —
    /// the D2 subtree-skip unwind stops at the first `true` node
    /// (precomputed here; the executor just reads the bit).
    pub sink_relevant: bool,
}

/// The sealed plan witness execution trusts; validated once at
/// construction, nothing downstream re-checks (post-mortem §38).
#[derive(Debug)]
pub struct ValidatedPlan {
    occurrences: Vec<PlanOccurrence>,
    nodes: Vec<PlanNode>,
    /// Dense binding-slot layout: `slots[i]` is the variable stored in
    /// slot `i`; `slot_of` maps a `VarId` to its slot.
    slots: Vec<VarId>,
    /// Provably-distinct-bindings: every occurrence's bound fields cover a
    /// unique constraint, so distinct facts imply distinct bindings and the
    /// aggregate sink may skip its seen-set (30-execution, elision).
    distinct_bindings: bool,
    /// The planner's per-step estimates (EXPLAIN's reader, PRD 24).
    estimates: Vec<u64>,
}

impl ValidatedPlan {
    #[must_use]
    pub fn occurrences(&self) -> &[PlanOccurrence] {
        &self.occurrences
    }

    /// # Panics
    ///
    /// On a programmer-invariant violation: an occurrence outside the plan.
    #[cfg(test)]
    #[must_use]
    pub fn occurrence(&self, occ: OccId) -> &PlanOccurrence {
        self.occurrences
            .iter()
            .find(|o| o.occ_id == occ)
            .expect("validated plan covers its occurrences")
    }

    #[must_use]
    pub fn nodes(&self) -> &[PlanNode] {
        &self.nodes
    }

    /// Slot order: the variable stored in each binding slot.
    #[must_use]
    pub fn slots(&self) -> &[VarId] {
        &self.slots
    }

    /// The slot index of a variable.
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: a variable outside the plan.
    #[must_use]
    pub fn slot_of(&self, var: VarId) -> usize {
        self.slots
            .iter()
            .position(|v| *v == var)
            .expect("validated plan binds every variable")
    }

    #[must_use]
    pub fn distinct_bindings(&self) -> bool {
        self.distinct_bindings
    }

    #[must_use]
    pub fn estimates(&self) -> &[u64] {
        &self.estimates
    }
}

/// Validates a plan against its normalized query, deriving covers, residual
/// placement, trie schemas, the binding-slot layout, and the
/// distinct-bindings flag.
///
/// # Errors
///
/// [`PlanError`] when the plan does not partition the query, duplicates an
/// occurrence within a node, lacks a cover, or leaves a residual unplaced.
///
/// # Panics
///
/// Only on programmer-invariant violations (more than 256 subatoms in one
/// node — impossible for plans over the planner's occurrence cap).
pub fn validate(
    plan: &FjPlan,
    normalized: &NormalizedQuery,
    schema: &Schema,
    estimates: Vec<u64>,
    sink_vars: &BTreeSet<VarId>,
) -> Result<ValidatedPlan, PlanError> {
    // Partition property, per occurrence: subatom vars are disjoint and
    // union to the occurrence's var set.
    for occurrence in &normalized.occurrences {
        let mut seen: BTreeSet<VarId> = BTreeSet::new();
        for node in &plan.nodes {
            for subatom in node.subatoms.iter().filter(|s| s.occ == occurrence.occ_id) {
                for var in &subatom.vars {
                    if !seen.insert(*var) {
                        return Err(PlanError::BrokenPartition {
                            occ: occurrence.occ_id,
                        });
                    }
                }
            }
        }
        let expected: BTreeSet<VarId> = occurrence.vars.iter().map(|(_, v)| *v).collect();
        if seen != expected {
            return Err(PlanError::BrokenPartition {
                occ: occurrence.occ_id,
            });
        }
    }

    let mut nodes = derive_nodes(plan)?;
    for node in &mut nodes {
        node.sink_relevant = node.new_vars.iter().any(|v| sink_vars.contains(v));
    }

    // Residual placement: the earliest node at which both sides are bound.
    for (residual_idx, residual) in normalized.residuals.iter().enumerate() {
        let mut bound: BTreeSet<VarId> = BTreeSet::new();
        let mut placed = false;
        for node in &mut nodes {
            bound.extend(node.new_vars.iter().copied());
            if bound.contains(&residual.lhs) && bound.contains(&residual.rhs) {
                node.residuals.push(*residual);
                placed = true;
                break;
            }
        }
        if !placed {
            return Err(PlanError::UnplacedResidual {
                residual: residual_idx,
            });
        }
    }

    // Trie schemas: each occurrence's subatom var-lists in node order.
    let occurrences: Vec<PlanOccurrence> = normalized
        .occurrences
        .iter()
        .map(|occurrence| {
            let trie_schema: Vec<Vec<VarId>> = plan
                .nodes
                .iter()
                .flat_map(|n| n.subatoms.iter())
                .filter(|s| s.occ == occurrence.occ_id)
                .map(|s| s.vars.clone())
                .collect();
            PlanOccurrence {
                occ_id: occurrence.occ_id,
                relation: occurrence.relation,
                vars: occurrence.vars.clone(),
                filters: occurrence.filters.clone(),
                trie_schema,
            }
        })
        .collect();

    // Binding-slot layout: node order, then subatom order — dense.
    let mut slots: Vec<VarId> = Vec::new();
    for node in &nodes {
        for var in &node.new_vars {
            if !slots.contains(var) {
                slots.push(*var);
            }
        }
    }

    let distinct_bindings = provably_distinct(normalized, schema);

    Ok(ValidatedPlan {
        occurrences,
        nodes,
        slots,
        distinct_bindings,
        estimates,
    })
}

/// The distinct-bindings elision check (30-execution): every occurrence's
/// bound fields — variable-bound or equality-filtered to a constant —
/// cover one of its unique constraints, so distinct facts imply distinct
/// bindings and the aggregate sink may skip its seen-set.
fn provably_distinct(normalized: &NormalizedQuery, schema: &Schema) -> bool {
    normalized.occurrences.iter().all(|occurrence| {
        let relation = schema.relation(occurrence.relation);
        let bound_fields: BTreeSet<crate::schema::FieldId> =
            occurrence
                .vars
                .iter()
                .map(|(f, _)| *f)
                .chain(occurrence.filters.iter().filter_map(|f| match f {
                    FilterPredicate::Compare {
                        field,
                        op: crate::ir::CmpOp::Eq,
                        value:
                            Const::Word(_)
                            | Const::Byte(_)
                            | Const::Param(_)
                            | Const::PendingIntern { .. },
                    } => Some(*field),
                    _ => None,
                }))
                .collect();
        relation.unique_constraints().iter().any(|cid| {
            relation
                .constraint(*cid)
                .fields()
                .iter()
                .all(|f| bound_fields.contains(f))
        })
    })
}

/// Derives per-node covers and new-var sets, rejecting duplicate
/// occurrences within a node and cover-less nodes.
fn derive_nodes(plan: &FjPlan) -> Result<Vec<PlanNode>, PlanError> {
    let mut nodes = Vec::with_capacity(plan.nodes.len());
    let mut available: BTreeSet<VarId> = BTreeSet::new();
    for (node_idx, node) in plan.nodes.iter().enumerate() {
        for (idx, subatom) in node.subatoms.iter().enumerate() {
            if node.subatoms[..idx].iter().any(|s| s.occ == subatom.occ) {
                return Err(PlanError::DuplicateOccurrenceInNode {
                    node: node_idx,
                    occ: subatom.occ,
                });
            }
        }
        let node_vars: BTreeSet<VarId> = node
            .subatoms
            .iter()
            .flat_map(|s| s.vars.iter().copied())
            .collect();
        let new_vars: Vec<VarId> = node_vars
            .iter()
            .copied()
            .filter(|v| !available.contains(v))
            .collect();
        // A cover must contain all of the node's new vars AND nothing else
        // (Deviation from the paper's Definition, recorded in
        // docs/architecture/30-execution.md): a subatom that also carries an
        // already-bound variable is iterable per the paper, but iterating it
        // would *rebind* the bound variable without re-checking the
        // occurrence that bound it — wrong results under dynamic cover
        // choice. Restricting covers to exactly-the-new-vars keeps every
        // binary2fj node's opening subatom (its vars are exactly the
        // remainder) and every GJ-style single-var cover.
        let covers: Vec<u8> = node
            .subatoms
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.vars.len() == new_vars.len() && new_vars.iter().all(|v| s.vars.contains(v))
            })
            .map(|(i, _)| u8::try_from(i).expect("subatoms per node fit u8"))
            .collect();
        if covers.is_empty() {
            return Err(PlanError::NoCover { node: node_idx });
        }
        available.extend(node_vars);
        nodes.push(PlanNode {
            subatoms: node.subatoms.clone(),
            covers,
            residuals: Vec::new(),
            new_vars,
            sink_relevant: false, // filled by the caller from sink_vars
        });
    }
    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::normalize::Occurrence;
    use crate::ir::CmpOp;
    use crate::schema::{
        FieldDescriptor, FieldId, Generation, RelationDescriptor, SchemaDescriptor, ValueType,
    };

    /// Named variables for readability: x=0, a=1, b=2, c=3, y=4, z=5, u=6,
    /// v=7.
    const X: VarId = VarId(0);
    const A: VarId = VarId(1);
    const B: VarId = VarId(2);
    const C: VarId = VarId(3);
    const Y: VarId = VarId(4);
    const Z: VarId = VarId(5);
    const U: VarId = VarId(6);
    const V: VarId = VarId(7);

    fn schema(relations: usize, arity: usize) -> Schema {
        SchemaDescriptor {
            relations: (0..relations)
                .map(|r| RelationDescriptor {
                    name: format!("R{r}").into(),
                    fields: (0..arity)
                        .map(|f| FieldDescriptor {
                            name: format!("f{f}").into(),
                            value_type: ValueType::U64,
                            generation: if f == 0 {
                                Generation::Serial
                            } else {
                                Generation::None
                            },
                        })
                        .collect(),
                    constraints: vec![],
                })
                .collect(),
        }
        .validate()
        .expect("valid fixture")
    }

    fn occurrence(occ: u16, relation: u32, vars: &[(u16, VarId)]) -> Occurrence {
        Occurrence {
            occ_id: OccId(occ),
            relation: RelationId(relation),
            vars: vars.iter().map(|(f, v)| (FieldId(*f), *v)).collect(),
            filters: vec![],
        }
    }

    fn order(ids: &[u16]) -> JoinOrder {
        JoinOrder {
            order: ids.iter().map(|i| OccId(*i)).collect(),
            estimates: vec![0; ids.len()],
        }
    }

    fn subatom(occ: u16, vars: &[VarId]) -> Subatom {
        Subatom {
            occ: OccId(occ),
            vars: vars.to_vec(),
        }
    }

    /// The clover query: R(x,a), S(x,b), T(x,c).
    fn clover() -> NormalizedQuery {
        NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(1, X), (2, A)]),
                occurrence(1, 1, &[(1, X), (2, B)]),
                occurrence(2, 2, &[(1, X), (2, C)]),
            ],
            residuals: vec![],
        }
    }

    #[test]
    fn binary2fj_and_factor_match_the_papers_clover_example() {
        let normalized = clover();
        let mut plan = binary2fj(&normalized, &order(&[0, 1, 2]));
        // Fig. 7 output: [[R(x,a),S(x)],[S(b),T(x)],[T(c)]].
        assert_eq!(
            plan.nodes,
            vec![
                Node {
                    subatoms: vec![subatom(0, &[X, A]), subatom(1, &[X])]
                },
                Node {
                    subatoms: vec![subatom(1, &[B]), subatom(2, &[X])]
                },
                Node {
                    subatoms: vec![subatom(2, &[C])]
                },
            ]
        );
        factor(&mut plan);
        // Fig. 8 output: [[R(x,a),S(x),T(x)],[S(b)],[T(c)]].
        assert_eq!(
            plan.nodes,
            vec![
                Node {
                    subatoms: vec![subatom(0, &[X, A]), subatom(1, &[X]), subatom(2, &[X])]
                },
                Node {
                    subatoms: vec![subatom(1, &[B])]
                },
                Node {
                    subatoms: vec![subatom(2, &[C])]
                },
            ]
        );
    }

    #[test]
    fn binary2fj_matches_the_papers_chain_example() {
        // Q :- R(x,y), S(y,z), T(z,u), W(u,v) with plan [R,S,T,W] (§4.1).
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(1, X), (2, Y)]),
                occurrence(1, 1, &[(1, Y), (2, Z)]),
                occurrence(2, 2, &[(1, Z), (2, U)]),
                occurrence(3, 3, &[(1, U), (2, V)]),
            ],
            residuals: vec![],
        };
        let plan = binary2fj(&normalized, &order(&[0, 1, 2, 3]));
        assert_eq!(
            plan.nodes,
            vec![
                Node {
                    subatoms: vec![subatom(0, &[X, Y]), subatom(1, &[Y])]
                },
                Node {
                    subatoms: vec![subatom(1, &[Z]), subatom(2, &[Z])]
                },
                Node {
                    subatoms: vec![subatom(2, &[U]), subatom(3, &[U])]
                },
                Node {
                    subatoms: vec![subatom(3, &[V])]
                },
            ]
        );
    }

    #[test]
    fn trie_schemas_match_the_papers_triangle_worked_example() {
        // Triangle plan [[R(x,y),S(y),T(x)],[S(z),T(z)]] (§3.3): R is a
        // vector, S a map->vector, T a map->map (no trailing [] under COLT
        // laziness — the build-phase question dissolves, 30-execution).
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(1, X), (2, Y)]),
                occurrence(1, 1, &[(1, Y), (2, Z)]),
                occurrence(2, 2, &[(1, X), (2, Z)]),
            ],
            residuals: vec![],
        };
        let plan = FjPlan {
            nodes: vec![
                Node {
                    subatoms: vec![subatom(0, &[X, Y]), subatom(1, &[Y]), subatom(2, &[X])],
                },
                Node {
                    subatoms: vec![subatom(1, &[Z]), subatom(2, &[Z])],
                },
            ],
        };
        let validated = validate(
            &plan,
            &normalized,
            &schema(3, 3),
            vec![0, 0],
            &BTreeSet::new(),
        )
        .expect("valid plan");
        assert_eq!(validated.occurrence(OccId(0)).trie_schema, vec![vec![X, Y]]);
        assert_eq!(
            validated.occurrence(OccId(1)).trie_schema,
            vec![vec![Y], vec![Z]]
        );
        assert_eq!(
            validated.occurrence(OccId(2)).trie_schema,
            vec![vec![X], vec![Z]]
        );
    }

    #[test]
    fn gj_style_plan_has_multiple_covers_on_the_first_node() {
        // The paper: "for the first node we could have also chosen S(x) or
        // T(x) as cover" — the GJ plan for the clover query.
        let plan = FjPlan {
            nodes: vec![
                Node {
                    subatoms: vec![subatom(0, &[X]), subatom(1, &[X]), subatom(2, &[X])],
                },
                Node {
                    subatoms: vec![subatom(0, &[A])],
                },
                Node {
                    subatoms: vec![subatom(1, &[B])],
                },
                Node {
                    subatoms: vec![subatom(2, &[C])],
                },
            ],
        };
        let validated = validate(
            &plan,
            &clover(),
            &schema(3, 3),
            vec![0; 4],
            &BTreeSet::new(),
        )
        .expect("valid plan");
        assert_eq!(validated.nodes()[0].covers, vec![0, 1, 2]);
        assert_eq!(validated.nodes()[1].covers, vec![0]);
    }

    #[test]
    fn residuals_attach_to_the_first_node_binding_both_sides() {
        // Residual a < b: a is bound by node 1 (R's a), b by node 2 (S's b)
        // in the unfactored clover plan — so it places on node 2.
        let mut normalized = clover();
        normalized.residuals = vec![PlacedComparison {
            op: CmpOp::Lt,
            lhs: A,
            rhs: B,
        }];
        let plan = binary2fj(&normalized, &order(&[0, 1, 2]));
        let validated = validate(
            &plan,
            &normalized,
            &schema(3, 3),
            vec![0; 3],
            &BTreeSet::new(),
        )
        .expect("valid plan");
        assert!(validated.nodes()[0].residuals.is_empty());
        assert_eq!(validated.nodes()[1].residuals.len(), 1);
        assert!(validated.nodes()[2].residuals.is_empty());
    }

    #[test]
    fn self_join_plans_validate_over_occurrences() {
        // Grandparent over OrgParent: two occurrences of one relation.
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(1, X), (2, Y)]),
                occurrence(1, 0, &[(1, Y), (2, Z)]),
            ],
            residuals: vec![],
        };
        let mut plan = binary2fj(&normalized, &order(&[0, 1]));
        factor(&mut plan);
        let validated = validate(
            &plan,
            &normalized,
            &schema(1, 3),
            vec![0, 0],
            &BTreeSet::new(),
        )
        .expect("self-joins validate");
        assert_eq!(validated.occurrences().len(), 2);
    }

    #[test]
    fn duplicate_occurrence_within_a_node_is_rejected() {
        let plan = FjPlan {
            nodes: vec![Node {
                subatoms: vec![subatom(0, &[X, A]), subatom(0, &[])],
            }],
        };
        let mut normalized = clover();
        normalized.occurrences.truncate(1);
        let err =
            validate(&plan, &normalized, &schema(3, 3), vec![0], &BTreeSet::new()).unwrap_err();
        assert_eq!(
            err,
            PlanError::DuplicateOccurrenceInNode {
                node: 0,
                occ: OccId(0)
            }
        );
    }

    #[test]
    fn distinct_bindings_flag_tracks_unique_coverage() {
        // Serial-bound occurrence: field 0 (serial) is var-bound in every
        // occurrence -> flag set.
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, X), (1, A)]),
                occurrence(1, 1, &[(0, B), (1, X)]),
            ],
            residuals: vec![],
        };
        let plan = binary2fj(&normalized, &order(&[0, 1]));
        let validated = validate(
            &plan,
            &normalized,
            &schema(2, 2),
            vec![0, 0],
            &BTreeSet::new(),
        )
        .expect("valid plan");
        assert!(validated.distinct_bindings());

        // Occurrence 1 binds only a non-unique field -> flag clear.
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, X), (1, A)]),
                occurrence(1, 1, &[(1, X)]),
            ],
            residuals: vec![],
        };
        let plan = binary2fj(&normalized, &order(&[0, 1]));
        let validated = validate(
            &plan,
            &normalized,
            &schema(2, 2),
            vec![0, 0],
            &BTreeSet::new(),
        )
        .expect("valid plan");
        assert!(!validated.distinct_bindings());
    }

    #[test]
    fn binding_slots_follow_node_order() {
        let normalized = clover();
        let mut plan = binary2fj(&normalized, &order(&[0, 1, 2]));
        factor(&mut plan);
        let validated = validate(
            &plan,
            &normalized,
            &schema(3, 3),
            vec![0; 3],
            &BTreeSet::new(),
        )
        .expect("valid plan");
        // Factored clover: node 0 binds {x, a}, node 1 binds {b}, node 2
        // binds {c}. Slot order follows.
        assert_eq!(validated.slots(), &[X, A, B, C]);
        assert_eq!(validated.slot_of(C), 3);
    }
}
