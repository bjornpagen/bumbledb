use bumbledb::{FieldId, Query, RelationId, Term, Value};

use crate::gen::{self, GenConfig, Rng, Sizes};
use crate::querygen::construct::extref_of;
use crate::querygen::dress_posting::posting_at_window;
use crate::querygen::{SetKind, PARAM_SETS};
use crate::schema::ids;

/// Resolves every param's anchor: the (relation, field) that types it —
/// directly for atom bindings, through the variable side for predicates.
pub(super) fn param_anchors(query: &Query) -> Vec<(RelationId, FieldId)> {
    let mut var_anchor = std::collections::HashMap::new();
    for atom in &query.atoms {
        for (field, term) in &atom.bindings {
            if let Term::Var(var) = term {
                var_anchor.entry(*var).or_insert((atom.relation, *field));
            }
        }
    }
    let count = usize::from(query.atoms.iter().flat_map(|a| &a.bindings).fold(
        0u16,
        |max, (_, term)| match term {
            Term::Param(p) => max.max(p.0 + 1),
            _ => max,
        },
    ))
    .max(usize::from(query.predicates.iter().fold(
        0u16,
        |max, c| match (&c.lhs, &c.rhs) {
            (Term::Param(p), _) | (_, Term::Param(p)) => max.max(p.0 + 1),
            _ => max,
        },
    )));
    let mut anchors = vec![None; count];
    for atom in &query.atoms {
        for (field, term) in &atom.bindings {
            if let Term::Param(p) = term {
                anchors[usize::from(p.0)] = Some((atom.relation, *field));
            }
        }
    }
    for comparison in &query.predicates {
        let ((Term::Param(param), Term::Var(var)) | (Term::Var(var), Term::Param(param))) =
            (&comparison.lhs, &comparison.rhs)
        else {
            continue;
        };
        if anchors[usize::from(param.0)].is_none() {
            anchors[usize::from(param.0)] = var_anchor.get(var).copied();
        }
    }
    anchors
        .into_iter()
        .map(|anchor| anchor.expect("validation anchors every param"))
        .collect()
}

/// The dense-id domain of a u64 field (every corpus id is `0..n`).
pub(super) fn u64_domain(rel: RelationId, field: FieldId, sizes: &Sizes) -> u64 {
    match (rel, field) {
        (ids::POSTING, ids::posting::TRANSFER) => sizes.transfers,
        (ids::POSTING, ids::posting::ACCOUNT) | (ids::ACCOUNT_TAG, ids::account_tag::ACCOUNT) => {
            sizes.accounts
        }
        (ids::POSTING, ids::posting::INSTRUMENT) | (ids::INSTRUMENT, ids::instrument::ID) => {
            sizes.instruments
        }
        (ids::ACCOUNT, ids::account::HOLDER) => sizes.holders,
        (ids::ACCOUNT, ids::account::CURRENCY) | (ids::INSTRUMENT, ids::instrument::CURRENCY) => {
            sizes.currencies
        }
        (ids::ACCOUNT_TAG, ids::account_tag::TAG) => sizes.tags,
        _ => sizes.rows(rel),
    }
}

fn string_hit(rel: RelationId, field: FieldId, rng: &mut Rng) -> String {
    match (rel, field) {
        (ids::CURRENCY, ids::currency::CODE) => format!("CUR{:02}", rng.range(16)),
        (ids::HOLDER, ids::holder::NAME) => format!("holder-{}", rng.range(gen::MEMO_VOCAB)),
        (ids::INSTRUMENT, ids::instrument::SYMBOL) => format!("SYM{:04}", rng.range(512)),
        (ids::TAG, ids::tag::LABEL) => format!("tag-{:03}", rng.range(256)),
        (ids::TAG_NOTE, ids::tag_note::NOTE) => format!("note-{}", rng.range(gen::MEMO_VOCAB)),
        _ => format!("m{}", rng.range(gen::MEMO_VOCAB)),
    }
}

fn param_value(
    anchor: (RelationId, FieldId),
    kind: SetKind,
    rng: &mut Rng,
    cfg: GenConfig,
    sizes: &Sizes,
) -> Value {
    use bumbledb::schema::ValueType;
    let (rel, field) = anchor;
    let ty = &crate::schema::schema()
        .relation(rel)
        .field(field)
        .value_type;
    match ty {
        ValueType::U64 => {
            let domain = u64_domain(rel, field, sizes).max(1);
            Value::U64(match kind {
                SetKind::Hit => rng.range(domain),
                // Boundary alternates the domain's edges.
                SetKind::Boundary => {
                    if rng.chance(1, 2) {
                        0
                    } else {
                        domain - 1
                    }
                }
                // Out-of-domain, matching the family miss policies.
                SetKind::Miss => domain + 1 + rng.range(domain),
            })
        }
        ValueType::I64 => {
            let (lo, hi) = match (rel, field) {
                (ids::POSTING, ids::posting::AMOUNT) => (-5_000_000, 5_000_000),
                (ids::ACCOUNT, ids::account::OPENED_AT) => (gen::AT_BASE - (1 << 30), gen::AT_BASE),
                _ => posting_at_window(sizes),
            };
            Value::I64(match kind {
                SetKind::Hit | SetKind::Miss => {
                    lo + i64::try_from(rng.range(u64::try_from(hi - lo).expect("ordered")))
                        .expect("fits")
                }
                SetKind::Boundary => {
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
                SetKind::Hit | SetKind::Boundary => string_hit(rel, field, rng),
                // Guaranteed miss: no corpus vocabulary starts with this.
                SetKind::Miss => format!("missing-{}", rng.u64()),
            }
            .into_bytes()
            .into(),
        ),
        ValueType::Enum { variants } => {
            let count = variants.len() as u64;
            Value::Enum(match kind {
                SetKind::Hit | SetKind::Miss => u8::try_from(rng.range(count)).expect("small"),
                SetKind::Boundary => {
                    if rng.chance(1, 2) {
                        0
                    } else {
                        u8::try_from(count - 1).expect("small")
                    }
                }
            })
        }
        // Both bool values are boundary values; every set kind draws
        // uniformly.
        ValueType::Bool => Value::Bool(rng.chance(1, 2)),
        ValueType::Bytes => match kind {
            // The hit (and boundary) is a real seeded extref; the miss a
            // fresh 16-byte value no corpus row carries.
            SetKind::Hit | SetKind::Boundary => extref_of(cfg, sizes, rng.range(sizes.transfers)),
            SetKind::Miss => {
                let mut raw = Vec::with_capacity(16);
                for _ in 0..2 {
                    raw.extend_from_slice(&rng.u64().to_le_bytes());
                }
                Value::Bytes(raw.into())
            }
        },
    }
}

/// Four param sets per query: two in-range hits, one of boundary values
/// (domain edges — minima and maxima alternate), and one where every
/// string, bytes, and u64 param is a guaranteed miss (out of vocabulary
/// or out of domain; i64/enum/bool params stay in range).
#[must_use]
pub fn params_for(query: &Query, rng: &mut Rng, cfg: GenConfig) -> Vec<Vec<Value>> {
    let sizes = Sizes::of(cfg.scale);
    let anchors = param_anchors(query);
    (0..PARAM_SETS)
        .map(|set| {
            let kind = match set {
                0 | 1 => SetKind::Hit,
                2 => SetKind::Boundary,
                _ => SetKind::Miss,
            };
            anchors
                .iter()
                .map(|anchor| param_value(*anchor, kind, rng, cfg, &sizes))
                .collect()
        })
        .collect()
}
