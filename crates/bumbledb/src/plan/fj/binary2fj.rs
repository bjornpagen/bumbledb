use super::{FjPlan, Node, Subatom};
use crate::ir::VarId;
use crate::ir::normalize::{NormalizedQuery, OccId};
use crate::plan::planner::JoinOrder;
use std::collections::BTreeSet;

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
