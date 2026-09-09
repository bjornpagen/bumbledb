//! Bounded allocation/timing controls on the frozen S/seed-1 corpus.
//! Numeric SQL goldens are independent of the engine's rendered queries.
use bumbledb::{
    Atom, AtomSource, CmpOp, Comparison, ConditionTree, FieldId, FindTerm, FoldOp, HeadTerm,
    Interior, InteriorId, NonEmpty, ParamId, Query, Rec, RecRule, RecStep, RelationId, Rule,
    ScalarExpr, Term, Value, VarId,
};

pub const CASES: &[(&str, &[u64], &str)] = &[
    (
        "computed",
        &[0, 5, 513, 8192, 100000],
        "SELECT id,amount FROM Posting WHERE id<:n ORDER BY id,amount",
    ),
    (
        "union",
        &[0, 5, 513, 8192, 100000],
        "SELECT id,amount FROM Posting WHERE id<:n UNION SELECT id,amount FROM Posting WHERE entry<:n ORDER BY id,amount",
    ),
    (
        "interior",
        &[0, 5, 513, 8192, 100000],
        "SELECT id,amount FROM Posting WHERE id<:n ORDER BY id,amount",
    ),
    (
        "groups",
        &[0, 5, 513, 8192, 100000],
        "SELECT id,sum(amount) FROM Posting WHERE id<:n GROUP BY id ORDER BY id",
    ),
    (
        "pairs",
        &[0, 5, 500],
        "SELECT account,sum(amount) FROM (SELECT DISTINCT account,amount FROM Posting WHERE account<:n) GROUP BY account ORDER BY account",
    ),
    (
        "union_groups",
        &[0, 5, 500],
        "SELECT account,sum(amount) FROM (SELECT account,amount FROM Posting WHERE account<:n UNION SELECT account,amount FROM Posting WHERE account<=:n) GROUP BY account ORDER BY account",
    ),
    (
        "dnf_groups",
        &[0, 5, 500],
        "SELECT account,sum(amount) FROM (SELECT DISTINCT account,amount FROM Posting WHERE account<:n OR account<=:n) GROUP BY account ORDER BY account",
    ),
    (
        "dense",
        &[0, 5, 500],
        "SELECT currency,count(*) FROM Account WHERE id<:n GROUP BY currency ORDER BY currency",
    ),
    (
        "point",
        &[0, 50000, 1000000],
        "SELECT amount,at FROM Posting WHERE id=:n ORDER BY amount,at",
    ),
    (
        "reach",
        &[0, 1, 63],
        "WITH RECURSIVE r(id) AS (SELECT parent FROM OrgParent WHERE child=:n UNION SELECT p.parent FROM OrgParent p JOIN r ON p.child=r.id) SELECT id FROM r ORDER BY id",
    ),
    (
        "interior_reach",
        &[0, 1, 63],
        "WITH RECURSIVE r(id) AS (SELECT parent FROM OrgParent WHERE child=:n UNION SELECT p.parent FROM OrgParent p JOIN r ON p.child=r.id) SELECT id FROM r ORDER BY id",
    ),
];

