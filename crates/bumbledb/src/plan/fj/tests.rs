use super::*;
use crate::ir::normalize::{NormalizedQuery, Occurrence, Role};
use crate::plan::planner::JoinOrder;
use crate::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, Schema, SchemaDescriptor, ValueType,
};
use std::collections::BTreeMap;

mod build;
mod selections;
mod validate;
mod witness;

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
                            Generation::Fresh
                        } else {
                            Generation::None
                        },
                    })
                    .collect(),
            })
            .collect(),
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

fn occurrence(occ: u16, relation: u32, vars: &[(u16, VarId)]) -> Occurrence {
    Occurrence {
        occ_id: OccId(occ),
        relation: RelationId(relation),
        role: Role::Positive,
        vars: vars.iter().map(|(f, v)| (FieldId(*f), *v)).collect(),
        filters: vec![],
    }
}

fn negated(occ: u16, relation: u32, vars: &[(u16, VarId)]) -> Occurrence {
    Occurrence {
        role: Role::Negated,
        ..occurrence(occ, relation, vars)
    }
}

/// Assembles a `NormalizedQuery` the way `normalize` would: anti-probe
/// descriptors derived from the negated occurrences, every variable one
/// slot wide (these fixtures are scalar-only; interval fixtures go
/// through real normalization in `witness.rs`).
fn normalized(occurrences: Vec<Occurrence>, residuals: Vec<PlacedComparison>) -> NormalizedQuery {
    let anti_probes = occurrences
        .iter()
        .filter(|o| o.role == Role::Negated)
        .map(|o| AntiProbe {
            occurrence: o.occ_id,
            probe_bindings: o.vars.clone(),
        })
        .collect();
    let slot_widths: BTreeMap<VarId, SlotWidth> = occurrences
        .iter()
        .flat_map(|o| o.vars.iter().map(|(_, v)| (*v, SlotWidth::One)))
        .collect();
    NormalizedQuery {
        occurrences,
        residuals,
        word_residuals: vec![],
        allen_residuals: vec![],
        duration_residuals: Vec::new(),
        anti_probes,
        slot_widths,
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
    normalized(
        vec![
            occurrence(0, 0, &[(1, X), (2, A)]),
            occurrence(1, 1, &[(1, X), (2, B)]),
            occurrence(2, 2, &[(1, X), (2, C)]),
        ],
        vec![],
    )
}
