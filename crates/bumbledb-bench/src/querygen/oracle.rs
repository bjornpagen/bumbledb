//! Param binding generation: four seeded draws per query — two
//! in-range hits, one of boundary values, one of guaranteed misses —
//! now carrying **param sets** beside the scalars. Set sizes draw from
//! {0, 1, 2, [`LARGE_BOUNDARY`]}, duplicate elements are injected (the
//! executor must dedup — asserted downstream), and the miss draw
//! applies the per-type miss policies to every element.

use bumbledb::schema::{IntervalElement, ValueType};
use bumbledb::{FieldId, ParamId, Query, RelationId, Term, Value};

use crate::gen::{GenConfig, Rng};
use crate::querygen::target::{self, ids, Domains, AMOUNT_LEVELS, AMOUNT_STEP};
use crate::querygen::{dress, interval_data, DrawKind, PARAM_DRAWS};

/// The large set size: one past the executor's batch width (128), so a
/// single set spans a full batch plus a straggler lane.
pub const LARGE_BOUNDARY: usize = 129;

/// One draw's bindings: scalar values and set element lists, both by
/// dense `ParamId`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamDraw {
    pub scalars: Vec<(ParamId, Value)>,
    pub sets: Vec<(ParamId, Vec<Value>)>,
}

/// How a param is used, with its typing anchor.
#[derive(Clone, Copy)]
pub(super) struct Anchor {
    pub(super) relation: RelationId,
    pub(super) field: FieldId,
    pub(super) set: bool,
    /// The param also occurs at a non-interval position — it is a
    /// *scalar point* even where it binds an interval field
    /// (membership); without one, an interval-field param is an
    /// interval value.
    scalar_anchored: bool,
}

/// Resolves every param's anchor: the (relation, field) that types it —
/// directly for atom bindings (positive and negated), through the
/// variable side for predicates. Prefers a scalar-field position: a
/// membership param's type is its element, established by the scalar
/// anchor the generator constructs.
pub(super) fn param_anchors(query: &Query) -> Vec<Anchor> {
    let schema = target::schema();
    let is_interval = |rel: RelationId, field: FieldId| {
        matches!(
            schema.relation(rel).field(field).value_type,
            ValueType::Interval { .. }
        )
    };
    let mut var_anchor = std::collections::HashMap::new();
    for atom in &query.rules[0].atoms {
        for (field, term) in &atom.bindings {
            if let Term::Var(var) = term {
                if !is_interval(atom.relation, *field) {
                    var_anchor.entry(*var).or_insert((atom.relation, *field));
                }
            }
        }
    }
    let mut count = 0u16;
    for atom in query.rules[0].atoms.iter().chain(&query.rules[0].negated) {
        for (_, term) in &atom.bindings {
            if let Term::Param(p) | Term::ParamSet(p) = term {
                count = count.max(p.0 + 1);
            }
        }
    }
    for comparison in &query.rules[0].predicates {
        for term in [&comparison.lhs, &comparison.rhs] {
            if let Term::Param(p) | Term::ParamSet(p) = term {
                count = count.max(p.0 + 1);
            }
        }
    }
    let mut anchors: Vec<Option<Anchor>> = vec![None; usize::from(count)];
    let place = |anchors: &mut Vec<Option<Anchor>>,
                 param: ParamId,
                 relation: RelationId,
                 field: FieldId,
                 set: bool| {
        let slot = &mut anchors[usize::from(param.0)];
        let scalar = !is_interval(relation, field);
        match slot {
            // A scalar-field position wins over an interval-field one.
            Some(anchor) if anchor.scalar_anchored => {}
            Some(_) if !scalar => {}
            _ => {
                *slot = Some(Anchor {
                    relation,
                    field,
                    set,
                    scalar_anchored: scalar,
                });
            }
        }
    };
    for atom in query.rules[0].atoms.iter().chain(&query.rules[0].negated) {
        for (field, term) in &atom.bindings {
            match term {
                Term::Param(p) => place(&mut anchors, *p, atom.relation, *field, false),
                Term::ParamSet(p) => place(&mut anchors, *p, atom.relation, *field, true),
                _ => {}
            }
        }
    }
    for comparison in &query.rules[0].predicates {
        let (param, set, var) = match (&comparison.lhs, &comparison.rhs) {
            (Term::Param(p), Term::Var(v)) | (Term::Var(v), Term::Param(p)) => (*p, false, *v),
            (Term::ParamSet(p), Term::Var(v)) | (Term::Var(v), Term::ParamSet(p)) => (*p, true, *v),
            _ => continue,
        };
        if let Some((relation, field)) = var_anchor.get(&var) {
            place(&mut anchors, param, *relation, *field, set);
        }
    }
    anchors
        .into_iter()
        .map(|anchor| anchor.expect("validation anchors every param"))
        .collect()
}

