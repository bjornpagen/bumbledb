//! The randomized query generator (docs/architecture/50-validation.md): seeded random
//! valid queries over the ledger schema — the fuel for `verify`'s
//! randomized half.
//!
//! Construction is correct **by construction**: fresh dense `VarId`s,
//! dense `ParamId`s allocated at their use site, and literals typed from
//! the schema walk. The engine's `validate` is the assertion, not the
//! filter — a generated query failing validation is a generator bug.

use bumbledb::{
    AggOp, Atom, CmpOp, Comparison, FieldId, FindTerm, ParamId, Query, RelationId, Term, Value,
    VarId,
};

use crate::gen::{self, GenConfig, Rng, Sizes};
use crate::schema::ids;

/// The shape grammar's weights (drawn by range over the sum — the PRD's
/// percentages, normative):
/// guard 10, star 20, chain 20, self-join 10, gated 10, aggregate 20.
const SHAPE_WEIGHTS: &[(Shape, u64)] = &[
    (Shape::Guard, 10),
    (Shape::Star, 20),
    (Shape::Chain, 20),
    (Shape::SelfJoin, 10),
    (Shape::Gated, 10),
    (Shape::Aggregate, 20),
];

/// Filter dressing applies to every shape with this percent chance…
const DRESS_PCT: u64 = 60;
/// …and the repeated in-atom variable to qualifying atoms with this one.
const REPEAT_VAR_PCT: u64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Guard,
    Star,
    Chain,
    SelfJoin,
    Gated,
    Aggregate,
}

/// Accumulating query state: atoms, predicates, finds, fresh id counters,
/// and the registry of variables the shapes bound (group-key candidates).
#[derive(Default)]
struct Builder {
    atoms: Vec<Atom>,
    predicates: Vec<Comparison>,
    finds: Vec<FindTerm>,
    next_var: u16,
    next_param: u16,
    bound: Vec<VarId>,
    /// Whether dressing emitted an out-of-vocabulary string or bytes
    /// literal.
    miss: bool,
    /// Whether dressing emitted an in-vocabulary bytes literal (a
    /// recomputed extref) / an out-of-vocabulary one.
    bytes_hit: bool,
    bytes_miss: bool,
}

impl Builder {
    fn fresh_var(&mut self) -> VarId {
        let var = VarId(self.next_var);
        self.next_var += 1;
        var
    }

    fn fresh_param(&mut self) -> ParamId {
        let param = ParamId(self.next_param);
        self.next_param += 1;
        param
    }

    fn atom(&mut self, relation: RelationId) -> usize {
        self.atoms.push(Atom {
            relation,
            bindings: Vec::new(),
        });
        self.atoms.len() - 1
    }

    fn bind(&mut self, atom: usize, field: FieldId, term: Term) {
        debug_assert!(
            !self.atoms[atom].bindings.iter().any(|(f, _)| *f == field),
            "duplicate field binding"
        );
        self.atoms[atom].bindings.push((field, term));
    }

    /// A fresh variable bound to the field, registered as a group-key
    /// candidate.
    fn bind_var(&mut self, atom: usize, field: FieldId) -> VarId {
        let var = self.fresh_var();
        self.bind(atom, field, Term::Var(var));
        self.bound.push(var);
        var
    }

    /// The variable already bound to the field, binding a fresh one if the
    /// field is free; `None` when the field is bound to a non-variable.
    fn var_at(&mut self, atom: usize, field: FieldId) -> Option<VarId> {
        match self.atoms[atom].bindings.iter().find(|(f, _)| *f == field) {
            Some((_, Term::Var(var))) => Some(*var),
            Some(_) => None,
            None => Some(self.bind_var(atom, field)),
        }
    }

    fn find_var(&mut self, var: VarId) {
        self.finds.push(FindTerm::Var(var));
    }

    fn into_query(self) -> Query {
        Query {
            finds: self.finds,
            atoms: self.atoms,
            predicates: self.predicates,
        }
    }
}

/// Guardable relations: (relation, serial-id field, projectable fields).
const GUARDABLE: &[(RelationId, FieldId, &[FieldId])] = &[
    (ids::CURRENCY, ids::currency::ID, &[ids::currency::CODE]),
    (
        ids::HOLDER,
        ids::holder::ID,
        &[ids::holder::NAME, ids::holder::REGION],
    ),
    (
        ids::INSTRUMENT,
        ids::instrument::ID,
        &[
            ids::instrument::SYMBOL,
            ids::instrument::CURRENCY,
            ids::instrument::KIND,
        ],
    ),
    (
        ids::ACCOUNT,
        ids::account::ID,
        &[
            ids::account::HOLDER,
            ids::account::STATUS,
            ids::account::OPENED_AT,
        ],
    ),
    (
        ids::TRANSFER,
        ids::transfer::ID,
        &[ids::transfer::AT, ids::transfer::EXTREF],
    ),
    (
        ids::POSTING,
        ids::posting::ID,
        &[
            ids::posting::ACCOUNT,
            ids::posting::AMOUNT,
            ids::posting::AT,
            ids::posting::MEMO,
        ],
    ),
    (ids::TAG, ids::tag::ID, &[ids::tag::LABEL]),
];

