use bumbledb::schema::{IntervalElement, ValueType};
use bumbledb::{AtomSource, FieldId, ParamId, Query, RelationId, Term, Value};

use crate::corpus_gen::{GenConfig, Rng};
use crate::querygen::target::{self, AMOUNT_LEVELS, AMOUNT_STEP, Domains, ids};
use crate::querygen::{DrawKind, PARAM_DRAWS, dress, interval_data};
use crate::walk;

pub const LARGE_BOUNDARY: usize = 129;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamDraw {
    pub scalars: Vec<(ParamId, Value)>,
    pub sets: Vec<(ParamId, Vec<Value>)>,
}

#[derive(Clone, Copy)]
pub(super) struct Anchor {
    pub(super) relation: RelationId,
    pub(super) field: FieldId,
    pub(super) set: bool,

    scalar_anchored: bool,
}

#[derive(Clone, Copy)]
pub(super) enum ParamAnchor {
    Field(Anchor),
}

pub(super) fn param_anchors(query: &Query) -> Vec<ParamAnchor> {
    let schema = target::schema();
    let is_interval = |rel: RelationId, field: FieldId| {
        schema.relation(rel).field(field).value_type.is_interval()
    };
    let mut count = 0u16;
    for rule in walk::rules(query) {
        for atom in rule.atoms.iter().chain(&rule.negated) {
            for (_, term) in &atom.bindings {
                if let Term::Param(p) | Term::ParamSet(p) = term {
                    count = count.max(p.0 + 1);
                }
            }
        }
        for comparison in rule.conditions.iter().map(super::leaf) {
            for term in [&comparison.lhs, &comparison.rhs] {
                if let Term::Param(p) | Term::ParamSet(p) = term {
                    count = count.max(p.0 + 1);
                }
            }
        }
    }
    let mut anchors: Vec<Option<ParamAnchor>> = vec![None; usize::from(count)];
    let place = |anchors: &mut Vec<Option<ParamAnchor>>,
                 param: ParamId,
                 relation: RelationId,
                 field: FieldId,
                 set: bool| {
        let slot = &mut anchors[usize::from(param.0)];
        let scalar = !is_interval(relation, field);
        match slot {

            Some(ParamAnchor::Field(anchor)) if anchor.scalar_anchored => {}
            Some(ParamAnchor::Field(_)) if !scalar => {}
            _ => {
                *slot = Some(ParamAnchor::Field(Anchor {
                    relation,
                    field,
                    set,
                    scalar_anchored: scalar,
                }));
            }
        }
    };
    for rule in walk::rules(query) {
        let mut var_anchor = std::collections::HashMap::new();
        for atom in &rule.atoms {
            let AtomSource::Edb(relation) = atom.source else {
                continue;
            };
            for (field, term) in &atom.bindings {
                if let Term::Var(var) = term
                    && !is_interval(relation, *field)
                {
                    var_anchor.entry(*var).or_insert((relation, *field));
                }
            }
        }
        for atom in rule.atoms.iter().chain(&rule.negated) {
            let AtomSource::Edb(relation) = atom.source else {
                continue;
            };
            for (field, term) in &atom.bindings {
                match term {
                    Term::Param(p) => place(&mut anchors, *p, relation, *field, false),
                    Term::ParamSet(p) => place(&mut anchors, *p, relation, *field, true),
                    _ => {}
                }
            }
        }
        for comparison in rule.conditions.iter().map(super::leaf) {
            let (param, set, var) = match (&comparison.lhs, &comparison.rhs) {
                (Term::Param(p), Term::Var(v)) | (Term::Var(v), Term::Param(p)) => (*p, false, *v),
                (Term::ParamSet(p), Term::Var(v)) | (Term::Var(v), Term::ParamSet(p)) => {
                    (*p, true, *v)
                }
                _ => continue,
            };
            if let Some((relation, field)) = var_anchor.get(&var) {
                place(&mut anchors, param, *relation, *field, set);
            }
        }
    }
    anchors
        .into_iter()
        .map(|anchor| anchor.expect("validation anchors every param"))
        .collect()
}

pub(super) fn u64_domain(rel: RelationId, field: FieldId, domains: &Domains) -> u64 {
    match (rel, field) {

        (ids::ACCOUNT, ids::account::CURRENCY)
        | (ids::JOURNAL_ENTRY, ids::journal_entry::SOURCE)
        | (ids::POSTING_TAG, ids::posting_tag::TAG)
        | (ids::CURRENCY_BACKING, ids::currency_backing::CURRENCY)
        | (ids::CASH_ROUNDING, ids::cash_rounding::CURRENCY)
        | (ids::CURRENCY | ids::SOURCE | ids::TAG, _) => 3,
        (ids::POSTING, ids::posting::ENTRY) | (ids::JOURNAL_ENTRY, ids::journal_entry::ID) => {
            domains.entries
        }
        (ids::POSTING, ids::posting::ACCOUNT)
        | (ids::ACCOUNT, ids::account::ID)
        | (ids::MANDATE, ids::mandate::ACCOUNT) => domains.accounts,
        (ids::POSTING, ids::posting::INSTRUMENT) | (ids::INSTRUMENT, ids::instrument::ID) => {
            domains.instruments
        }
        (ids::ACCOUNT, ids::account::HOLDER) | (ids::HOLDER, ids::holder::ID) => domains.holders,
        (ids::POSTING, ids::posting::ID) | (ids::POSTING_TAG, ids::posting_tag::POSTING) => {
            domains.postings
        }
        (ids::ORG, ids::org::ID) | (ids::ORG_PARENT, _) | (ids::MANDATE, ids::mandate::ORG) => {
            domains.orgs
        }
        (ids::TRANSFER, ids::transfer::ID) => domains.transfers,
        _ => domains.postings,
    }
}

