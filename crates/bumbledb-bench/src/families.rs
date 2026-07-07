//! The ten gated read families (docs/architecture/50-validation.md): exact IR, exact
//! param policy, hand-written SQL golden, gate classification. This file
//! of queries **is** the benchmark's identity — `digest()` keys the
//! verify stamp and every report on it.

use bumbledb::{AggOp, Atom, CmpOp, Comparison, FindTerm, ParamId, Query, Term, Value, VarId};

use crate::gen::{self, GenConfig, Rng, Sizes};
use crate::schema::ids;
use crate::translate::goldens;

/// Whether a family gates the suite (loses ⇒ the run fails) or merely
/// reports. All ten read families gate (`00-product.md`: every family
/// must win).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Gate,
    Report,
}

/// One read family: the benchmark's unit of identity.
pub struct Family {
    pub name: &'static str,
    pub kind: Kind,
    pub query: fn() -> Query,
    /// The seeded param sets — verify and bench call this with the same
    /// `GenConfig` and therefore see identical sets.
    pub params: fn(&GenConfig) -> Vec<Vec<Value>>,
    /// Hand-written (docs/architecture/50-validation.md) — never regenerated from the
    /// translator; pinned equal to `translate` output by test.
    pub golden_sql: &'static str,
    /// The documented param policy, rendered into the versioned query
    /// list.
    pub param_policy: &'static str,
}

fn var(id: u16) -> Term {
    Term::Var(VarId(id))
}

fn param(id: u16) -> Term {
    Term::Param(ParamId(id))
}

/// point — `Q(amount, at) :- Posting(id = ?0, amount, at)`. Guard probe.
fn point_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![
                (ids::posting::ID, param(0)),
                (ids::posting::AMOUNT, var(0)),
                (ids::posting::AT, var(1)),
            ],
        }],
        predicates: vec![],
    }
}

fn point_params(cfg: &GenConfig) -> Vec<Vec<Value>> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0114_0001);
    let mut sets: Vec<Vec<Value>> = (0..3)
        .map(|_| vec![Value::U64(rng.range(sizes.postings))])
        .collect();
    sets.push(vec![Value::U64(sizes.postings + 1_000_000)]);
    sets
}

/// `fk_walk` — `Q(name, amount) :- Posting(account = ?0, amount),
/// Account(id = ?0, holder = h), Holder(id = h, name)`.
fn fk_walk_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::ACCOUNT, param(0)),
                    (ids::posting::AMOUNT, var(1)),
                ],
            },
            Atom {
                relation: ids::ACCOUNT,
                bindings: vec![(ids::account::ID, param(0)), (ids::account::HOLDER, var(2))],
            },
            Atom {
                relation: ids::HOLDER,
                bindings: vec![(ids::holder::ID, var(2)), (ids::holder::NAME, var(0))],
            },
        ],
        predicates: vec![],
    }
}

fn fk_walk_params(cfg: &GenConfig) -> Vec<Vec<Value>> {
    let sizes = Sizes::of(cfg.scale);
    let hot = sizes.hot_accounts();
    let mut rng = Rng::new(cfg.seed ^ 0x0114_0002);
    vec![
        vec![Value::U64(hot + rng.range(sizes.accounts - hot))],
        vec![Value::U64(hot + rng.range(sizes.accounts - hot))],
        vec![Value::U64(rng.range(hot))],
        vec![Value::U64(sizes.accounts + 1_000_000)],
    ]
}

/// chain — `Q(region, amount, at) :- Posting(account = a, amount, at),
/// Account(id = a, holder = h, status = Open), Holder(id = h, region)`
/// with `at >= ?0`.
fn chain_query() -> Query {
    Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(1)),
            FindTerm::Var(VarId(2)),
        ],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::ACCOUNT, var(3)),
                    (ids::posting::AMOUNT, var(1)),
                    (ids::posting::AT, var(2)),
                ],
            },
            Atom {
                relation: ids::ACCOUNT,
                bindings: vec![
                    (ids::account::ID, var(3)),
                    (ids::account::HOLDER, var(4)),
                    (ids::account::STATUS, Term::Literal(Value::Enum(0))),
                ],
            },
            Atom {
                relation: ids::HOLDER,
                bindings: vec![(ids::holder::ID, var(4)), (ids::holder::REGION, var(0))],
            },
        ],
        predicates: vec![Comparison {
            op: CmpOp::Ge,
            lhs: var(2),
            rhs: param(0),
        }],
    }
}