/// One atom, serial id bound to a param, 1–2 vars projected.
fn guard(b: &mut Builder, rng: &mut Rng) {
    let idx = usize::try_from(rng.range(GUARDABLE.len() as u64)).expect("small");
    let (relation, id, fields) = GUARDABLE[idx];
    let atom = b.atom(relation);
    let param = b.fresh_param();
    b.bind(atom, id, Term::Param(param));
    let take = 1 + usize::try_from(rng.range(2)).expect("small");
    let start = usize::try_from(rng.range(fields.len() as u64)).expect("small");
    for k in 0..take.min(fields.len()) {
        let field = fields[(start + k) % fields.len()];
        let var = b.bind_var(atom, field);
        b.find_var(var);
    }
}

/// Star satellites: (posting FK field, relation, projected payload field).
const SATELLITES: &[(FieldId, RelationId, FieldId)] = &[
    (ids::posting::ACCOUNT, ids::ACCOUNT, ids::account::STATUS),
    (
        ids::posting::INSTRUMENT,
        ids::INSTRUMENT,
        ids::instrument::KIND,
    ),
    (ids::posting::TRANSFER, ids::TRANSFER, ids::transfer::AT),
];

/// Posting joined to 1–3 of {Account, Instrument, Transfer} on its FK
/// fields, projecting amount plus each satellite's payload.
fn star(b: &mut Builder, rng: &mut Rng) {
    let posting = b.atom(ids::POSTING);
    let amount = b.bind_var(posting, ids::posting::AMOUNT);
    b.find_var(amount);
    let take = 1 + usize::try_from(rng.range(3)).expect("small");
    let start = usize::try_from(rng.range(SATELLITES.len() as u64)).expect("small");
    for k in 0..take {
        let (fk, relation, payload) = SATELLITES[(start + k) % SATELLITES.len()];
        let join = b.bind_var(posting, fk);
        let satellite = b.atom(relation);
        b.bind(satellite, FieldId(0), Term::Var(join));
        let projected = b.bind_var(satellite, payload);
        b.find_var(projected);
    }
    repeat_var(b, rng, posting);
}

/// Holder ← Account ← Posting (2–3 hops), projecting the ends.
fn chain(b: &mut Builder, rng: &mut Rng) {
    let posting = b.atom(ids::POSTING);
    let amount = b.bind_var(posting, ids::posting::AMOUNT);
    b.find_var(amount);
    let account_join = b.bind_var(posting, ids::posting::ACCOUNT);
    let account = b.atom(ids::ACCOUNT);
    b.bind(account, ids::account::ID, Term::Var(account_join));
    if rng.chance(1, 2) {
        // Three hops: through to Holder, projecting its name.
        let holder_join = b.bind_var(account, ids::account::HOLDER);
        let holder = b.atom(ids::HOLDER);
        b.bind(holder, ids::holder::ID, Term::Var(holder_join));
        let name = b.bind_var(holder, ids::holder::NAME);
        b.find_var(name);
    } else {
        let opened = b.bind_var(account, ids::account::OPENED_AT);
        b.find_var(opened);
    }
    repeat_var(b, rng, posting);
}

/// Two Posting occurrences equated on `transfer`, projecting both
/// amounts — and, half the time, a cross-atom ordered residual between
/// them (`x < y` and friends): the randomized twin of the spread
/// family, exercising residual placement and survivor compaction.
fn self_join(b: &mut Builder, rng: &mut Rng) {
    let first = b.atom(ids::POSTING);
    let transfer = b.bind_var(first, ids::posting::TRANSFER);
    let x = b.bind_var(first, ids::posting::AMOUNT);
    let second = b.atom(ids::POSTING);
    b.bind(second, ids::posting::TRANSFER, Term::Var(transfer));
    let y = b.bind_var(second, ids::posting::AMOUNT);
    b.find_var(x);
    b.find_var(y);
    if rng.chance(1, 2) {
        b.predicates.push(Comparison {
            op: order_op(rng),
            lhs: Term::Var(x),
            rhs: Term::Var(y),
        });
    }
    repeat_var(b, rng, first);
}

