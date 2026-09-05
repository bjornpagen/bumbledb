//! D06/D26 public-path discriminators: complete admit, no-clobber install.

use bumbledb::store::{InstallOutcome, MapPolicy, StoreError, UnreadyStore};
use bumbledb::{ApplyExpected, ApplyOutcome, ChangeSet, Db, Theory, Value, WorkContext};
use bumbledb::schema::ValidateDescriptor as _;

mod common;

bumbledb::schema! {
    pub GateAdmit;

    relation User {
        id: u64 as UserId,
        email: str,
    }

    User(email) -> User;
}

fn work() -> WorkContext {
    common::work()
}

#[test]
fn d26_two_key_conflicting_tuples_then_admit_with_no_delta_rejects() {
    let dir = common::TempDir::new("gate-d26-conflict");
    let dest = dir.path().join("store");
    let schema = GateAdmit.descriptor().validate().expect("schema");
    let ctx = work();
    let unready = UnreadyStore::begin(&dest, &schema, MapPolicy::default(), &ctx).expect("begin");
    let first = {
        let mut builder = ChangeSet::builder(&schema, ctx.clone());
        builder
            .insert(
                bumbledb::RelationId(0),
                &[Value::U64(1), Value::String("dup@ex".into())],
            )
            .expect("insert");
        builder.finish().expect("first")
    };
    let second = {
        let mut builder = ChangeSet::builder(&schema, ctx.clone());
        builder
            .insert(
                bumbledb::RelationId(0),
                &[Value::U64(2), Value::String("dup@ex".into())],
            )
            .expect("insert");
        builder.finish().expect("second")
    };
    unready
        .populate(&ctx, |stage, work| {
            stage.apply(&first, work)?;
            stage.apply(&second, work)?;
            Ok(())
        })
        .expect("populate");
    assert!(
        unready.admit(&schema, &ctx).is_err(),
        "complete admit must reject the conflicting populated state"
    );
    assert!(!dest.exists(), "destination stays absent");
}

#[test]
fn d06_two_installers_never_overwrite() {
    let dir = common::TempDir::new("gate-d06-two");
    let dest = dir.path().join("store");
    let schema = GateAdmit.descriptor().validate().expect("schema");
    let ctx = work();
    let first = UnreadyStore::begin(&dest, &schema, MapPolicy::default(), &ctx).expect("first");
    let second = UnreadyStore::begin(&dest, &schema, MapPolicy::default(), &ctx).expect("second");
    let row = {
        let mut builder = ChangeSet::builder(&schema, ctx.clone());
        builder
            .insert(
                bumbledb::RelationId(0),
                &[Value::U64(1), Value::String("a@ex".into())],
            )
            .expect("insert");
        builder.finish().expect("row")
    };
    first
        .populate(&ctx, |stage, work| stage.apply(&row, work).map(|_| ()))
        .expect("populate");
    let admitted = first.admit(&schema, &ctx).expect("admit");
    match admitted.install(&schema, MapPolicy::default(), &ctx) {
        InstallOutcome::Installed(store) => drop(store),
        other => panic!("first installer publishes, got {other:?}"),
    }
    match second
        .admit(&schema, &ctx)
        .expect("second admit of empty sibling")
        .install(&schema, MapPolicy::default(), &ctx)
    {
        InstallOutcome::NotInstalled { cleanup, detail } => {
            assert!(matches!(detail, StoreError::DestinationExists { .. }));
            cleanup.abandon();
            assert!(dest.exists());
        }
        other => panic!("second must not overwrite, got {other:?}"),
    }
    let _reopen = Db::<GateAdmit>::open(&dest, GateAdmit, work()).expect("reopen winner");
}