fn chain_params(cfg: &GenConfig) -> Vec<Vec<Value>> {
    let sizes = Sizes::of(cfg.scale);
    let span = i64::try_from(sizes.postings).expect("fits") * gen::AT_STEP;
    // Four suffix edges near the corpus end, selecting ≈2/4/6/8%.
    (1..=4)
        .map(|k| vec![Value::I64(gen::AT_BASE + span - span * k / 50)])
        .collect()
}

/// range — `Q(id, amount) :- Posting(id, amount, at)`, `at >= ?0`,
/// `at < ?1` — the pure scan family.
fn range_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![
                (ids::posting::ID, var(0)),
                (ids::posting::AMOUNT, var(1)),
                (ids::posting::AT, var(2)),
            ],
        }],
        predicates: vec![
            Comparison {
                op: CmpOp::Ge,
                lhs: var(2),
                rhs: param(0),
            },
            Comparison {
                op: CmpOp::Lt,
                lhs: var(2),
                rhs: param(1),
            },
        ],
    }
}

fn range_params(cfg: &GenConfig) -> Vec<Vec<Value>> {
    let sizes = Sizes::of(cfg.scale);
    let span = i64::try_from(sizes.postings).expect("fits") * gen::AT_STEP;
    let width = span / 50;
    // Four ≈2%-selectivity windows spread over the timestamp span.
    (0..4)
        .map(|k| {
            let start = gen::AT_BASE + span * (2 * k + 1) / 16;
            vec![Value::I64(start), Value::I64(start + width)]
        })
        .collect()
}

/// balance — `Q(a, Sum(amount)) :- Posting(id, account = a, amount),
/// Account(id = a, holder = ?0)`. The serial id binding makes every
/// posting a distinct binding, so the fold is the *ledger balance* —
/// duplicate amounts on one account count once each, not once total —
/// and the distinct-bindings elision engages (unique coverage), putting
/// the seen-set-elided aggregate path under the oracle.
fn balance_query() -> Query {
    Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::ID, var(2)),
                    (ids::posting::ACCOUNT, var(0)),
                    (ids::posting::AMOUNT, var(1)),
                ],
            },
            Atom {
                relation: ids::ACCOUNT,
                bindings: vec![(ids::account::ID, var(0)), (ids::account::HOLDER, param(0))],
            },
        ],
        predicates: vec![],
    }
}

fn balance_params(cfg: &GenConfig) -> Vec<Vec<Value>> {
    let sizes = Sizes::of(cfg.scale);
    // The hot-owning holder: whoever holds hot account 0 (deterministic —
    // the generator is a pure function of (cfg, relation, row)).
    let account0 = gen::row(cfg, &sizes, ids::ACCOUNT, 0);
    let hot_holder = account0[usize::from(ids::account::HOLDER.0)].clone();
    let mut rng = Rng::new(cfg.seed ^ 0x0114_0005);
    let mut sets = vec![vec![hot_holder]];
    sets.extend((0..3).map(|_| vec![Value::U64(rng.range(sizes.holders))]));
    sets
}

/// stats — `Q(k, Min(at), Max(amount), Count) :- Posting(instrument = i,
/// amount, at), Instrument(id = i, kind = k)`.
fn stats_query() -> Query {
    Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Min,
                over: Some(VarId(2)),
            },
            FindTerm::Aggregate {
                op: AggOp::Max,
                over: Some(VarId(1)),
            },
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
        ],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::INSTRUMENT, var(3)),
                    (ids::posting::AMOUNT, var(1)),
                    (ids::posting::AT, var(2)),
                ],
            },
            Atom {
                relation: ids::INSTRUMENT,
                bindings: vec![
                    (ids::instrument::ID, var(3)),
                    (ids::instrument::KIND, var(0)),
                ],
            },
        ],
        predicates: vec![],
    }
}