/// The repeated in-atom variable ([`REPEAT_VAR_PCT`]% of qualifying
/// Posting atoms): `at` rebound to the `amount` variable — two same-typed
/// (i64) fields of one atom carrying one variable.
fn repeat_var(b: &mut Builder, rng: &mut Rng, posting: usize) {
    if !rng.chance(REPEAT_VAR_PCT, 100) {
        return;
    }
    let amount = b.atoms[posting]
        .bindings
        .iter()
        .find_map(|(f, t)| (*f == ids::posting::AMOUNT).then(|| t.clone()));
    let at_free = !b.atoms[posting]
        .bindings
        .iter()
        .any(|(f, _)| *f == ids::posting::AT);
    if let (Some(term @ Term::Var(_)), true) = (amount, at_free) {
        b.bind(posting, ids::posting::AT, term);
    }
}

/// Any join shape re-projected as group-by + one aggregate (sometimes
/// two); group key = 0–2 of the shape's bound variables. Aggregate
/// targets cover both integer types: i64 (amount/at) and u64 (the
/// posting's account id — Sum over it is provably bounded: the fold is
/// over distinct bindings, so any group's sum is at most
/// postings × accounts ≤ 10⁷ × 5 × 10⁴ = 5 × 10¹¹ ≪ 2⁶³ at every scale,
/// satisfying the Sum-range rule). A fifth of the time the posting's
/// bool field joins the group-key candidates.
fn aggregate(b: &mut Builder, rng: &mut Rng) {
    if rng.chance(1, 2) {
        star(b, rng);
    } else {
        chain(b, rng);
    }
    let amount = b
        .var_at(0, ids::posting::AMOUNT)
        .expect("shape binds amount");
    let at = b.var_at(0, ids::posting::AT).expect("var or fresh");
    if rng.chance(1, 5) {
        // A bool group-key candidate (registered by bind_var).
        let _ = b.var_at(0, ids::posting::RECONCILED);
    }
    let (op, over) = match rng.range(7) {
        0 => (AggOp::Sum, Some(amount)),
        1 => (AggOp::Count, None),
        2 => (AggOp::Min, Some(at)),
        3 => (AggOp::Max, Some(amount)),
        // The u64 targets (account: dense ids, bounded sums).
        4 => (AggOp::Sum, b.var_at(0, ids::posting::ACCOUNT)),
        5 => (AggOp::Min, b.var_at(0, ids::posting::ACCOUNT)),
        _ => (AggOp::Max, b.var_at(0, ids::posting::ACCOUNT)),
    };
    let candidates: Vec<VarId> = b
        .bound
        .iter()
        .copied()
        .filter(|var| Some(*var) != over)
        .collect();
    let group = usize::try_from(rng.range(3))
        .expect("small")
        .min(candidates.len());
    let start = if candidates.is_empty() {
        0
    } else {
        usize::try_from(rng.range(candidates.len() as u64)).expect("small")
    };
    b.finds.clear();
    let mut key: Vec<VarId> = (0..group)
        .map(|k| candidates[(start + k) % candidates.len()])
        .collect();
    key.sort_unstable();
    key.dedup();
    let in_key = |var: Option<VarId>| var.is_some_and(|v| key.contains(&v));
    for var in &key {
        b.find_var(*var);
    }
    b.finds.push(FindTerm::Aggregate { op, over });
    // Multi-aggregate finds, a quarter of the time: Count beside any
    // valued aggregate (always distinct), or Sum(amount) beside Count
    // when amount stays off the group key.
    if rng.chance(1, 4) {
        let amount_term = FindTerm::Aggregate {
            op: AggOp::Sum,
            over: Some(amount),
        };
        if op == AggOp::Count {
            if !in_key(Some(amount)) {
                b.finds.push(amount_term);
            }
        } else {
            b.finds.push(FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            });
        }
    }
}

/// One order operator, uniformly.
fn order_op(rng: &mut Rng) -> CmpOp {
    match rng.range(4) {
        0 => CmpOp::Lt,
        1 => CmpOp::Le,
        2 => CmpOp::Gt,
        _ => CmpOp::Ge,
    }
}

/// Any of the six operators, uniformly (integer dressing — every legal
/// (op, integer-type) cell of the coverage matrix must be reachable).
fn any_op(rng: &mut Rng) -> CmpOp {
    match rng.range(6) {
        0 => CmpOp::Eq,
        1 => CmpOp::Ne,
        2 => CmpOp::Lt,
        3 => CmpOp::Le,
        4 => CmpOp::Gt,
        _ => CmpOp::Ge,
    }
}

/// An i64 predicate on the field (any operator): literal or param, 50/50.
fn i64_dress(b: &mut Builder, rng: &mut Rng, atom: usize, field: FieldId, lo: i64, hi: i64) {
    let Some(var) = b.var_at(atom, field) else {
        return;
    };
    let op = any_op(rng);
    let width = u64::try_from(hi - lo).expect("ordered window");
    let rhs = if rng.chance(1, 2) {
        Term::Literal(Value::I64(
            lo + i64::try_from(rng.range(width.max(1))).expect("fits"),
        ))
    } else {
        Term::Param(b.fresh_param())
    };
    b.predicates.push(Comparison {
        op,
        lhs: Term::Var(var),
        rhs,
    });
}

