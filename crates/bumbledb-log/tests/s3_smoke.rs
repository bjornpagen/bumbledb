//! Credential-gated REAL S3 lane (S3-01/02 core shapes). Emulator green is
//! not S3 qualification; these tests run only with real credentials and are
//! reported skipped — never passed — without them. Required env:
//! `BUMBLEDB_S3_SMOKE_BUCKET`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`;
//! `BUMBLEDB_S3_SMOKE_REGION` defaults to `us-east-1`;
//! `BUMBLEDB_S3_SMOKE_ENDPOINT` optional. The full S3-01..06 qualification
//! (multipart ambiguity, versioning/lifecycle policy, credential rotation)
//! is the F3 campaign over this same adapter with the deployment's actual
//! configuration. Verification: `NotRun` (F1 authors, does not execute).

#![cfg(feature = "store")]

use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

use bumbledb_log::store::s3::{S3Config, S3Credentials, S3Store};
use bumbledb_log::store::{
    ConditionalOutcome, ConditionalStore as _, HeadRead, ObjectKind, ObjectRead, get_verified,
    put_verified,
};

const REQUIRED: [&str; 3] = [
    "BUMBLEDB_S3_SMOKE_BUCKET",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
];

fn credentials_available() -> bool {
    REQUIRED.iter().all(|name| std::env::var(name).is_ok())
}

/// Loud skip: prints WHY the lane did not run. A skipped credential lane is
/// `NotRun`, never qualification.
macro_rules! require_credentials {
    () => {
        if !credentials_available() {
            eprintln!(
                "SKIP (NotRun, not passed): real-S3 lane requires {:?}",
                REQUIRED
            );
            return;
        }
    };
}

fn store() -> S3Store {
    S3Store::new(&S3Config {
        endpoint: std::env::var("BUMBLEDB_S3_SMOKE_ENDPOINT").ok(),
        region: std::env::var("BUMBLEDB_S3_SMOKE_REGION").unwrap_or_else(|_| "us-east-1".into()),
        bucket: std::env::var("BUMBLEDB_S3_SMOKE_BUCKET").expect("bucket env"),
        credentials: S3Credentials::Static {
            access_key_id: std::env::var("AWS_ACCESS_KEY_ID").expect("key env"),
            secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY").expect("secret env"),
            session_token: std::env::var("AWS_SESSION_TOKEN").ok(),
        },
    })
    .expect("S3 store constructs")
}

static PREFIX_SEQ: AtomicU64 = AtomicU64::new(0);

fn fresh_prefix(tag: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    format!(
        "bdb-p05-smoke/{tag}-{}-{nanos}-{}",
        std::process::id(),
        PREFIX_SEQ.fetch_add(1, Ordering::Relaxed)
    )
}

fn cleanup(store: &S3Store, prefix: &str) {
    if let Ok(page) = store.list_objects(&format!("{prefix}/"), None) {
        for key in page.keys {
            let _ = store.delete_object(&key);
        }
    }
    let _ = store.delete_object(&format!("{prefix}/HEAD"));
}

#[test]
fn s3_01_conditional_create_and_exact_version_replacement_have_one_winner() {
    require_credentials!();
    let store = store();
    let prefix = fresh_prefix("cas");
    let head_key = format!("{prefix}/HEAD");
    // Conditional create: first wins, second definitively loses.
    let v1 = match store.create_head(&head_key, b"rev-1").expect("create") {
        ConditionalOutcome::Published { version } => version,
        other => panic!("first create publishes: {other:?}"),
    };
    assert_eq!(
        store
            .create_head(&head_key, b"rev-1-imposter")
            .expect("second create"),
        ConditionalOutcome::PreconditionFailed,
        "a never-reused head is never re-created over"
    );
    // Exact-version race: two contenders, one captured version — at most one
    // winner, and a loser is a DEFINITE loss even under identical fact bytes
    // (no ABA: every proposed body differs through its revision).
    let store_b = self::store();
    let (outcome_a, outcome_b) = thread::scope(|scope| {
        let key_a = head_key.clone();
        let v_a = v1.clone();
        let store_ref = &store;
        let a = scope.spawn(move || {
            store_ref
                .replace_head(&key_a, &v_a, b"rev-2-from-a")
                .expect("a swaps")
        });
        let outcome_b = store_b
            .replace_head(&head_key, &v1, b"rev-2-from-b")
            .expect("b swaps");
        (a.join().expect("a thread"), outcome_b)
    });
    let wins = [&outcome_a, &outcome_b]
        .iter()
        .filter(|outcome| matches!(outcome, ConditionalOutcome::Published { .. }))
        .count();
    let losses = [&outcome_a, &outcome_b]
        .iter()
        .filter(|outcome| matches!(outcome, ConditionalOutcome::PreconditionFailed))
        .count();
    // Real transports may legitimately return Indeterminate; that arm is
    // uncertainty, never a second win.
    assert!(wins <= 1, "at most one publication per observed version");
    assert!(wins + losses >= 1, "the race terminated observably");
    match store.read_head(&format!("{prefix}/HEAD")).expect("read") {
        HeadRead::Present { body, .. } => {
            assert!(
                &*body == b"rev-2-from-a" || &*body == b"rev-2-from-b",
                "exactly one contender's body holds"
            );
        }
        HeadRead::Absent => panic!("the head exists"),
    }
    cleanup(&store, &prefix);
}

#[test]
fn s3_02_immutable_objects_verify_and_stale_versions_lose_after_movement() {
    require_credentials!();
    let store = store();
    let prefix = fresh_prefix("objects");
    // Content-addressed immutable object: store, verify, idempotent re-put.
    let reference = put_verified(
        &store,
        &prefix,
        1,
        ObjectKind::Chunk,
        b"real-s3 chunk bytes",
    )
    .expect("put");
    assert_eq!(
        get_verified(&store, &prefix, &reference).expect("verified"),
        b"real-s3 chunk bytes"
    );
    put_verified(
        &store,
        &prefix,
        1,
        ObjectKind::Chunk,
        b"real-s3 chunk bytes",
    )
    .expect("idempotent re-put");
    // Definite absence is Absent, not an error.
    assert!(matches!(
        store
            .get_object(&format!("{prefix}/objects/1/chunk/{}", "0".repeat(64)))
            .expect("read"),
        ObjectRead::Absent
    ));
    // Listing enumerates the actual extant name.
    let page = store
        .list_objects(&format!("{prefix}/objects/"), None)
        .expect("list");
    assert_eq!(page.keys.len(), 1, "{:?}", page.keys);
    // A stale head version cannot win after the head moved (S3-01/02 tail).
    let head_key = format!("{prefix}/HEAD");
    let v1 = match store.create_head(&head_key, b"one").expect("create") {
        ConditionalOutcome::Published { version } => version,
        other => panic!("{other:?}"),
    };
    match store.replace_head(&head_key, &v1, b"two").expect("swap") {
        ConditionalOutcome::Published { .. } => {}
        other => panic!("{other:?}"),
    }
    assert_eq!(
        store
            .replace_head(&head_key, &v1, b"three")
            .expect("stale swap"),
        ConditionalOutcome::PreconditionFailed,
        "ETags are opaque exact tokens; a stale one definitively loses"
    );
    cleanup(&store, &prefix);
}