fn stats_params(_: &GenConfig) -> Vec<Vec<Value>> {
    // Literal-free full fold: one empty param set.
    vec![vec![]]
}

/// string — `Q(id, amount) :- Posting(id, amount, memo = ?0)`.
fn string_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: ids::POSTING,
            bindings: vec![
                (ids::posting::ID, var(0)),
                (ids::posting::AMOUNT, var(1)),
                (ids::posting::MEMO, param(0)),
            ],
        }],
        predicates: vec![],
    }
}

fn string_params(cfg: &GenConfig) -> Vec<Vec<Value>> {
    let mut rng = Rng::new(cfg.seed ^ 0x0114_0007);
    let mut sets: Vec<Vec<Value>> = (0..3)
        .map(|_| {
            vec![Value::String(
                format!("m{}", rng.range(gen::MEMO_VOCAB))
                    .into_bytes()
                    .into(),
            )]
        })
        .collect();
    // The never-interned miss: no corpus vocabulary starts with this.
    sets.push(vec![Value::String(b"missing-family".to_vec().into())]);
    sets
}

/// The two tag ids the generator guarantees on hot accounts: tag 0 (every
/// hot account's `k = 0` pair) and tag 97 (hot account 0's `k = 1` pair,
/// `(0 + 97) % 256`).
pub const SKEW_HOT_TAGS: [u64; 2] = [0, 97];

/// skew — `Q(label, amount) :- Posting(account = a, amount),
/// AccountTag(account = a, tag = t), Tag(id = t, label)` with
/// `label = ?0`.
fn skew_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::ACCOUNT, var(2)),
                    (ids::posting::AMOUNT, var(1)),
                ],
            },
            Atom {
                relation: ids::ACCOUNT_TAG,
                bindings: vec![
                    (ids::account_tag::ACCOUNT, var(2)),
                    (ids::account_tag::TAG, var(3)),
                ],
            },
            Atom {
                relation: ids::TAG,
                bindings: vec![(ids::tag::ID, var(3)), (ids::tag::LABEL, var(0))],
            },
        ],
        predicates: vec![Comparison {
            op: CmpOp::Eq,
            lhs: var(0),
            rhs: param(0),
        }],
    }
}

fn skew_params(cfg: &GenConfig) -> Vec<Vec<Value>> {
    let label = |tag: u64| vec![Value::String(format!("tag-{tag:03}").into_bytes().into())];
    let mut rng = Rng::new(cfg.seed ^ 0x0114_0008);
    // Two hot-attached tags, two uniform tags (drawn above the hot k = 1
    // band, which tops out well below 150).
    vec![
        label(SKEW_HOT_TAGS[0]),
        label(SKEW_HOT_TAGS[1]),
        label(150 + rng.range(100)),
        label(150 + rng.range(100)),
    ]
}

/// spread — `Q(x, y) :- Posting(transfer = t, amount = x),
/// Posting(transfer = t, amount = y)` with `x < y` — the cross-atom
/// residual family: a self-join whose ordered comparison exercises
/// `PlacedComparison` placement, per-node residual evaluation, and
/// survivor compaction (no other family or generated shape did).
fn spread_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::TRANSFER, var(2)),
                    (ids::posting::AMOUNT, var(0)),
                ],
            },
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::TRANSFER, var(2)),
                    (ids::posting::AMOUNT, var(1)),
                ],
            },
        ],
        predicates: vec![Comparison {
            op: CmpOp::Lt,
            lhs: var(0),
            rhs: var(1),
        }],
    }
}

fn spread_params(_: &GenConfig) -> Vec<Vec<Value>> {
    // Param-less full-relation family (like stats): ~1 ordered pair per
    // transfer at any scale (2 postings per transfer), so the result is
    // ~transfers rows — measured acceptable at S.
    vec![vec![]]
}