/// A u64 predicate on a dense-id field (any operator): the literal or
/// param draws in-domain, so ordered comparisons select real slices.
fn u64_dress(b: &mut Builder, rng: &mut Rng, atom: usize, field: FieldId, domain: u64) {
    let Some(var) = b.var_at(atom, field) else {
        return;
    };
    let op = any_op(rng);
    let rhs = if rng.chance(1, 2) {
        Term::Literal(Value::U64(rng.range(domain.max(1))))
    } else {
        Term::Param(b.fresh_param())
    };
    b.predicates.push(Comparison {
        op,
        lhs: Term::Var(var),
        rhs,
    });
}

/// An `Eq`/`Ne` predicate against an enum-ordinal literal.
fn enum_cmp(b: &mut Builder, rng: &mut Rng, atom: usize, field: FieldId, variants: u64) {
    let Some(var) = b.var_at(atom, field) else {
        return;
    };
    let op = if rng.chance(1, 2) {
        CmpOp::Eq
    } else {
        CmpOp::Ne
    };
    let ordinal = u8::try_from(rng.range(variants)).expect("small");
    b.predicates.push(Comparison {
        op,
        lhs: Term::Var(var),
        rhs: Term::Literal(Value::Enum(ordinal)),
    });
}

/// The i64 windows the corpus draws from, per field (dressing literals
/// land inside them so range predicates select real subsets).
fn posting_at_window(sizes: &Sizes) -> (i64, i64) {
    let span = i64::try_from(sizes.postings).expect("fits") * gen::AT_STEP;
    (gen::AT_BASE, gen::AT_BASE + span)
}

/// One dressing predicate on a Posting atom.
fn dress_posting(b: &mut Builder, rng: &mut Rng, atom: usize, sizes: &Sizes) {
    match rng.range(6) {
        0 => i64_dress(b, rng, atom, ids::posting::AMOUNT, -5_000_000, 5_000_000),
        5 => {
            // U64 dressing on a dense-id FK field: ordered comparisons
            // (and Eq/Ne) over real id slices.
            let (field, domain) = match rng.range(3) {
                0 => (ids::posting::ACCOUNT, sizes.accounts),
                1 => (ids::posting::INSTRUMENT, sizes.instruments),
                _ => (ids::posting::TRANSFER, sizes.transfers),
            };
            u64_dress(b, rng, atom, field, domain);
        }
        1 => {
            let (lo, hi) = posting_at_window(sizes);
            i64_dress(b, rng, atom, ids::posting::AT, lo, hi);
        }
        2 => {
            // Eq/Ne on memo: in-vocabulary literal, out-of-vocabulary
            // literal (the miss path), or a param — equal weight.
            let Some(var) = b.var_at(atom, ids::posting::MEMO) else {
                return;
            };
            let op = if rng.chance(1, 2) {
                CmpOp::Eq
            } else {
                CmpOp::Ne
            };
            let rhs = match rng.range(3) {
                0 => Term::Literal(Value::String(
                    format!("m{}", rng.range(gen::MEMO_VOCAB))
                        .into_bytes()
                        .into(),
                )),
                1 => {
                    b.miss = true;
                    Term::Literal(Value::String(
                        format!("missing-{}", rng.u64()).into_bytes().into(),
                    ))
                }
                _ => Term::Param(b.fresh_param()),
            };
            b.predicates.push(Comparison {
                op,
                lhs: Term::Var(var),
                rhs,
            });
        }
        3 => {
            let Some(var) = b.var_at(atom, ids::posting::RECONCILED) else {
                return;
            };
            let op = if rng.chance(1, 2) {
                CmpOp::Eq
            } else {
                CmpOp::Ne
            };
            b.predicates.push(Comparison {
                op,
                lhs: Term::Var(var),
                rhs: Term::Literal(Value::Bool(rng.chance(1, 2))),
            });
        }
        _ => {
            // Same-atom var-vs-var: amount vs at, the same-typed (i64)
            // pair. Skipped when the repeated-var pass fused them (a
            // self-comparison is invalid by the roster).
            let (Some(amount), Some(at)) = (
                b.var_at(atom, ids::posting::AMOUNT),
                b.var_at(atom, ids::posting::AT),
            ) else {
                return;
            };
            if amount == at {
                return;
            }
            let op = match rng.range(6) {
                0 => CmpOp::Eq,
                1 => CmpOp::Ne,
                2 => CmpOp::Lt,
                3 => CmpOp::Le,
                4 => CmpOp::Gt,
                _ => CmpOp::Ge,
            };
            b.predicates.push(Comparison {
                op,
                lhs: Term::Var(amount),
                rhs: Term::Var(at),
            });
        }
    }
}

