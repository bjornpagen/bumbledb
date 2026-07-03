//! The eight gated read families (docs/benchmarks/14): exact IR, exact
//! param policy, hand-written SQL golden, gate classification. This file
//! of queries **is** the benchmark's identity — `digest()` keys the
//! verify stamp and every report on it.

use bumbledb::{AggOp, Atom, CmpOp, Comparison, FindTerm, ParamId, Query, Term, Value, VarId};

use crate::gen::{self, GenConfig, Rng, Sizes};
use crate::schema::ids;
use crate::translate::goldens;

/// Whether a family gates the suite (loses ⇒ the run fails) or merely
/// reports. All eight read families gate (`00-product.md`: every family
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
    /// Hand-written (docs/benchmarks/09) — never regenerated from the
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

/// balance — `Q(a, Sum(amount)) :- Posting(account = a, amount),
/// Account(id = a, holder = ?0)`.
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

/// The registry: the eight, in the suite's canonical order.
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
    fn all_eight_validate_and_prepare() {
        let dir = std::env::temp_dir().join("bumbledb-bench-families");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let db = bumbledb::Db::create(&dir, schema()).expect("create");
        assert_eq!(all().len(), 8);
        for family in all() {
            db.prepare(&(family.query)())
                .unwrap_or_else(|e| panic!("{} fails validation: {e:?}", family.name));
        }
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
            let expected_sets = if family.name == "stats" { 1 } else { 4 };
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
    fn the_query_list_renders_all_eight_sections() {
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
        assert_eq!(md.matches("```sql").count(), 8);
    }
}