fn param_value(
    anchor: Anchor,
    kind: DrawKind,
    rng: &mut Rng,
    cfg: GenConfig,
    domains: &Domains,
) -> Value {
    let (rel, field) = (anchor.relation, anchor.field);
    let ty = &target::schema().relation(rel).field(field).value_type;
    match ty {
        ValueType::U64 => {
            let domain = u64_domain(rel, field, domains).max(1);
            Value::U64(match kind {
                DrawKind::Hit => rng.range(domain),

                DrawKind::Boundary => {
                    if rng.chance(1, 2) {
                        0
                    } else {
                        domain - 1
                    }
                }

                DrawKind::Miss => domain + 1 + rng.range(domain),
            })
        }
        ValueType::I64 => {
            let (lo, hi) = if (rel, field) == (ids::POSTING, ids::posting::AMOUNT) {
                (
                    -(AMOUNT_LEVELS / 2) * AMOUNT_STEP,
                    (AMOUNT_LEVELS / 2) * AMOUNT_STEP,
                )
            } else {
                dress::at_window(domains)
            };
            Value::I64(match kind {
                DrawKind::Hit | DrawKind::Miss => {
                    lo + i64::try_from(rng.range(u64::try_from(hi - lo).expect("ordered")))
                        .expect("fits")
                }
                DrawKind::Boundary => {
                    if rng.chance(1, 2) {
                        lo
                    } else {
                        hi
                    }
                }
            })
        }
        ValueType::String => Value::String(
            match kind {
                DrawKind::Hit | DrawKind::Boundary => target::string_hit(rel, field, rng),

                DrawKind::Miss => format!("missing-{}", rng.u64()),
            }
            .into(),
        ),

        ValueType::Bool => Value::Bool(rng.chance(1, 2)),

        ValueType::FixedBytes { len } => {
            let hit = if *len == 32 {
                target::extref(cfg, rng.range(domains.transfers))
            } else {
                target::digest_vocab_value(*len, rng.range(target::DIGEST_VOCAB))
            };
            match kind {
                DrawKind::Hit | DrawKind::Boundary => hit,
                DrawKind::Miss => {
                    let Value::FixedBytes(mut raw) = hit else {
                        unreachable!("digests are bytes<N>")
                    };

                    raw[0] = 0xA5;
                    Value::FixedBytes(raw)
                }
            }
        }

        ValueType::Interval { element } | ValueType::FixedInterval { element, .. } => {
            let group = rng.range(64);
            match element {
                IntervalElement::U64 => {
                    let ((start, end), _) = interval_data::ladder_u64(cfg.seed, group, rng);
                    Value::IntervalU64(
                        bumbledb::Interval::<u64>::new(start, end).expect("nonempty interval"),
                    )
                }
                IntervalElement::I64 => {
                    let ((start, end), _) = interval_data::ladder_i64(cfg.seed, group, rng);
                    Value::IntervalI64(
                        bumbledb::Interval::<i64>::new(start, end).expect("nonempty interval"),
                    )
                }
            }
        }
    }
}

fn set_elements(
    anchor: Anchor,
    kind: DrawKind,
    rng: &mut Rng,
    cfg: GenConfig,
    domains: &Domains,
) -> Vec<Value> {
    let size = match rng.range(8) {
        0 => 0,
        1 | 2 => 1,
        3..=5 => 2,
        _ => LARGE_BOUNDARY,
    };
    let mut elements: Vec<Value> = (0..size)
        .map(|_| param_value(anchor, kind, rng, cfg, domains))
        .collect();
    if elements.len() >= 2 && rng.chance(3, 10) {
        elements[1] = elements[0].clone();
    }
    elements
}

/// # Panics
/// On a programmer-invariant violation: an unanchored param (validation
#[must_use]
pub fn params_for(query: &Query, rng: &mut Rng, cfg: GenConfig) -> Vec<ParamDraw> {
    let domains = Domains::of(cfg.scale);
    let anchors = param_anchors(query);
    (0..PARAM_DRAWS)
        .map(|draw| {
            let kind = match draw {
                0 | 1 => DrawKind::Hit,
                2 => DrawKind::Boundary,
                _ => DrawKind::Miss,
            };
            let mut scalars = Vec::new();
            let mut sets = Vec::new();
            for (index, anchor) in anchors.iter().enumerate() {
                let param = ParamId(u16::try_from(index).expect("dense params fit"));
                match anchor {
                    ParamAnchor::Field(anchor) if anchor.set => {
                        sets.push((param, set_elements(*anchor, kind, rng, cfg, &domains)));
                    }
                    ParamAnchor::Field(anchor) => {
                        scalars.push((param, param_value(*anchor, kind, rng, cfg, &domains)));
                    }
                }
            }
            ParamDraw { scalars, sets }
        })
        .collect()
}