/// Filter dressing ([`DRESS_PCT`]% of queries, 1–3 predicates): i64 range
/// ops on amount/at, Eq/Ne on memo (hit, miss, or param), Eq on
/// enums/bools, and same-typed var-vs-var — per the dressed atom's
/// relation.
fn dress(b: &mut Builder, rng: &mut Rng, cfg: GenConfig, sizes: &Sizes) {
    if !rng.chance(DRESS_PCT, 100) {
        return;
    }
    let count = 1 + rng.range(3);
    for _ in 0..count {
        let dressable: Vec<usize> = b
            .atoms
            .iter()
            .enumerate()
            .filter(|(_, atom)| !atom.bindings.is_empty())
            .map(|(index, _)| index)
            .collect();
        let atom = dressable[usize::try_from(rng.range(dressable.len() as u64)).expect("small")];
        match b.atoms[atom].relation {
            ids::POSTING => dress_posting(b, rng, atom, sizes),
            ids::ACCOUNT => {
                if rng.chance(1, 2) {
                    enum_cmp(b, rng, atom, ids::account::STATUS, 3);
                } else {
                    i64_dress(
                        b,
                        rng,
                        atom,
                        ids::account::OPENED_AT,
                        gen::AT_BASE - (1 << 30),
                        gen::AT_BASE,
                    );
                }
            }
            ids::INSTRUMENT => enum_cmp(b, rng, atom, ids::instrument::KIND, 4),
            ids::HOLDER => enum_cmp(b, rng, atom, ids::holder::REGION, 4),
            ids::TRANSFER => {
                if rng.chance(1, 2) {
                    // Bytes Eq/Ne on extref: the hit literal is the
                    // *actual* extref of a seeded row (recomputed via
                    // gen::row — the corpus is a pure function of the
                    // config); the miss is a fresh 16-byte value.
                    let Some(var) = b.var_at(atom, ids::transfer::EXTREF) else {
                        continue;
                    };
                    let op = if rng.chance(1, 2) {
                        CmpOp::Eq
                    } else {
                        CmpOp::Ne
                    };
                    let rhs = match rng.range(3) {
                        0 => {
                            b.bytes_hit = true;
                            Term::Literal(extref_of(cfg, sizes, rng.range(sizes.transfers)))
                        }
                        1 => {
                            b.miss = true;
                            b.bytes_miss = true;
                            let mut raw = Vec::with_capacity(16);
                            for _ in 0..2 {
                                raw.extend_from_slice(&rng.u64().to_le_bytes());
                            }
                            Term::Literal(Value::Bytes(raw.into()))
                        }
                        _ => Term::Param(b.fresh_param()),
                    };
                    b.predicates.push(Comparison {
                        op,
                        lhs: Term::Var(var),
                        rhs,
                    });
                } else {
                    let span = i64::try_from(sizes.transfers).expect("fits") * gen::AT_STEP * 2;
                    i64_dress(
                        b,
                        rng,
                        atom,
                        ids::transfer::AT,
                        gen::AT_BASE,
                        gen::AT_BASE + span,
                    );
                }
            }
            _ => {}
        }
    }
}

fn shape_of(rng: &mut Rng) -> Shape {
    let total: u64 = SHAPE_WEIGHTS.iter().map(|(_, w)| w).sum();
    let mut draw = rng.range(total);
    for (shape, weight) in SHAPE_WEIGHTS {
        if draw < *weight {
            return *shape;
        }
        draw -= weight;
    }
    unreachable!("weights cover the draw")
}

fn build(rng: &mut Rng, shape: Shape, cfg: GenConfig, sizes: &Sizes) -> Builder {
    let mut b = Builder::default();
    match shape {
        Shape::Guard => guard(&mut b, rng),
        Shape::Star => star(&mut b, rng),
        Shape::Chain => chain(&mut b, rng),
        Shape::SelfJoin => self_join(&mut b, rng),
        Shape::Gated => {
            match rng.range(5) {
                0 => guard(&mut b, rng),
                1 => star(&mut b, rng),
                2 => chain(&mut b, rng),
                3 => aggregate(&mut b, rng),
                _ => self_join(&mut b, rng),
            }
            // The zero-binding nonemptiness gate, over either non-empty
            // relation (falsity is the empty-store pass's job; diversity
            // here is about relation shape).
            b.atom(if rng.chance(1, 2) {
                ids::TAG
            } else {
                ids::TAG_NOTE
            });
        }
        Shape::Aggregate => aggregate(&mut b, rng),
    }
    dress(&mut b, rng, cfg, sizes);
    b
}