fn v(id: u16) -> Term {
    Term::Var(VarId(id))
}
fn find(id: u16) -> FindTerm {
    FindTerm::Var(VarId(id))
}
fn atom(relation: u32, fields: &[(u16, u16)]) -> Atom {
    Atom {
        source: AtomSource::Edb(RelationId(relation)),
        bindings: fields
            .iter()
            .map(|&(field, var)| (FieldId(field), v(var)))
            .collect(),
    }
}
fn bound(var: u16, op: CmpOp) -> ConditionTree {
    ConditionTree::Leaf(Comparison {
        op,
        lhs: v(var),
        rhs: Term::Param(ParamId(0)),
    })
}
fn posting() -> Rule {
    Rule {
        finds: vec![find(0), find(1)],
        atoms: vec![atom(4, &[(0, 0), (4, 1)])],
        negated: vec![],
        conditions: vec![bound(0, CmpOp::Lt)],
    }
}
fn grouped_pairs() -> Rule {
    Rule {
        finds: vec![
            find(0),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
            },
        ],
        atoms: vec![atom(4, &[(2, 0), (4, 1)])],
        negated: vec![],
        conditions: vec![bound(0, CmpOp::Lt)],
    }
}
fn read_interior(id: u32, width: u16) -> Rule {
    Rule {
        finds: (0..width).map(find).collect(),
        atoms: vec![Atom {
            source: AtomSource::Interior(InteriorId(id)),
            bindings: (0..width).map(|i| (FieldId(i), v(i))).collect(),
        }],
        negated: vec![],
        conditions: vec![],
    }
}
fn reach(interior: bool) -> Query {
    let edge = |source, child, parent| Atom {
        source,
        bindings: vec![(FieldId(0), child), (FieldId(1), parent)],
    };
    let source = if interior {
        AtomSource::Interior(InteriorId(0))
    } else {
        AtomSource::Edb(RelationId(7))
    };
    let main = read_interior(u32::from(interior), 1);
    Query::reach(
        if interior {
            vec![Interior {
                rules: vec![Rule {
                    finds: vec![find(0), find(1)],
                    atoms: vec![atom(7, &[(0, 0), (1, 1)])],
                    negated: vec![],
                    conditions: vec![],
                }],
            }]
        } else {
            vec![]
        },
        Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0)],
                atoms: vec![edge(source, Term::Param(ParamId(0)), v(0))],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(1)],
                self_bindings: vec![(FieldId(0), v(0))],
                atoms: vec![edge(source, v(0), v(1))],
                conditions: vec![],
            }),
        },
        vec![HeadTerm::Var],
        vec![main],
    )
}

pub fn query(name: &str) -> Query {
    match name {
        "computed" => {
            let mut rule = posting();
            rule.finds[0] = FindTerm::Compute(ScalarExpr::Add(
                Box::new(ScalarExpr::Var(VarId(0))),
                Box::new(ScalarExpr::Literal(Value::U64(0))),
            ));
            Query::single(rule)
        }
        "union" => {
            let rule = posting();
            let mut second = rule.clone();
            second.atoms[0].bindings.push((FieldId(1), v(2)));
            second.conditions = vec![bound(2, CmpOp::Lt)];
            Query::cq(vec![], rule.head(), vec![rule, second])
        }
        "interior" => {
            let main = read_interior(0, 2);
            Query::cq(
                vec![Interior {
                    rules: vec![posting()],
                }],
                main.head(),
                vec![main],
            )
        }
        "groups" => {
            let mut rule = posting();
            rule.finds[1] = FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
            };
            Query::single(rule)
        }
        "pairs" => Query::single(grouped_pairs()),
        "union_groups" => {
            let rule = grouped_pairs();
            let mut second = rule.clone();
            second.conditions = vec![bound(0, CmpOp::Le)];
            Query::cq(vec![], rule.head(), vec![rule, second])
        }
        "dnf_groups" => {
            let mut rule = grouped_pairs();
            rule.conditions = vec![ConditionTree::Or(vec![
                bound(0, CmpOp::Lt),
                bound(0, CmpOp::Le),
            ])];
            Query::single(rule)
        }
        "dense" => Query::single(Rule {
            finds: vec![find(1), FindTerm::Count],
            atoms: vec![atom(1, &[(0, 0), (2, 1)])],
            negated: vec![],
            conditions: vec![bound(0, CmpOp::Lt)],
        }),
        "point" => Query::single(Rule {
            finds: vec![find(0), find(1)],
            atoms: vec![Atom {
                source: AtomSource::Edb(RelationId(4)),
                bindings: vec![
                    (FieldId(0), Term::Param(ParamId(0))),
                    (FieldId(4), v(0)),
                    (FieldId(5), v(1)),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        }),
        "reach" => reach(false),
        "interior_reach" => reach(true),
        _ => panic!("unknown control {name}"),
    }
}

pub fn expected(path: &std::path::Path, sql: &str, n: u64) -> Vec<Vec<i64>> {
    let output = std::process::Command::new("sqlite3")
        .args(["-readonly", "-csv"])
        .arg(path)
        .arg(sql.replace(":n", &n.to_string()))
        .output()
        .expect("SQLite oracle");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| {
            line.split(',')
                .map(|value| value.parse().unwrap())
                .collect()
        })
        .collect()
}