/// triangle — `Q(a) :- Posting(account = a, instrument = i),
/// Posting(instrument = i, transfer = w), Posting(transfer = w,
/// account = a)` with `?0 <= a < ?1` — a true 3-cycle over the ledger via
/// self-joins on Posting's three FK fields: three occurrences, three
/// shared variables, a cyclic hypergraph — exactly the dynamic-cover
/// stress the paper's triangle exposes.
fn triangle_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::ACCOUNT, var(0)),
                    (ids::posting::INSTRUMENT, var(1)),
                ],
            },
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::TRANSFER, var(2)),
                    (ids::posting::INSTRUMENT, var(1)),
                ],
            },
            Atom {
                relation: ids::POSTING,
                bindings: vec![
                    (ids::posting::TRANSFER, var(2)),
                    (ids::posting::ACCOUNT, var(0)),
                ],
            },
        ],
        predicates: vec![
            Comparison {
                op: CmpOp::Ge,
                lhs: var(0),
                rhs: param(0),
            },
            Comparison {
                op: CmpOp::Lt,
                lhs: var(0),
                rhs: param(1),
            },
        ],
    }
}

fn triangle_params(cfg: &GenConfig) -> Vec<Vec<Value>> {
    // The unnarrowed cycle is O(postings x instrument-fanout) work —
    // beyond the 10 ms budget class at S (measured) — so the account
    // window `?0 <= a < ?1` keeps the family inside it. Windows are
    // *cold* (they start past the hot set — any window containing hot
    // account 0 is dominated by its posting share): three ~1%-of-
    // accounts slices spread over the cold range, plus the empty window.
    let sizes = Sizes::of(cfg.scale);
    let hot = sizes.hot_accounts();
    let width = (sizes.accounts / 100).max(1);
    let cold = sizes.accounts - hot;
    let window = |k: u64| {
        let lo = hot + cold * k / 3;
        vec![Value::U64(lo), Value::U64(lo + width)]
    };
    let mut sets: Vec<Vec<Value>> = (0..3).map(window).collect();
    sets.push(vec![Value::U64(sizes.accounts), Value::U64(sizes.accounts)]);
    sets
}

/// The registry: the ten, in the suite's canonical order.
#[must_use]
pub fn all() -> &'static [Family] {
    &[
        Family {
            name: "point",
            kind: Kind::Gate,
            query: point_query,
            params: point_params,
            golden_sql: goldens::POINT,
            param_policy: "3 existing posting ids + 1 miss (id = postings + 10^6).",
        },
        Family {
            name: "fk_walk",
            kind: Kind::Gate,
            query: fk_walk_query,
            params: fk_walk_params,
            golden_sql: goldens::FK_WALK,
            param_policy: "2 cold accounts, 1 hot account, 1 miss (id = accounts + 10^6).",
        },
        Family {
            name: "chain",
            kind: Kind::Gate,
            query: chain_query,
            params: chain_params,
            golden_sql: goldens::CHAIN,
            param_policy: "4 suffix edges near the corpus end (at >= edge selects ~2/4/6/8%).",
        },
        Family {
            name: "range",
            kind: Kind::Gate,
            query: range_query,
            params: range_params,
            golden_sql: goldens::RANGE,
            param_policy: "4 windows of the pinned ~2% selectivity, spread over the span.",
        },
        Family {
            name: "balance",
            kind: Kind::Gate,
            query: balance_query,
            params: balance_params,
            golden_sql: goldens::BALANCE,
            param_policy: "4 holders, the first owning hot account 0.",
        },
        Family {
            name: "stats",
            kind: Kind::Gate,
            query: stats_query,
            params: stats_params,
            golden_sql: goldens::STATS,
            param_policy: "No params — literal-free full fold; one empty set.",
        },
        Family {
            name: "string",
            kind: Kind::Gate,
            query: string_query,
            params: string_params,
            golden_sql: goldens::STRING,
            param_policy: "3 vocabulary memos + 1 never-interned miss.",
        },
        Family {
            name: "skew",
            kind: Kind::Gate,
            query: skew_query,
            params: skew_params,
            golden_sql: goldens::SKEW,
            param_policy: "2 hot-attached tag labels (tags 0 and 97) + 2 uniform tag labels.",
        },
        Family {
            name: "spread",
            kind: Kind::Gate,
            query: spread_query,
            params: spread_params,
            golden_sql: goldens::SPREAD,
            param_policy: "No params — full-relation cross-atom residual; one empty set.",
        },
        Family {
            name: "triangle",
            kind: Kind::Gate,
            query: triangle_query,
            params: triangle_params,
            golden_sql: goldens::TRIANGLE,
            param_policy: "3 cold ~1%-of-accounts windows (?0 <= a < ?1, past the hot set) + the empty window.",
        },
    ]
}

