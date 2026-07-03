//! The randomized query generator (docs/benchmarks/11): seeded random
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
    /// Whether dressing emitted an out-of-vocabulary string literal.
    miss: bool,
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

/// Two Posting occurrences equated on `transfer`, projecting both amounts.
fn self_join(b: &mut Builder, rng: &mut Rng) {
    let first = b.atom(ids::POSTING);
    let transfer = b.bind_var(first, ids::posting::TRANSFER);
    let x = b.bind_var(first, ids::posting::AMOUNT);
    let second = b.atom(ids::POSTING);
    b.bind(second, ids::posting::TRANSFER, Term::Var(transfer));
    let y = b.bind_var(second, ids::posting::AMOUNT);
    b.find_var(x);
    b.find_var(y);
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

/// Any join shape re-projected as group-by + one aggregate; group key =
/// 0–2 of the shape's bound variables.
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
    let (op, over) = match rng.range(4) {
        0 => (AggOp::Sum, Some(amount)),
        1 => (AggOp::Count, None),
        2 => (AggOp::Min, Some(at)),
        _ => (AggOp::Max, Some(amount)),
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
    for var in key {
        b.find_var(var);
    }
    b.finds.push(FindTerm::Aggregate { op, over });
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

/// An i64 range predicate on the field: literal or param, 50/50.
fn i64_range(b: &mut Builder, rng: &mut Rng, atom: usize, field: FieldId, lo: i64, hi: i64) {
    let Some(var) = b.var_at(atom, field) else {
        return;
    };
    let op = order_op(rng);
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

/// An `Eq` predicate against an enum-ordinal literal.
fn enum_eq(b: &mut Builder, rng: &mut Rng, atom: usize, field: FieldId, variants: u64) {
    let Some(var) = b.var_at(atom, field) else {
        return;
    };
    let ordinal = u8::try_from(rng.range(variants)).expect("small");
    b.predicates.push(Comparison {
        op: CmpOp::Eq,
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
    match rng.range(5) {
        0 => i64_range(b, rng, atom, ids::posting::AMOUNT, -5_000_000, 5_000_000),
        1 => {
            let (lo, hi) = posting_at_window(sizes);
            i64_range(b, rng, atom, ids::posting::AT, lo, hi);
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
            b.predicates.push(Comparison {
                op: CmpOp::Eq,
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
fn dress(b: &mut Builder, rng: &mut Rng, sizes: &Sizes) {
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
                    enum_eq(b, rng, atom, ids::account::STATUS, 3);
                } else {
                    i64_range(
                        b,
                        rng,
                        atom,
                        ids::account::OPENED_AT,
                        gen::AT_BASE - (1 << 30),
                        gen::AT_BASE,
                    );
                }
            }
            ids::INSTRUMENT => enum_eq(b, rng, atom, ids::instrument::KIND, 4),
            ids::HOLDER => enum_eq(b, rng, atom, ids::holder::REGION, 4),
            ids::TRANSFER => {
                let span = i64::try_from(sizes.transfers).expect("fits") * gen::AT_STEP * 2;
                i64_range(
                    b,
                    rng,
                    atom,
                    ids::transfer::AT,
                    gen::AT_BASE,
                    gen::AT_BASE + span,
                );
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

fn build(rng: &mut Rng, shape: Shape, sizes: &Sizes) -> Builder {
    let mut b = Builder::default();
    match shape {
        Shape::Guard => guard(&mut b, rng),
        Shape::Star => star(&mut b, rng),
        Shape::Chain => chain(&mut b, rng),
        Shape::SelfJoin => self_join(&mut b, rng),
        Shape::Gated => {
            match rng.range(4) {
                0 => guard(&mut b, rng),
                1 => star(&mut b, rng),
                2 => chain(&mut b, rng),
                _ => self_join(&mut b, rng),
            }
            // The zero-binding nonemptiness gate.
            b.atom(ids::TAG);
        }
        Shape::Aggregate => aggregate(&mut b, rng),
    }
    dress(&mut b, rng, sizes);
    b
}

fn random_query_tagged(rng: &mut Rng, sizes: &Sizes) -> (Query, Shape, bool) {
    let shape = shape_of(rng);
    let b = build(rng, shape, sizes);
    let miss = b.miss;
    (b.into_query(), shape, miss)
}

/// One seeded random valid query over the ledger schema. The schema is
/// the ledger (the grammar is schema-specific by design); `Sizes` bounds
/// the dressing literals so they select real subsets.
#[must_use]
pub fn random_query(rng: &mut Rng, sizes: &Sizes) -> Query {
    random_query_tagged(rng, sizes).0
}

/// Construct counts over a generated batch — the coverage contract's
/// evidence.
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
    pub cmp_eq: u64,
    pub cmp_ne: u64,
    pub cmp_lt: u64,
    pub cmp_le: u64,
    pub cmp_gt: u64,
    pub cmp_ge: u64,
}

impl Coverage {
    fn record(&mut self, query: &Query, shape: Shape, miss: bool) {
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
        self.misses += u64::from(miss);
        for atom in &query.atoms {
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
        }
        for comparison in &query.predicates {
            match comparison.op {
                CmpOp::Eq => self.cmp_eq += 1,
                CmpOp::Ne => self.cmp_ne += 1,
                CmpOp::Lt => self.cmp_lt += 1,
                CmpOp::Le => self.cmp_le += 1,
                CmpOp::Gt => self.cmp_gt += 1,
                CmpOp::Ge => self.cmp_ge += 1,
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
        for term in &query.finds {
            if let FindTerm::Aggregate { op, .. } = term {
                match op {
                    AggOp::Sum => self.agg_sum += 1,
                    AggOp::Min => self.agg_min += 1,
                    AggOp::Max => self.agg_max += 1,
                    AggOp::Count => self.agg_count += 1,
                }
            }
        }
    }
}

/// Generates `n` queries at the seed and counts every construct.
#[must_use]
pub fn coverage(n: u64, seed: u64, sizes: &Sizes) -> Coverage {
    let mut rng = Rng::new(seed);
    let mut cov = Coverage::default();
    for _ in 0..n {
        let (query, shape, miss) = random_query_tagged(&mut rng, sizes);
        cov.record(&query, shape, miss);
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
            let domain = u64_domain(rel, field, sizes);
            Value::U64(match kind {
                SetKind::Hit | SetKind::Miss => rng.range(domain),
                SetKind::Boundary => 0,
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
                SetKind::Boundary => lo,
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
        ValueType::Enum { variants } => Value::Enum(match kind {
            SetKind::Hit | SetKind::Miss => {
                u8::try_from(rng.range(variants.len() as u64)).expect("small")
            }
            SetKind::Boundary => 0,
        }),
        ValueType::Bool => Value::Bool(match kind {
            SetKind::Hit | SetKind::Miss => rng.chance(1, 2),
            SetKind::Boundary => false,
        }),
        ValueType::Bytes => {
            let mut raw = Vec::with_capacity(16);
            for _ in 0..2 {
                raw.extend_from_slice(&rng.u64().to_le_bytes());
            }
            Value::Bytes(raw.into())
        }
    }
}

/// Four param sets per query: two in-range hits, one of boundary values
/// (domain minima), and one where every string param is a guaranteed
/// miss (non-string params stay in range).
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
                .map(|anchor| param_value(*anchor, kind, rng, &sizes))
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

    fn sizes() -> Sizes {
        Sizes::of(Scale::S)
    }

    /// Every generated query passes the engine's validate (via prepare on
    /// an empty schema-loaded db) AND translates to SQL.
    #[test]
    fn a_thousand_queries_validate_and_translate() {
        let dir = std::env::temp_dir().join("bumbledb-bench-querygen");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let db = bumbledb::Db::create(&dir, schema()).expect("create");
        let mut rng = Rng::new(SEED);
        let s = sizes();
        for i in 0..N {
            let query = random_query(&mut rng, &s);
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

    /// Every construct appears at n = 1000, and shape counts sit within
    /// ±30% of their weight expectations (weight regressions surface).
    #[test]
    fn the_coverage_contract_holds_at_a_thousand() {
        let cov = coverage(N, SEED, &sizes());
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
            ("cmp_eq", cov.cmp_eq),
            ("cmp_ne", cov.cmp_ne),
            ("cmp_lt", cov.cmp_lt),
            ("cmp_le", cov.cmp_le),
            ("cmp_gt", cov.cmp_gt),
            ("cmp_ge", cov.cmp_ge),
        ] {
            assert!(count > 0, "{name} never generated");
        }
    }

    /// Same seed ⇒ identical query stream (pinned on #500's rendering).
    #[test]
    fn generation_is_deterministic() {
        let query_500 = |seed| {
            let mut rng = Rng::new(seed);
            let s = sizes();
            let mut last = None;
            for _ in 0..=500 {
                last = Some(random_query(&mut rng, &s));
            }
            format!("{:?}", last.expect("generated"))
        };
        assert_eq!(query_500(SEED), query_500(SEED));
        assert_ne!(query_500(SEED), query_500(SEED + 1));
    }

    /// Four sets, with every string param a guaranteed miss in the last.
    #[test]
    fn params_for_produces_the_documented_sets() {
        let cfg = GenConfig {
            seed: SEED,
            scale: Scale::S,
        };
        let mut rng = Rng::new(SEED);
        let s = sizes();
        let mut saw_string_param = false;
        for _ in 0..200 {
            let query = random_query(&mut rng, &s);
            let sets = params_for(&query, &mut rng, cfg);
            assert_eq!(sets.len(), 4);
            let anchors = param_anchors(&query);
            for set in &sets {
                assert_eq!(set.len(), anchors.len());
            }
            for value in &sets[3] {
                if let Value::String(raw) = value {
                    saw_string_param = true;
                    assert!(
                        raw.starts_with(b"missing-"),
                        "set 3 string params are guaranteed misses"
                    );
                }
            }
        }
        assert!(saw_string_param, "the batch produced string params");
    }
}