/// After complete admit+install, ordinary apply is incremental: a new
/// email commits, a duplicate email is `InvariantRejected`, and the
/// owned pin still sees only the admitted rows. Verification NotRun.
#[test]
fn apply_after_admit_install_rejects_conflict_and_pins() {
    let dir = common::TempDir::new("gate-apply-after-admit");
    let dest = dir.path().join("store");
    let schema = GateAdmit.descriptor().validate().expect("schema");
    let ctx = work();
    let unready = UnreadyStore::begin(&dest, &schema, MapPolicy::default(), &ctx).expect("begin");
    let first = {
        let mut builder = ChangeSet::builder(&schema, ctx.clone());
        builder
            .insert(
                bumbledb::RelationId(0),
                &[Value::U64(1), Value::String("a@ex".into())],
            )
            .expect("insert");
        builder.finish().expect("first")
    };
    unready
        .populate(&ctx, |stage, work| stage.apply(&first, work).map(|_| ()))
        .expect("populate");
    let admitted = unready.admit(&schema, &ctx).expect("complete admit");
    match admitted.install(&schema, MapPolicy::default(), &ctx) {
        InstallOutcome::Installed(_) => {}
        other => panic!("install must publish, got {other:?}"),
    }
    let db = Db::<GateAdmit>::open(&dest, GateAdmit, work()).expect("open");
    let second = {
        let mut builder = ChangeSet::builder(db.schema(), ctx.clone());
        builder
            .insert(
                bumbledb::RelationId(0),
                &[Value::U64(2), Value::String("b@ex".into())],
            )
            .expect("insert");
        builder.finish().expect("second")
    };
    match db
        .apply(&second, ApplyExpected::Any, &ctx)
        .expect("incremental apply")
    {
        ApplyOutcome::Accepted { .. } => {}
        ApplyOutcome::NoChange { .. }
        | ApplyOutcome::InvariantRejected { .. }
        | ApplyOutcome::Moved { .. } => panic!("a distinct email must accept"),
    }
    let conflict = {
        let mut builder = ChangeSet::builder(db.schema(), ctx.clone());
        builder
            .insert(
                bumbledb::RelationId(0),
                &[Value::U64(3), Value::String("a@ex".into())],
            )
            .expect("insert");
        builder.finish().expect("conflict")
    };
    match db
        .apply(&conflict, ApplyExpected::Any, &ctx)
        .expect("conflict")
    {
        ApplyOutcome::InvariantRejected { violations } => {
            assert!(!violations.is_empty());
        }
        ApplyOutcome::Accepted { .. } | ApplyOutcome::NoChange { .. } => {
            panic!("duplicate email must reject")
        }
        ApplyOutcome::Moved { .. } => panic!("Any cannot Move"),
    }
    let pin = db.owned_read().expect("pin");
    assert_eq!(pin.count(bumbledb::RelationId(0)).expect("count"), 2);
    let frame = pin.frame(&ctx);
    assert!(frame
        .contains_dyn(
            bumbledb::RelationId(0),
            &[Value::U64(1), Value::String("a@ex".into())]
        )
        .expect("first remains"));
    assert!(frame
        .contains_dyn(
            bumbledb::RelationId(0),
            &[Value::U64(2), Value::String("b@ex".into())]
        )
        .expect("second remains"));
    assert!(!frame
        .contains_dyn(
            bumbledb::RelationId(0),
            &[Value::U64(3), Value::String("a@ex".into())]
        )
        .expect("conflict never landed"));
    let empty = ChangeSet::builder(db.schema(), ctx.clone())
        .finish()
        .expect("empty");
    match db
        .apply(&empty, ApplyExpected::Exact(pin.witness()), &ctx)
        .expect("no-change")
    {
        ApplyOutcome::NoChange { .. } => {}
        ApplyOutcome::Accepted { .. }
        | ApplyOutcome::InvariantRejected { .. }
        | ApplyOutcome::Moved { .. } => panic!("empty apply under the pin's witness is NoChange"),
    }
    assert!(pin.generation_handle().strong_count() >= 1);
    match db.close() {
        bumbledb::CloseReport::Incomplete {
            live_transactions, ..
        } => assert!(live_transactions >= 1),
        bumbledb::CloseReport::Closed => panic!("close cannot complete under a live pin"),
    }
    drop(pin);
    assert_eq!(db.close(), bumbledb::CloseReport::Closed);
}