/// One write/cold family (docs/architecture/50-validation.md): a name, its report-only
/// classification, and its write-appropriate protocol. The runners live
/// in `writebench` — these are identities, not closures.
pub struct WriteFamily {
    pub name: &'static str,
    pub kind: Kind,
    pub protocol: crate::harness::Protocol,
}

/// The write and cold families — all `Kind::Report` (the suite ruling:
/// "every family must win" is the read set; writes and cold are
/// described honestly, never gated).
#[must_use]
pub fn write_families() -> &'static [WriteFamily] {
    use crate::harness::Protocol;
    &[
        WriteFamily {
            name: "commit_single",
            kind: Kind::Report,
            protocol: Protocol {
                warmups: 8,
                samples: 64,
            },
        },
        WriteFamily {
            name: "commit_batch",
            kind: Kind::Report,
            protocol: Protocol {
                warmups: 4,
                samples: 32,
            },
        },
        WriteFamily {
            name: "bulk",
            kind: Kind::Report,
            protocol: Protocol {
                warmups: 1,
                samples: 8,
            },
        },
        WriteFamily {
            name: "cold_fk_walk",
            kind: Kind::Report,
            protocol: Protocol::COLD,
        },
    ]
}

fn digest_over<'a>(items: impl Iterator<Item = (&'a str, String, &'a str)>) -> [u8; 32] {
    let mut digest = bumbledb::digest::Digest::new();
    for (name, query_debug, golden_sql) in items {
        digest.update(name.as_bytes());
        digest.update(query_debug.as_bytes());
        digest.update(golden_sql.as_bytes());
    }
    digest.finalize()
}

/// The family-list digest: blake3 over every family's name, query IR
/// (Debug), and golden SQL — a verify-stamp ingredient. Any change to any
/// family re-baselines every stamp and report.
#[must_use]
pub fn digest() -> [u8; 32] {
    digest_over(
        all()
            .iter()
            .map(|f| (f.name, format!("{:?}", (f.query)()), f.golden_sql)),
    )
}

