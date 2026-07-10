use std::collections::BTreeMap;

use super::{
    lower_literal::{lower_literal, point_word},
    place_comparisons::place_comparisons,
    AntiProbe, NormalizedQuery, OccId, Occurrence, Role, SlotWidth,
};
use crate::image::view::{Const, FilterPredicate, ResolvedWordSource};
use crate::ir::validate::ValidatedQuery;
use crate::ir::{Atom, CmpOp, Term, Value, VarId};
use crate::schema::{FieldId, Schema, ValueType};

/// Lowers the witness into paper form.
///
/// # Panics
///
/// Only on programmer-invariant violations already excluded by validation
/// (e.g. a comparison variable bound by no atom).
#[must_use]
pub fn normalize(schema: &Schema, query: &ValidatedQuery) -> NormalizedQuery {
    let positive = query.query().atoms.len();
    let mut occurrences: Vec<Occurrence> =
        Vec::with_capacity(positive + query.query().negated.len());
    for (idx, atom) in query.query().atoms.iter().enumerate() {
        occurrences.push(lower_atom(schema, query, idx, Role::Positive, atom));
    }
    for (idx, atom) in query.query().negated.iter().enumerate() {
        occurrences.push(lower_atom(
            schema,
            query,
            positive + idx,
            Role::Negated,
            atom,
        ));
    }

    // One anti-probe descriptor per negated occurrence: the probe keys are
    // its variable bindings; its constant bindings became its filter list
    // above, evaluated inside the probe.
    let anti_probes: Vec<AntiProbe> = occurrences[positive..]
        .iter()
        .map(|occurrence| AntiProbe {
            occurrence: occurrence.occ_id,
            probe_bindings: occurrence.vars.clone(),
        })
        .collect();

    let (residuals, word_residuals, allen_residuals) = place_comparisons(query, &mut occurrences);

    // The binding-slot widths — the two-slot interval layout, decided at
    // [`SlotWidth`] and exported here to the plan witness.
    let slot_widths: BTreeMap<VarId, SlotWidth> = query
        .var_types()
        .map(|(var, value_type)| (var, SlotWidth::of(value_type)))
        .collect();

    // Nothing single-occurrence survives to the residual list
    // (docs/architecture/20-query-ir.md, § normalization step 5) — across
    // every residual kind: whole-value, decomposed word, and Allen mask
    // comparisons.
    debug_assert!(residuals
        .iter()
        .map(|r| (r.lhs, r.rhs))
        .chain(word_residuals.iter().map(|r| (r.lhs.var, r.rhs.var)))
        .chain(allen_residuals.iter().map(|r| (r.lhs, r.rhs)))
        .all(|(lhs, rhs)| {
            !occurrences
                .iter()
                .filter(|occ| occ.role.participates())
                .any(|occ| {
                    occ.vars.iter().any(|(_, v)| *v == lhs)
                        && occ.vars.iter().any(|(_, v)| *v == rhs)
                })
        }));

    NormalizedQuery {
        occurrences,
        residuals,
        word_residuals,
        allen_residuals,
        anti_probes,
        slot_widths,
    }
}

/// Whether a binding is a **membership** position: an interval field bound
/// to an element-typed term — the term is a point in the field's interval,
/// never its value (`docs/architecture/20-query-ir.md`, the membership
/// rule; validation resolved every term's type).
fn is_membership(field_type: &ValueType, term_type: &ValueType) -> bool {
    matches!(field_type, ValueType::Interval { .. })
        && !matches!(term_type, ValueType::Interval { .. })
}

