use std::collections::BTreeMap;

use super::{
    AntiProbe, NormalizedQuery, OccBind, OccId, Occurrence, Role, SlotWidth,
    lower_literal::{lower_literal, point_word},
    place_comparisons::place_comparisons,
};
use crate::image::view::{Const, FilterPredicate, SetConst, ViewWordSource};
use crate::ir::validate::RuleWitness;
use crate::ir::{Atom, Term, Value, VarId, WordCmp};
use crate::schema::Schema;
use bumbledb_theory::schema::{FieldId, ValueType};

/// [`normalize_rules`] with the interiors/rec typing surface: `signatures`
/// holds every derived table's sealed signature in `InteriorId` then rec
/// order, and an
/// `Interior` binding's field type reads the target's column — `FieldId(i)`
/// is head position `i`. Everything else is the conjunctive lowering, verbatim.
#[must_use]
pub fn normalize_rules<'a>(
    schema: &Schema,
    signatures: &[&crate::ir::validate::Signature],
    rules: impl IntoIterator<Item = RuleWitness<'a>>,
) -> Vec<NormalizedQuery> {
    rules
        .into_iter()
        .map(|rule| normalize_rule(schema, signatures, &rule))
        .collect()
}

fn normalize_rule(
    schema: &Schema,
    signatures: &[&crate::ir::validate::Signature],
    rule: &RuleWitness<'_>,
) -> NormalizedQuery {
    normalize_rule_with(schema, signatures, rule, rule.classified_comparisons())
}

fn normalize_rule_with(
    schema: &Schema,
    signatures: &[&crate::ir::validate::Signature],
    rule: &RuleWitness<'_>,
    comparisons: &[crate::ir::validate::ClassifiedComparison],
) -> NormalizedQuery {
    let positive = rule.rule().atoms.len();
    let mut occurrences: Vec<Occurrence> = Vec::with_capacity(positive + rule.rule().negated.len());
    for (idx, atom) in rule.rule().atoms.iter().enumerate() {
        occurrences.push(lower_atom(
            schema,
            signatures,
            rule,
            idx,
            Role::Positive,
            atom,
        ));
    }
    for (idx, atom) in rule.rule().negated.iter().enumerate() {
        occurrences.push(lower_atom(
            schema,
            signatures,
            rule,
            positive + idx,
            Role::Negated,
            atom,
        ));
    }

    let anti_probes: Vec<AntiProbe> = occurrences[positive..]
        .iter()
        .map(|occurrence| AntiProbe {
            occurrence: occurrence.occ_id,
            probe_bindings: occurrence.vars.clone(),
        })
        .collect();

    let (residuals, word_residuals, allen_residuals) = {
        let mut span = crate::obs::span(crate::obs::names::PLACE_COMPARISONS);
        let placed = place_comparisons(comparisons, &mut occurrences);
        span.set_count((placed.0.len() + placed.1.len() + placed.2.len()) as u64);
        placed
    };

    let slot_widths: BTreeMap<VarId, SlotWidth> = rule
        .var_types()
        .map(|(var, value_type)| (var, SlotWidth::of(value_type)))
        .collect();

    debug_assert!(
        residuals
            .iter()
            .map(|r| {
                let (left, right, _) = r.compare_sides();
                (left.var(), right.var())
            })
            .chain(word_residuals.iter().map(|r| {
                let (left, right, _) = r.compare_sides();
                (left.var(), right.var())
            }))
            .chain(allen_residuals.iter().map(|r| {
                let (left, right, _) = r.allen_sides();
                (left.var(), right.var())
            }))
            .all(|(lhs, rhs)| {
                !occurrences
                    .iter()
                    .filter(|occ| occ.role.participates())
                    .any(|occ| {
                        occ.vars.iter().any(|(_, v)| *v == lhs)
                            && occ.vars.iter().any(|(_, v)| *v == rhs)
                    })
            })
    );

    let dead = {
        let mut span = crate::obs::span(crate::obs::names::NORMALIZE_FOLD);
        let dead = super::fold::fold(schema, &mut occurrences);
        span.set_flag(dead.is_some());
        dead
    };

    NormalizedQuery {
        occurrences,
        residuals,
        word_residuals,
        allen_residuals,
        anti_probes,
        slot_widths,
        dead,
    }
}