/// Generation facts the query alone cannot reveal (hit-vs-miss is a
/// corpus-content property).
#[derive(Debug, Clone, Copy, Default)]
struct GenTags {
    miss: bool,
    bytes_hit: bool,
    bytes_miss: bool,
}

fn random_query_tagged(rng: &mut Rng, cfg: GenConfig) -> (Query, Shape, GenTags) {
    let sizes = Sizes::of(cfg.scale);
    let shape = shape_of(rng);
    let b = build(rng, shape, cfg, &sizes);
    let tags = GenTags {
        miss: b.miss,
        bytes_hit: b.bytes_hit,
        bytes_miss: b.bytes_miss,
    };
    (b.into_query(), shape, tags)
}

/// The seeded extref of one Transfer row — corpus rows are a pure
/// function of the config, so in-vocabulary Bytes literals recompute.
fn extref_of(cfg: GenConfig, sizes: &Sizes, row: u64) -> Value {
    gen::row(&cfg, sizes, ids::TRANSFER, row)
        .into_iter()
        .nth(usize::from(ids::transfer::EXTREF.0))
        .expect("transfer rows carry extref")
}

/// One seeded random valid query over the ledger schema. The schema is
/// the ledger (the grammar is schema-specific by design); the config
/// bounds dressing literals (and recomputes in-vocabulary Bytes hits)
/// so predicates select real subsets.
#[must_use]
pub fn random_query(rng: &mut Rng, cfg: GenConfig) -> Query {
    random_query_tagged(rng, cfg).0
}

/// The comparison-type axis of the coverage matrix.
pub const CMP_TYPES: [&str; 6] = ["u64", "i64", "enum", "bool", "string", "bytes"];
/// The operator axis, in `CmpOp` order.
pub const CMP_OPS: [CmpOp; 6] = [
    CmpOp::Eq,
    CmpOp::Ne,
    CmpOp::Lt,
    CmpOp::Le,
    CmpOp::Gt,
    CmpOp::Ge,
];

/// Whether an (op, type) cell is legal under the roster: `Eq`/`Ne`
/// everywhere, order operators over the two integer types only.
#[must_use]
pub fn cmp_cell_legal(op_idx: usize, type_idx: usize) -> bool {
    op_idx < 2 || type_idx < 2
}

fn op_index(op: CmpOp) -> usize {
    CMP_OPS.iter().position(|o| *o == op).expect("all six ops")
}

fn type_index(ty: &bumbledb::schema::ValueType) -> usize {
    use bumbledb::schema::ValueType;
    match ty {
        ValueType::U64 => 0,
        ValueType::I64 => 1,
        ValueType::Enum { .. } => 2,
        ValueType::Bool => 3,
        ValueType::String => 4,
        ValueType::Bytes => 5,
    }
}

/// Construct counts over a generated batch — the coverage contract's
/// evidence. `matrix[op][type]` counts comparisons per (operator,
/// structural type): the asserted form of 50-validation's "every
/// comparison op on every legal type".
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Coverage {
    pub guard: u64,
    pub star: u64,
    pub chain: u64,
    pub self_join: u64,
    pub gated: u64,
    pub aggregate: u64,
    pub gates: u64,
    pub misses: u64,
    pub params: u64,
    pub repeated_vars: u64,
    pub agg_sum: u64,
    pub agg_min: u64,
    pub agg_max: u64,
    pub agg_count: u64,
    /// Aggregates whose input variable is u64-typed.
    pub agg_u64: u64,
    /// Aggregate-bearing find lists with more than one aggregate.
    pub multi_aggregate: u64,
    /// Var-vs-var comparisons whose variables bind in different atoms.
    pub cross_residuals: u64,
    /// In-vocabulary / out-of-vocabulary bytes literals.
    pub bytes_hits: u64,
    pub bytes_misses: u64,
    /// Comparison counts per `(CMP_OPS index, CMP_TYPES index)`.
    pub matrix: [[u64; 6]; 6],
}