/// The dense-id domain of a u64 field (every corpus id is `0..n`).
pub(super) fn u64_domain(rel: RelationId, field: FieldId, domains: &Domains) -> u64 {
    match (rel, field) {
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
                // Boundary alternates the domain's edges.
                DrawKind::Boundary => {
                    if rng.chance(1, 2) {
                        0
                    } else {
                        domain - 1
                    }
                }
                // Out-of-domain, matching the family miss policies.
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
                // Guaranteed miss: no corpus vocabulary starts with this.
                DrawKind::Miss => format!("missing-{}", rng.u64()),
            }
            .into_bytes()
            .into(),
        ),
        ValueType::Enum { variants } => {
            let count = variants.len() as u64;
            Value::Enum(match kind {
                DrawKind::Hit | DrawKind::Miss => u8::try_from(rng.range(count)).expect("small"),
                DrawKind::Boundary => {
                    if rng.chance(1, 2) {
                        0
                    } else {
                        u8::try_from(count - 1).expect("small")
                    }
                }
            })
        }
        // Both bool values are boundary values; every draw kind draws
        // uniformly.
        ValueType::Bool => Value::Bool(rng.chance(1, 2)),
        ValueType::Bytes => match kind {
            // The hit (and boundary) is a real seeded extref; the miss a
            // fresh 16-byte value no corpus row carries.
            DrawKind::Hit | DrawKind::Boundary => target::extref(cfg, rng.range(domains.transfers)),
            DrawKind::Miss => {
                let mut raw = Vec::with_capacity(16);
                for _ in 0..2 {
                    raw.extend_from_slice(&rng.u64().to_le_bytes());
                }
                Value::Bytes(raw.into())
            }
        },
        // An interval-typed param (no scalar anchor): an in-data window
        // literal, whatever the draw kind — hit-vs-miss for interval
        // values is a corpus alignment question, not a vocabulary one.
        ValueType::Interval { element } => {
            let group = rng.range(64);
            let k = rng.range(interval_data::PER_GROUP);
            match element {
                IntervalElement::U64 => {
                    let (start, end) = interval_data::group_u64(cfg.seed, group, k);
                    Value::IntervalU64(start, end)
                }
                IntervalElement::I64 => {
                    let (start, end) = interval_data::group_i64(cfg.seed, group, k);
                    Value::IntervalI64(start, end)
                }
            }
        }
    }
}

/// One set's element list: size from {0, 1, 2, [`LARGE_BOUNDARY`]},
/// elements per the draw kind's policy, and — often — an injected
/// duplicate (dedup is the executor's obligation, exercised here).
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

/// Four param draws per query: two in-range hits, one of boundary
/// values (domain edges — minima and maxima alternate), and one where
/// every string, bytes, and u64 param — scalar or set element — is a
/// guaranteed miss (out of vocabulary or out of domain; i64/enum/bool
/// stay in range).
///
/// # Panics
///
/// On a programmer-invariant violation: an unanchored param (validation
/// anchors every param the grammar emits).
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
                if anchor.set {
                    sets.push((param, set_elements(*anchor, kind, rng, cfg, &domains)));
                } else {
                    scalars.push((param, param_value(*anchor, kind, rng, cfg, &domains)));
                }
            }
            ParamDraw { scalars, sets }
        })
        .collect()
}