/// Lowers one atom (positive or negated — the rules are identical; only
/// the role differs) into an occurrence: variable positions plus the
/// filters lowered out of its constant, repeated-variable, and membership
/// bindings.
fn lower_atom(
    schema: &Schema,
    query: &ValidatedQuery,
    idx: usize,
    role: Role,
    atom: &Atom,
) -> Occurrence {
    let occ_id = OccId(u16::try_from(idx).expect("validated: occurrence count fits u16"));
    let relation = schema.relation(atom.relation);

    // Pass 1 — variable positions: the first *domain* binding of each
    // variable (a scalar field, or an interval field read by value).
    // Membership positions bind no variable — they are conditions, lowered
    // to filters in pass 2.
    let mut vars: Vec<(FieldId, VarId)> = Vec::new();
    for (field, term) in &atom.bindings {
        if let Term::Var(var) = term {
            let field_type = &relation.field(*field).value_type;
            if is_membership(field_type, query.var_type(*var)) {
                continue;
            }
            if !vars.iter().any(|(_, v)| v == var) {
                vars.push((*field, *var));
            }
        }
    }

    // Pass 2 — filters, in binding order.
    let mut filters = Vec::new();
    for (field, term) in &atom.bindings {
        let field_type = &relation.field(*field).value_type;
        match term {
            Term::Var(var) => {
                if is_membership(field_type, query.var_type(*var)) {
                    // Membership: `start ≤ var < end`. With the point
                    // variable scalar-bound in this atom, the condition is
                    // a same-fact field composition; otherwise it reads the
                    // variable's binding once bound (the point-membership
                    // scan, docs/architecture/40-execution.md).
                    filters.push(match vars.iter().find(|(_, v)| v == var) {
                        Some((point_field, _)) => FilterPredicate::FieldsContainPoint {
                            interval: *field,
                            point: *point_field,
                        },
                        None => FilterPredicate::PointIn {
                            field: *field,
                            point: ResolvedWordSource::Var(*var),
                        },
                    });
                } else {
                    // A repeated variable keeps its first field binding as
                    // the variable position; subsequent positions lower to
                    // same-fact equality filters.
                    let (first_field, _) = vars
                        .iter()
                        .find(|(_, v)| v == var)
                        .expect("pass 1 recorded every domain-bound variable");
                    if first_field != field {
                        filters.push(FilterPredicate::FieldsCompare {
                            left: *first_field,
                            right: *field,
                            op: CmpOp::Eq,
                        });
                    }
                }
            }
            Term::Param(param) => {
                if is_membership(field_type, query.param_type(*param)) {
                    filters.push(FilterPredicate::PointIn {
                        field: *field,
                        point: ResolvedWordSource::Param(*param),
                    });
                } else {
                    filters.push(FilterPredicate::Compare {
                        field: *field,
                        op: CmpOp::Eq,
                        value: Const::Param(*param),
                    });
                }
            }
            Term::ParamSet(param) => {
                if matches!(field_type, ValueType::Interval { .. }) {
                    // A set holds points (validation anchored the element
                    // type): any element in the field's interval.
                    filters.push(FilterPredicate::AnyPointIn {
                        field: *field,
                        set: Const::ParamSet(*param),
                    });
                } else {
                    // The selection-level set marker: an Eq compare the
                    // plan routes into `selections`, carried as a word set
                    // at bind (docs/architecture/20-query-ir.md, § param
                    // sets; executor side is PRD 17).
                    filters.push(FilterPredicate::Compare {
                        field: *field,
                        op: CmpOp::Eq,
                        value: Const::ParamSet(*param),
                    });
                }
            }
            Term::Literal(value) => {
                let membership = matches!(field_type, ValueType::Interval { .. })
                    && !matches!(value, Value::IntervalU64(..) | Value::IntervalI64(..));
                if membership {
                    filters.push(FilterPredicate::PointIn {
                        field: *field,
                        point: ResolvedWordSource::Word(point_word(value)),
                    });
                } else {
                    filters.push(FilterPredicate::Compare {
                        field: *field,
                        op: CmpOp::Eq,
                        value: lower_literal(value),
                    });
                }
            }
        }
    }

    Occurrence {
        occ_id,
        relation: atom.relation,
        role,
        vars,
        filters,
    }
}