/// The human-readable versioned query list: IR + SQL + param policy per
/// family (PRD 18 emits this into the repo as QUERIES.md).
#[must_use]
pub fn render_queries_md() -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    out.push_str("# The read query families\n\n");
    let _ = writeln!(
        out,
        "Family-list digest: `{}`.\n",
        gen::digest_hex(&digest())
    );
    for family in all() {
        let _ = writeln!(out, "## {}\n", family.name);
        let kind = match family.kind {
            Kind::Gate => "gate",
            Kind::Report => "report",
        };
        let _ = writeln!(out, "Kind: {kind}.\n");
        let _ = writeln!(out, "```text\n{:#?}\n```\n", (family.query)());
        let _ = writeln!(out, "```sql\n{}\n```\n", family.golden_sql);
        let _ = writeln!(out, "Params: {}\n", family.param_policy);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gen::Scale;
    use crate::schema::schema;
    use crate::translate::translate;

    const CFG: GenConfig = GenConfig {
        seed: 1,
        scale: Scale::S,
    };

    #[test]
    fn all_ten_validate_and_prepare() {
        let dir = std::env::temp_dir().join("bumbledb-bench-families");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let db = bumbledb::Db::create(&dir, schema()).expect("create");
        assert_eq!(all().len(), 10);
        for family in all() {
            db.prepare(&(family.query)())
                .unwrap_or_else(|e| panic!("{} fails validation: {e:?}", family.name));
        }
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// PRD 09 (docs/perf/): the skip-free roster, pinned from real plans
    /// (the classification test the PRD orders written FIRST — its output
    /// decides which families gate PRD 09 vs PRD 10). The result moved
    /// the suite's plan: every skip-free family is a ≤2-node plan whose
    /// leaf already runs fused (cross-node batching has no parents to
    /// batch), while the deep-node families — triangle, chain, skew,
    /// `fk_walk` — all carry D2-crossing nodes and gate PRD 10.
    #[test]
    fn skip_free_classification_is_pinned() {
        let dir = std::env::temp_dir().join("bumbledb-bench-families-skipfree");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let db = bumbledb::Db::create(&dir, schema()).expect("create");
        let mut seen: Vec<(&str, Option<bool>)> = Vec::new();
        for family in all() {
            let prepared = db.prepare(&(family.query)()).expect("prepares");
            seen.push((family.name, prepared.skip_free()));
        }
        assert_eq!(
            seen,
            vec![
                ("point", None),
                ("fk_walk", Some(false)),
                ("chain", Some(false)),
                ("range", Some(true)),
                ("balance", Some(true)),
                ("stats", Some(true)),
                ("string", Some(true)),
                ("skew", Some(false)),
                ("spread", Some(true)),
                ("triangle", Some(false)),
            ],
            "the skip-free roster and with it the PRD 09/10 gate split"
        );
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// PRD 02 (docs/perf/): the aggregate families' fold regimes, pinned.
    /// balance binds the posting serial — distinct bindings proven, the
    /// seen-set elided, the constant-group fast path bare. stats binds
    /// no unique coverage **by design** (collapsing duplicate
    /// (kind, amount, at, instrument) bindings is the family's set
    /// semantics), so its dedup pass is semantically required and the
    /// batch fold runs the dedup-then-gather arm. A planner change that
    /// flips either regime is a semantics bug, not a tuning change.
    #[test]
    fn aggregate_family_fold_regimes_are_pinned() {
        let dir = std::env::temp_dir().join("bumbledb-bench-families-elide");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let db = bumbledb::Db::create(&dir, schema()).expect("create");
        let mut seen: Vec<(&str, bool)> = Vec::new();
        for family in all() {
            let query = (family.query)();
            if !query
                .finds
                .iter()
                .any(|f| matches!(f, bumbledb::FindTerm::Aggregate { .. }))
            {
                continue;
            }
            let prepared = db.prepare(&query).expect("prepares");
            seen.push((family.name, prepared.distinct_bindings()));
        }
        assert_eq!(
            seen,
            vec![("balance", true), ("stats", false)],
            "the aggregate roster and its fold regimes"
        );
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_golden_equals_its_translation() {
        for family in all() {
            let t = translate(&(family.query)(), schema())
                .unwrap_or_else(|e| panic!("{} fails translation: {e}", family.name));
            assert_eq!(t.sql, family.golden_sql, "family {}", family.name);
        }
    }

    #[test]
    fn params_are_deterministic_with_the_documented_misses() {
        let sizes = Sizes::of(CFG.scale);
        for family in all() {
            let a = (family.params)(&CFG);
            let b = (family.params)(&CFG);
            assert_eq!(a, b, "{} params must be seeded", family.name);
            let expected_sets = if matches!(family.name, "stats" | "spread") {
                1
            } else {
                4
            };
            assert_eq!(a.len(), expected_sets, "{}", family.name);
        }
        // The documented misses.
        let point = (all()[0].params)(&CFG);
        let Value::U64(miss) = point[3][0] else {
            panic!("point param")
        };
        assert!(miss >= sizes.postings, "point set 4 is a miss");
        let fk_walk = (all()[1].params)(&CFG);
        let Value::U64(miss) = fk_walk[3][0] else {
            panic!("fk_walk param")
        };
        assert!(miss >= sizes.accounts, "fk_walk set 4 is a miss");
        let string = (all()[6].params)(&CFG);
        let Value::String(raw) = &string[3][0] else {
            panic!("string param")
        };
        assert!(raw.starts_with(b"missing-"), "string set 4 is a miss");
    }

    #[test]
    fn the_digest_tracks_every_ingredient() {
        let baseline = digest();
        assert_eq!(baseline, digest(), "deterministic");
        // Perturb each ingredient of one family on a copy of the items.
        let items = |perturb: usize| {
            all().iter().enumerate().map(move |(i, f)| {
                let mut name = f.name;
                let mut debug = format!("{:?}", (f.query)());
                let mut sql = f.golden_sql;
                if i == 2 {
                    match perturb {
                        0 => name = "renamed",
                        1 => debug.push('!'),
                        _ => sql = "SELECT 1",
                    }
                }
                (name, debug, sql)
            })
        };
        for perturb in 0..3 {
            assert_ne!(
                digest_over(items(perturb)),
                baseline,
                "perturbation {perturb} must change the digest"
            );
        }
    }

    /// Estimate honesty over the pinned S corpus (docs/architecture/30-execution.md): with
    /// images resident, every family's worst per-node est/actual factor
    /// sits under its pin — the "for good" tripwire for the 114,679x
    /// dishonesty the first benchmark run measured.
    #[test]
    fn estimates_are_honest_over_the_pinned_corpus() {
        let dir = std::env::temp_dir().join("bumbledb-bench-honesty");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let db = bumbledb::Db::create(&dir, schema()).expect("create");
        crate::corpus::load_bumbledb(&db, CFG).expect("load");

        let pin = |name: &str| -> f64 {
            match name {
                "point" | "string" | "fk_walk" | "balance" => 16.0,
                // The cyclic family: the closing edge is fully correlated
                // with the opening one, which a per-step fanout model
                // cannot see — the paper's triangle exists precisely
                // because pairwise estimates explode on cycles. The pin
                // documents the class rather than pretending the
                // estimator can beat it (measured 5.2e3 at S).
                "triangle" => 8192.0,
                _ => 64.0,
            }
        };
        // Estimates are per-plan statics: honesty is judged on each
        // family's *typical* param set — an unskewed hit. The hot sets
        // (balance 0, skew 0/1) and the misses measure execution
        // behavior under skew, which no static estimate can or should
        // match.
        let typical = |name: &str| -> usize {
            match name {
                "balance" => 1,
                "skew" => 2,
                _ => 0,
            }
        };
        for family in all() {
            let query = (family.query)();
            let mut prepared = db.prepare(&query).expect("prepare");
            let sets = (family.params)(&CFG);
            // Warm: images + views resident before the measured profile.
            for params in &sets {
                db.read(|snap| snap.execute_collect(&mut prepared, params).map(|_| ()))
                    .expect("warm");
            }
            let (_, stats) = db
                .read(|snap| snap.profile(&mut prepared, &sets[typical(family.name)]))
                .expect("profile");
            let mut worst = 1.0_f64;
            #[allow(clippy::cast_precision_loss)]
            for node in &stats.nodes {
                let (est, act) = (node.estimate.max(1) as f64, node.actual.max(1) as f64);
                worst = worst.max((est / act).max(act / est));
            }
            eprintln!("honesty {}: worst factor {worst:.1}", family.name);
            assert!(
                worst <= pin(family.name),
                "{}: worst est/actual factor {worst:.1} exceeds the pin {}\n{stats:?}",
                family.name,
                pin(family.name),
            );
        }
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// PRD 08 (docs/hardening): the balance family is a *true balance* —
    /// two equal-amount postings on one account sum to both, engine and
    /// translated SQL alike (the pre-rebind query collapsed them into
    /// one distinct (account, amount) pair).
    /// The minimal consistent slice: one holder/account, two postings of
    /// amount 5 on distinct transfers (every FK target present).
    fn equal_amount_slice() -> Vec<(bumbledb::RelationId, Vec<Value>)> {
        vec![
            (ids::CURRENCY, vec![Value::U64(0), s("USD")]),
            (ids::HOLDER, vec![Value::U64(0), s("h"), Value::Enum(0)]),
            (
                ids::INSTRUMENT,
                vec![Value::U64(0), s("SYM"), Value::U64(0), Value::Enum(0)],
            ),
            (
                ids::ACCOUNT,
                vec![
                    Value::U64(0),
                    Value::U64(0),
                    Value::U64(0),
                    Value::Enum(0),
                    Value::I64(0),
                ],
            ),
            (
                ids::TRANSFER,
                vec![
                    Value::U64(0),
                    Value::I64(0),
                    Value::Bytes(vec![0; 16].into()),
                ],
            ),
            (
                ids::TRANSFER,
                vec![
                    Value::U64(1),
                    Value::I64(1),
                    Value::Bytes(vec![1; 16].into()),
                ],
            ),
            (
                ids::POSTING,
                vec![
                    Value::U64(0),
                    Value::U64(0),
                    Value::U64(0),
                    Value::U64(0),
                    Value::I64(5),
                    Value::I64(0),
                    s("m"),
                    Value::Bool(false),
                ],
            ),
            (
                ids::POSTING,
                vec![
                    Value::U64(1),
                    Value::U64(1),
                    Value::U64(0),
                    Value::U64(0),
                    Value::I64(5),
                    Value::I64(1),
                    s("m"),
                    Value::Bool(false),
                ],
            ),
        ]
    }

    #[test]
    fn balance_counts_equal_amounts_separately() {
        let rows = equal_amount_slice();
        // Engine side.
        let dir = std::env::temp_dir().join("bumbledb-bench-true-balance");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let db = bumbledb::Db::create(&dir, schema()).expect("create");
        db.write(|tx| {
            for (rel, values) in &rows {
                tx.insert_dyn(*rel, values)?;
            }
            Ok(())
        })
        .expect("seed");
        let mut prepared = db.prepare(&balance_query()).expect("prepare");
        let out = db
            .read(|snap| snap.execute_collect(&mut prepared, &[Value::U64(0)]))
            .expect("execute");
        assert_eq!(out.len(), 1);
        assert_eq!(
            out.get(0, 1),
            bumbledb::ResultValue::I64(10),
            "both amount-5 postings count"
        );

        // Translated-SQL side, over the identical rows.
        let conn = rusqlite::Connection::open_in_memory().expect("sqlite");
        for statement in crate::sqlmap::ddl(schema()) {
            conn.execute(&statement, []).expect("ddl");
        }
        for (rel, values) in &rows {
            let relation = schema().relation(*rel);
            let placeholders = (1..=relation.fields().len())
                .map(|i| format!("?{i}"))
                .collect::<Vec<_>>()
                .join(", ");
            let params: Vec<rusqlite::types::Value> =
                values.iter().map(crate::sqlmap::to_sql_value).collect();
            conn.execute(
                &format!(
                    "INSERT INTO \"{}\" VALUES ({placeholders})",
                    relation.name()
                ),
                rusqlite::params_from_iter(params),
            )
            .expect("insert");
        }
        let sum: i64 = conn
            .query_row(goldens::BALANCE, [0i64], |row| row.get(1))
            .expect("query");
        assert_eq!(sum, 10, "the golden agrees");

        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn s(text: &str) -> Value {
        Value::String(text.as_bytes().into())
    }

    /// The generator attaches the skew params' tags to hot accounts at S.
    #[test]
    fn skew_tags_are_hot_attached() {
        let sizes = Sizes::of(CFG.scale);
        let hot = sizes.hot_accounts();
        let attached: std::collections::HashSet<u64> = (0..sizes.account_tags)
            .map(|i| gen::account_tag_pair(&sizes, i))
            .filter(|(account, _)| *account < hot)
            .map(|(_, tag)| tag)
            .collect();
        for tag in SKEW_HOT_TAGS {
            assert!(attached.contains(&tag), "tag {tag} not hot-attached");
        }
    }

    #[test]
    fn the_query_list_renders_all_ten_sections() {
        let md = render_queries_md();
        assert!(md.starts_with("# The read query families"));
        for family in all() {
            assert!(
                md.contains(&format!("## {}", family.name)),
                "{}",
                family.name
            );
            assert!(md.contains(family.golden_sql), "{} sql", family.name);
            assert!(md.contains(family.param_policy), "{} policy", family.name);
        }
        assert!(md.contains("Family-list digest: `"));
        assert_eq!(md.matches("```sql").count(), 10);
    }
}
