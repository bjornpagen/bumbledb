//! Chapter 34's typed templates and `params!` — the Rust half of the
//! compile-time param/row typing parity (API-12; C05 bind roster):
//!
//! - `query!` evaluates to a typed TEMPLATE wrapping the owned immutable
//!   IR: `Deref<Target = Query>` keeps every untyped consumer compiling,
//!   `into_query()` moves the plain IR out, `param_names()`/`columns()`
//!   carry the name tables that used to die at expansion;
//! - `template.bind(params! { name: value, … })` is order-free typed
//!   named binding onto the positional C05 `Vec<ParamArg>` — unknown,
//!   missing and doubled names are COMPILE errors (typestate builder;
//!   compile-fail fixtures pin them), and value-vs-slot type agreement
//!   stays the engine's typed bind error at execution;
//! - `field in ?set` params bind as `&[Value]` slices.
//!
//! Verification: `NotRun` until F3 (campaign phase rule).

use bumbledb::{AnswerValue, Answers, BindValue, Db, Id128, ParamArg, Value};
use bumbledb_query::{params, query};

mod common;
use common::TempDir;

mod learning {
    bumbledb::schema! {
        pub Learning;

        relation Student { id: id128 as StudentId, name: str, budget: u64 }
        relation Attempt {
            id: id128 as AttemptId,
            student: id128 as StudentId,
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

use learning::{Attempt, AttemptId, Learning, Student, StudentId};

fn id(byte: u8) -> Id128 {
    Id128::from_bytes([byte; 16])
}

fn f(value: f64) -> bumbledb::F64 {
    bumbledb::F64::from(value)
}

/// Seeds one student with three attempts (scores 0.2 / 0.6 / 0.9, units
/// 1 / 2 / 3) and a second student with one attempt (score 0.8, units 5).
fn seeded(tag: &str) -> (TempDir, Db<Learning>) {
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), Learning)
        .expect("create the Learning store")
        .expect("accepted");
    db.write(|tx| {
        tx.insert([
            &Student {
                id: StudentId(id(1)),
                name: "ada",
                budget: 100,
            },
            &Student {
                id: StudentId(id(2)),
                name: "grace",
                budget: 100,
            },
        ])?;
        let span = bumbledb::Interval::new(0i64, 60i64).expect("nonempty");
        tx.insert([
            &Attempt {
                id: AttemptId(id(11)),
                student: StudentId(id(1)),
                score: f(0.2),
                units: 1,
                active: span,
            },
            &Attempt {
                id: AttemptId(id(12)),
                student: StudentId(id(1)),
                score: f(0.6),
                units: 2,
                active: span,
            },
            &Attempt {
                id: AttemptId(id(13)),
                student: StudentId(id(1)),
                score: f(0.9),
                units: 3,
                active: span,
            },
            &Attempt {
                id: AttemptId(id(14)),
                student: StudentId(id(2)),
                score: f(0.8),
                units: 5,
                active: span,
            },
        ])?;
        Ok(())
    })
    .expect("seed the store")
    .unwrap();
    (dir, db)
}

fn units_of(out: &Answers) -> Vec<u64> {
    let mut units: Vec<u64> = (0..out.len())
        .map(|answer| {
            let AnswerValue::U64(value) = out.get(answer, 0) else {
                panic!("the finds project one u64 column");
            };
            value
        })
        .collect();
    units.sort_unstable();
    units
}

/// The chapter 34 shape: named order-free binding produces exactly the
/// positional bind's answers — `params!` is construction, not a second
/// bind semantics.
#[test]
fn named_binds_match_positional_binds_order_free() {
    let attempts_for = query!(Learning {
        (units) | Attempt(id, student == ?student, score, units), score > ?floor;
    });
    assert_eq!(attempts_for.param_names(), ["student", "floor"]);
    assert_eq!(attempts_for.columns(), ["units"]);

    let (_dir, db) = seeded("typed-named-binds");
    let mut prepared = db.prepare(&attempts_for).expect("the template validates");
    db.read(|snap| {
        // Positional: ParamId order is first use (student, then floor).
        let positional = snap.execute_collect(
            &mut prepared,
            &[BindValue::Id128(id(1)), BindValue::F64(f(0.5))],
        )?;
        // Named, REVERSED order: the builder aims each value by name.
        let named = attempts_for.bind(params! { floor: 0.5f64, student: id(1) });
        let bound = snap.execute_collect(&mut prepared, &named)?;
        assert_eq!(units_of(&positional), vec![2, 3], "the seeded rows filter");
        assert_eq!(
            units_of(&bound),
            units_of(&positional),
            "order-free named binding agrees"
        );
        Ok(())
    })
    .expect("both bind spellings execute");
}

/// A `field in ?set` param binds a `&[Value]` slice; rebinding the same
/// template with a different set is an ordinary re-execution.
#[test]
fn set_params_bind_value_slices() {
    let sized = query!(Learning {
        (score) | Attempt(id, score, units in ?sizes);
    });
    assert_eq!(sized.param_names(), ["sizes"]);

    let scores_of = |out: &Answers| -> Vec<u64> {
        let mut bits: Vec<u64> = (0..out.len())
            .map(|answer| {
                let AnswerValue::F64(value) = out.get(answer, 0) else {
                    panic!("the finds project one f64 column");
                };
                value.to_bits()
            })
            .collect();
        bits.sort_unstable();
        bits
    };
    let expected = |values: &[f64]| -> Vec<u64> {
        let mut bits: Vec<u64> = values.iter().map(|v| f(*v).to_bits()).collect();
        bits.sort_unstable();
        bits
    };

    let (_dir, db) = seeded("typed-set-binds");
    let mut prepared = db.prepare(&sized).expect("the template validates");
    db.read(|snap| {
        let small = [Value::U64(1), Value::U64(2)];
        let out = snap.execute_collect(&mut prepared, &sized.bind(params! { sizes: &small }))?;
        assert_eq!(scores_of(&out), expected(&[0.2, 0.6]));
        let large = [Value::U64(5)];
        let out = snap.execute_collect(&mut prepared, &sized.bind(params! { sizes: &large }))?;
        assert_eq!(scores_of(&out), expected(&[0.8]));
        Ok(())
    })
    .expect("set binds execute");
}

/// A zero-param template still binds (`params! {}` is the empty builder),
/// and the untyped positional path stays available beside it.
#[test]
fn zero_param_templates_bind_empty() {
    let all = query!(Learning {
        (units) | Attempt(id, units);
    });
    assert!(
        all.param_names().is_empty(),
        "a paramless template has no names"
    );
    let (_dir, db) = seeded("typed-zero-params");
    let mut prepared = db.prepare(&all).expect("the template validates");
    db.read(|snap| {
        let named = all.bind(params! {});
        assert!(named.is_empty(), "no params, no args");
        let out = snap.execute_collect(&mut prepared, &named)?;
        assert_eq!(units_of(&out), vec![1, 2, 3, 5]);
        let positional: &[ParamArg<'_>] = &[];
        let out = snap.execute_collect(&mut prepared, positional)?;
        assert_eq!(units_of(&out), vec![1, 2, 3, 5]);
        Ok(())
    })
    .expect("the paramless template executes");
}

/// The template's column table: projected variables keep their names,
/// `name:` labels name aggregates, and an unlabeled aggregate renders as
/// its spelling — the row-typing half of the chapter 34 parity (Rust rows
/// stay the typed `AnswerValue` roster; the names come from here).
#[test]
fn columns_carry_head_names_and_labels() {
    let stats = query!(Learning {
        (student, total: Sum(units), Count) | Attempt(id, student, units);
    });
    assert_eq!(stats.columns(), ["student", "total", "Count"]);
    let unlabeled = query!(Learning {
        (student, Sum(units)) | Attempt(id, student, units);
    });
    assert_eq!(unlabeled.columns(), ["student", "Sum(units)"]);
}

/// `into_query()` moves the plain owned IR out — identical to the IR a
/// second expansion of the same tokens builds, and the value the roster
/// arrays / cross-signature carriers use.
#[test]
fn into_query_is_the_plain_ir() {
    let a = query!(Learning {
        (units) | Attempt(id, units);
    });
    let b = query!(Learning {
        (units) | Attempt(id, units);
    });
    let ir = a.into_query();
    assert_eq!(&ir, b.query(), "one notation, one IR");
    // Deref half: the borrowed view is the same IR.
    assert_eq!(ir.rules().len(), b.rules().len());
}

/// The engine's typed bind refusal is the runtime half of `bind(...)`:
/// a name aimed at the right slot with the WRONG value kind fails at
/// execution as `ParamTypeMismatch` — never a silent coercion.
#[test]
fn wrong_value_kinds_stay_typed_bind_errors() {
    let attempts_for = query!(Learning {
        (units) | Attempt(id, student == ?student, units);
    });
    let (_dir, db) = seeded("typed-bind-mismatch");
    let mut prepared = db.prepare(&attempts_for).expect("the template validates");
    db.read(|snap| {
        let bound = attempts_for.bind(params! { student: 7u64 });
        let error = snap
            .execute_collect(&mut prepared, &bound)
            .expect_err("a u64 in an id128 slot refuses");
        assert!(
            matches!(error, bumbledb::Error::ParamTypeMismatch { .. }),
            "the C05 typed bind error surfaces: {error:?}"
        );
        Ok(())
    })
    .expect("the refusal is typed, not a panic");
}