impl Coverage {
    #[allow(clippy::too_many_lines)]
    fn record(&mut self, query: &Query, shape: Shape, tags: GenTags) {
        match shape {
            Shape::Guard => self.guard += 1,
            Shape::Star => self.star += 1,
            Shape::Chain => self.chain += 1,
            Shape::SelfJoin => self.self_join += 1,
            Shape::Gated => self.gated += 1,
            Shape::Aggregate => self.aggregate += 1,
        }
        self.gates += query
            .atoms
            .iter()
            .filter(|atom| atom.bindings.is_empty())
            .count() as u64;
        self.misses += u64::from(tags.miss);
        self.bytes_hits += u64::from(tags.bytes_hit);
        self.bytes_misses += u64::from(tags.bytes_miss);
        // Per-variable anchors: the (relation, field) that types each
        // var, and the atom set it binds in (cross-residual detection).
        let mut var_type = std::collections::HashMap::new();
        let mut var_atoms: std::collections::HashMap<VarId, Vec<usize>> =
            std::collections::HashMap::new();
        for (atom_idx, atom) in query.atoms.iter().enumerate() {
            let vars: Vec<&Term> = atom
                .bindings
                .iter()
                .filter(|(_, term)| matches!(term, Term::Var(_)))
                .map(|(_, term)| term)
                .collect();
            if vars
                .iter()
                .enumerate()
                .any(|(index, term)| vars[..index].contains(term))
            {
                self.repeated_vars += 1;
            }
            for (field, term) in &atom.bindings {
                if let Term::Var(var) = term {
                    var_type.entry(*var).or_insert_with(|| {
                        crate::schema::schema()
                            .relation(atom.relation)
                            .field(*field)
                            .value_type
                            .clone()
                    });
                    var_atoms.entry(*var).or_default().push(atom_idx);
                }
            }
        }
        for comparison in &query.predicates {
            let ty = match (&comparison.lhs, &comparison.rhs) {
                (Term::Var(var), _) | (_, Term::Var(var)) => var_type
                    .get(var)
                    .expect("comparison variables are atom-bound"),
                _ => unreachable!("the grammar never compares two constants"),
            };
            self.matrix[op_index(comparison.op)][type_index(ty)] += 1;
            if let (Term::Var(lhs), Term::Var(rhs)) = (&comparison.lhs, &comparison.rhs) {
                let shared = var_atoms[lhs].iter().any(|a| var_atoms[rhs].contains(a));
                if !shared {
                    self.cross_residuals += 1;
                }
            }
            for term in [&comparison.lhs, &comparison.rhs] {
                if matches!(term, Term::Param(_)) {
                    self.params += 1;
                }
            }
        }
        for atom in &query.atoms {
            for (_, term) in &atom.bindings {
                if matches!(term, Term::Param(_)) {
                    self.params += 1;
                }
            }
        }
        let mut aggregates = 0u64;
        for term in &query.finds {
            if let FindTerm::Aggregate { op, over } = term {
                aggregates += 1;
                match op {
                    AggOp::Sum => self.agg_sum += 1,
                    AggOp::Min => self.agg_min += 1,
                    AggOp::Max => self.agg_max += 1,
                    AggOp::Count => self.agg_count += 1,
                }
                if let Some(var) = over {
                    if matches!(var_type.get(var), Some(bumbledb::schema::ValueType::U64)) {
                        self.agg_u64 += 1;
                    }
                }
            }
        }
        self.multi_aggregate += u64::from(aggregates > 1);
    }
}

/// Generates `n` queries at the seed and counts every construct.
#[must_use]
pub fn coverage(n: u64, seed: u64, cfg: GenConfig) -> Coverage {
    let mut rng = Rng::new(seed);
    let mut cov = Coverage::default();
    for _ in 0..n {
        let (query, shape, tags) = random_query_tagged(&mut rng, cfg);
        cov.record(&query, shape, tags);
    }
    cov
}

/// Which set each of the four generated param vectors is.
const PARAM_SETS: usize = 4;