fn is_membership(field_type: &ValueType, term_type: &ValueType) -> bool {
    field_type.is_interval() && !term_type.is_interval()
}

#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)]
fn lower_atom(
    schema: &Schema,
    signatures: &[&crate::ir::validate::Signature],
    witness: &RuleWitness<'_>,
    idx: usize,
    role: Role,
    atom: &Atom,
) -> Occurrence {
    let occ_id = OccId(u16::try_from(idx).expect("validated: occurrence count fits u16"));

    let field_type = |field: FieldId| -> &ValueType {
        match atom.source {
            crate::ir::AtomSource::Edb(relation_id) => {
                &schema.relation(relation_id).field(field).value_type
            }
            crate::ir::AtomSource::Interior(pred) => {
                signatures[pred.index()].columns[usize::from(field.0)].ty()
            }
        }
    };

    let mut vars: Vec<(FieldId, VarId)> = Vec::new();
    for (field, term) in &atom.bindings {
        if let Term::Var(var) = term {
            if is_membership(field_type(*field), witness.var_type(*var)) {
                continue;
            }
            if !vars.iter().any(|(_, v)| v == var) {
                vars.push((*field, *var));
            }
        }
    }

    let mut filters = Vec::new();
    let mut point_vars = Vec::new();
    for (field, term) in &atom.bindings {
        let field_type = field_type(*field);
        match term {
            Term::Var(var) => {
                if is_membership(field_type, witness.var_type(*var)) {
                    match vars.iter().find(|(_, v)| v == var) {
                        Some((point_field, _)) => filters.push(FilterPredicate::FieldsPointIn {
                            interval: (*field).into(),
                            point: (*point_field).into(),
                        }),
                        None => point_vars.push((*field, *var)),
                    }
                } else {
                    let (first_field, _) = vars
                        .iter()
                        .find(|(_, v)| v == var)
                        .expect("pass 1 recorded every domain-bound variable");
                    if first_field != field {
                        filters.push(FilterPredicate::FieldsCompare {
                            left: (*first_field).into(),
                            right: (*field).into(),
                            op: WordCmp::Eq,
                        });
                    }
                }
            }
            Term::Param(param) => {
                if is_membership(field_type, witness.param_type(*param)) {
                    filters.push(FilterPredicate::PointIn {
                        field: (*field).into(),
                        point: ViewWordSource::Param(*param),
                    });
                } else {
                    filters.push(FilterPredicate::Compare {
                        field: (*field).into(),
                        op: WordCmp::Eq,
                        value: Const::Param(*param),
                    });
                }
            }
            Term::ParamSet(param) => {
                if field_type.is_interval() {
                    filters.push(FilterPredicate::AnyPointIn {
                        field: (*field).into(),
                        set: SetConst::ParamSet(*param),
                    });
                } else {
                    filters.push(FilterPredicate::Compare {
                        field: (*field).into(),
                        op: WordCmp::Eq,
                        value: Const::ParamSet(*param),
                    });
                }
            }
            Term::Literal(value) => {
                let membership = field_type.is_interval()
                    && !matches!(value, Value::IntervalU64(..) | Value::IntervalI64(..));
                if membership {
                    filters.push(FilterPredicate::PointIn {
                        field: (*field).into(),
                        point: ViewWordSource::Word(point_word(value)),
                    });
                } else {
                    filters.push(FilterPredicate::Compare {
                        field: (*field).into(),
                        op: WordCmp::Eq,
                        value: lower_literal(value),
                    });
                }
            }
        }
    }

    Occurrence {
        occ_id,
        role,
        bind: match atom.source {
            crate::ir::AtomSource::Edb(relation) => OccBind::Edb(relation),
            crate::ir::AtomSource::Interior(id) => OccBind::Finished(id),
        },
        vars,
        filters,
        point_vars,
    }
}
