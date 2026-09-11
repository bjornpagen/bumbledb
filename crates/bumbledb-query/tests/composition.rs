//! Typed query composition and structural literals, exercised end to end:
//!
//! - `use name = &template;` — NONRECURSIVE COMPOSITION: an existing
//!   schema-bound typed query value splices into the importing query's
//!   derived-stage roster as owned immutable IR (interior-id shifted,
//!   cloned, never a borrow of a database or session);
//! - interior heads may aggregate; recursive cycles stay projection-only;
//! - `uuid:"…"` literals and dense `f64..f64` interval literals lower to
//!   the canonical `Value::Uuid` / `Value::IntervalF64`.

use bumbledb::ir::Value;
use bumbledb::{Atom, AtomSource, Db, FindTerm, FoldOp, Interval, Query, Term, Uuid, VarId};
use bumbledb_query::query;

mod common;
use common::TempDir;

mod learning {
    bumbledb::schema! {
        pub Learning;

        relation Student { id: uuid as StudentId, name: str, budget: u64 }
        relation Attempt {
            id: uuid as AttemptId,
            student: uuid as StudentId,
            score: f64,
            units: u64,
            active: interval<i64>,
        }

        Student(id) -> Student;
        Attempt(id) -> Attempt;
        Attempt(student) <= Student(id);
        Student(id) <=[units]{0..budget} Attempt(student);
    }
}

use learning::Learning;

fn validated(tag: &str, query: &Query) {
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), Learning, common::work())
        .expect("create the Learning store")
        .expect("accepted");
    db.prepare(query, common::work())
        .expect("the composed query validates");
}

/// Chapter 34's exact composition: `attempt_stats` is a grouped aggregate
/// template; `student_summary` imports it with `use` and joins it against
/// `Student` — the same relation-expression nodes the TS `match(imported)`
/// splice builds.
#[test]
fn use_imports_a_nonrecursive_aggregate_template() {
    let attempt_stats = query!(Learning {
        (student, total: Sum(score), mean: Mean(score)) | Attempt(id, student, score);
    });

    let student_summary = query!(Learning {
        use stats = &attempt_stats;
        (student, name, total, mean) |
            stats(student, total, mean), Student(id: student, name);
    });

    // The import landed as ONE derived stage: the imported query has no
    // interiors of its own, so the head stage is interior 0 and main joins
    // it with Student.
    assert!(student_summary.rec().is_none(), "composition stays a CQ");
    assert_eq!(student_summary.interiors().len(), 1, "one spliced stage");
    let stage = &student_summary.interiors()[0];
    assert_eq!(
        stage.rules, attempt_stats.rules,
        "the imported main rules are the stage, verbatim"
    );
    // The imported template itself is untouched (owned immutable IR).
    assert_eq!(attempt_stats.interiors().len(), 0);

    // Main reads the stage through Interior(0).
    let main = &student_summary.rules()[0];
    assert!(
        main.atoms
            .iter()
            .any(|atom| atom.source == AtomSource::Interior(bumbledb::InteriorId(0))),
        "main joins the imported stage"
    );

    validated("compose-use", &student_summary);
}

/// An import WITH its own interiors: every internal `Interior(id)`
/// reference shifts by the splice offset, and a second import stacks after
/// the first (its head stage id observes the earlier import's stages).
#[test]
fn use_imports_shift_internal_interior_ids() {
    let staged = query!(Learning {
        interior passing(student) | Attempt(id, student, score), score > 0.5;
        (student, n: Count) | passing(student);
    });
    assert_eq!(
        staged.interiors().len(),
        1,
        "the base template has one interior"
    );

    let doubled = query!(Learning {
        use a = &staged;
        use b = &staged;
        (student) | a(student, n), b(student, n), Student(id: student, name);
    });

    // Import a: stages 0 (passing) and 1 (head). Import b: stages 2 and 3.
    assert_eq!(doubled.interiors().len(), 4, "two imports, two stages each");
    let head_a = &doubled.interiors()[1];
    let head_b = &doubled.interiors()[3];
    // a's head stage reads ITS OWN passing stage at the shifted id 0;
    // b's reads id 2 (shifted by a's two stages).
    let reads = |rule: &bumbledb::Rule| -> Vec<AtomSource> {
        rule.atoms.iter().map(|atom| atom.source).collect()
    };
    assert!(
        reads(&head_a.rules[0]).contains(&AtomSource::Interior(bumbledb::InteriorId(0))),
        "import a reads its own stage at the unshifted base"
    );
    assert!(
        reads(&head_b.rules[0]).contains(&AtomSource::Interior(bumbledb::InteriorId(2))),
        "import b's internal reference shifted past import a's stages"
    );

    validated("compose-shift", &doubled);
}