/// Resolves every param's anchor: the (relation, field) that types it —
/// directly for atom bindings, through the variable side for predicates.
fn param_anchors(query: &Query) -> Vec<(RelationId, FieldId)> {
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
fn u64_domain(rel: RelationId, field: FieldId, sizes: &Sizes) -> u64 {
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

/// Which of the four sets is being filled.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SetKind {
    Hit,
    Boundary,
    Miss,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gen::Scale;
    use crate::schema::schema;
    use crate::translate::translate;

    const SEED: u64 = 11;
    const N: u64 = 1000;

    const CFG: GenConfig = GenConfig {
        seed: 1,
        scale: Scale::S,
    };

    /// Every generated query passes the engine's validate (via prepare on
    /// an empty schema-loaded db) AND translates to SQL.
    #[test]
    fn a_thousand_queries_validate_and_translate() {
        let dir = std::env::temp_dir().join("bumbledb-bench-querygen");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let db = bumbledb::Db::create(&dir, schema()).expect("create");
        let mut rng = Rng::new(SEED);
        for i in 0..N {
            let query = random_query(&mut rng, CFG);
            if let Err(error) = db.prepare(&query) {
                panic!("query {i} fails validation: {error:?}\n{query:#?}");
            }
            if let Err(error) = translate(&query, schema()) {
                panic!("query {i} fails translation: {error}\n{query:#?}");
            }
        }
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Every construct appears at n = 1000, every *legal* cell of the
    /// per-(op, type) comparison matrix is nonzero, and shape counts sit
    /// within ±30% of their weight expectations (weight regressions
    /// surface). This is 50-validation's coverage contract, asserted.
    #[test]
    fn the_coverage_contract_holds_at_a_thousand() {
        let cov = coverage(N, SEED, CFG);
        let band = |count: u64, weight: u64| {
            let expected = N * weight / 90;
            assert!(
                count * 10 >= expected * 7 && count * 10 <= expected * 13,
                "count {count} outside ±30% of {expected}"
            );
        };
        band(cov.guard, 10);
        band(cov.star, 20);
        band(cov.chain, 20);
        band(cov.self_join, 10);
        band(cov.gated, 10);
        band(cov.aggregate, 20);
        for (name, count) in [
            ("gates", cov.gates),
            ("misses", cov.misses),
            ("params", cov.params),
            ("repeated_vars", cov.repeated_vars),
            ("agg_sum", cov.agg_sum),
            ("agg_min", cov.agg_min),
            ("agg_max", cov.agg_max),
            ("agg_count", cov.agg_count),
            ("agg_u64", cov.agg_u64),
            ("multi_aggregate", cov.multi_aggregate),
            ("cross_residuals", cov.cross_residuals),
            ("bytes_hits", cov.bytes_hits),
            ("bytes_misses", cov.bytes_misses),
        ] {
            assert!(count > 0, "{name} never generated");
        }
        for (op_idx, op) in CMP_OPS.iter().enumerate() {
            for (type_idx, ty) in CMP_TYPES.iter().enumerate() {
                let count = cov.matrix[op_idx][type_idx];
                if cmp_cell_legal(op_idx, type_idx) {
                    assert!(count > 0, "({op:?}, {ty}) never generated");
                } else {
                    assert_eq!(count, 0, "({op:?}, {ty}) is illegal by the roster");
                }
            }
        }
    }

    /// PRD 07 (docs/hardening): the grammar never emits a NUL — the
    /// translator rejects NUL string literals by name, and this property
    /// keeps that boundary unreachable from generated queries.
    #[test]
    fn generated_string_literals_are_nul_free() {
        let mut rng = Rng::new(SEED);
        for _ in 0..N {
            let query = random_query(&mut rng, CFG);
            for atom in &query.atoms {
                for (_, term) in &atom.bindings {
                    if let bumbledb::Term::Literal(bumbledb::Value::String(raw)) = term {
                        assert!(!raw.contains(&0), "a generated literal carries NUL");
                    }
                }
            }
            for comparison in &query.predicates {
                for term in [&comparison.lhs, &comparison.rhs] {
                    if let bumbledb::Term::Literal(bumbledb::Value::String(raw)) = term {
                        assert!(!raw.contains(&0), "a generated literal carries NUL");
                    }
                }
            }
        }
    }

    /// Same seed ⇒ identical query stream (pinned on #500's rendering).
    #[test]
    fn generation_is_deterministic() {
        let query_500 = |seed| {
            let mut rng = Rng::new(seed);
            let mut last = None;
            for _ in 0..=500 {
                last = Some(random_query(&mut rng, CFG));
            }
            format!("{:?}", last.expect("generated"))
        };
        assert_eq!(query_500(SEED), query_500(SEED));
        assert_ne!(query_500(SEED), query_500(SEED + 1));
    }

    /// Four sets, with every string, bytes, and u64 param a guaranteed
    /// miss in the last (out of vocabulary or out of domain).
    #[test]
    fn params_for_produces_the_documented_sets() {
        let mut rng = Rng::new(SEED);
        let sizes = Sizes::of(CFG.scale);
        let (mut saw_string, mut saw_u64, mut saw_bytes) = (false, false, false);
        for _ in 0..200 {
            let query = random_query(&mut rng, CFG);
            let sets = params_for(&query, &mut rng, CFG);
            assert_eq!(sets.len(), 4);
            let anchors = param_anchors(&query);
            for set in &sets {
                assert_eq!(set.len(), anchors.len());
            }
            for (value, anchor) in sets[3].iter().zip(&anchors) {
                match value {
                    Value::String(raw) => {
                        saw_string = true;
                        assert!(
                            raw.starts_with(b"missing-"),
                            "set 3 string params are guaranteed misses"
                        );
                    }
                    Value::U64(v) => {
                        saw_u64 = true;
                        let domain = u64_domain(anchor.0, anchor.1, &sizes);
                        assert!(*v > domain, "set 3 u64 params are out of domain");
                    }
                    Value::Bytes(raw) => {
                        saw_bytes = true;
                        assert_eq!(raw.len(), 16, "a fresh 16-byte miss value");
                    }
                    _ => {}
                }
            }
        }
        assert!(saw_string, "the batch produced string params");
        assert!(saw_u64, "the batch produced u64 params");
        assert!(saw_bytes, "the batch produced bytes params");
    }
}