/// A recursive template refuses `use` at construction: nothing aggregates
/// or splices through the feedback cycle; a completed recursive result is
/// consumed by downstream queries, not imported as a stage.
#[test]
#[should_panic(expected = "NONRECURSIVE")]
fn use_refuses_a_recursive_template() {
    let reach = query!(Learning {
        rec reach(s) | Attempt(id, student: s);
        rec reach(s) | Attempt(id: a, student: s), reach(s);
        (s) | reach(s);
    });
    let _ = query!(Learning {
        use r = &reach;
        (s) | r(s);
    });
}

/// A parameterized template refuses `use`: an imported stage cannot bind
/// the importing query's positional params (values are supplied in the
/// importing query's own atoms).
#[test]
#[should_panic(expected = "parameterless")]
fn use_refuses_a_parameterized_template() {
    let for_student = query!(Learning {
        (score) | Attempt(id, student == ?student, score);
    });
    let _ = query!(Learning {
        use scores = &for_student;
        (score) | scores(score);
    });
}

/// Interior heads may aggregate now (the deleted projection-only wall):
/// the declared interior computes a grouped sum and main consumes it.
#[test]
fn interior_heads_aggregate() {
    let per_student = query!(Learning {
        interior totals(student, total: Sum(units)) | Attempt(id, student, units);
        (student, total) | totals(student, total), Student(id: student, name);
    });
    let stage = &per_student.interiors()[0];
    let finds = &stage.rules[0].finds;
    assert_eq!(finds.len(), 2);
    assert!(
        matches!(
            finds[1],
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(_)
            }
        ),
        "the interior head carries the fold"
    );
    validated("interior-aggregate", &per_student);
}

/// `uuid:"…"` lowers to the canonical `Value::Uuid` in selections and
/// comparison terms; the canonical hyphenated UUID spelling is the only
/// accepted image (compile-fail fixtures pin the refusals).
#[test]
fn uuid_literals_lower_canonically() {
    let expected = Uuid::parse_str("00112233445566778899aabbccddeeff").expect("canonical hex");
    let by_lit = query!(Learning {
        (name) | Student(id == uuid:"00112233445566778899aabbccddeeff", name);
    });
    let rule = &by_lit.rules()[0];
    let atom: &Atom = &rule.atoms[0];
    assert!(
        atom.bindings
            .iter()
            .any(|(_, term)| *term == Term::Literal(Value::Uuid(expected))),
        "the selection carries the canonical Uuid value"
    );
    validated("uuid-literal", &by_lit);
}

/// `0.25..1.5` lowers to the canonical dense `Value::IntervalF64`
/// (half-open, nonempty, `-0` canonicalized) — chapter 11's parameterized
/// dense float interval as an ordinary literal.
#[test]
fn float_interval_literals_lower_canonically() {
    let f = |v: f64| bumbledb::F64::from(v);
    let expected = Interval::<bumbledb::F64>::new(f(0.25), f(1.5)).expect("nonempty");
    let scored = query!(Learning {
        (student) | Attempt(id, student, score), score in 0.25..1.5;
    });
    let rule = &scored.rules()[0];
    let bumbledb::ConditionTree::Leaf(cmp) = &rule.conditions[0] else {
        panic!("the membership condition is one comparison leaf");
    };
    assert!(matches!(cmp.op, bumbledb::CmpOp::PointIn));
    assert_eq!(
        cmp.lhs,
        Term::Literal(Value::IntervalF64(expected)),
        "the container is the canonical dense interval"
    );
    validated("float-interval-literal", &scored);
}
