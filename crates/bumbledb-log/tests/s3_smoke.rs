//! Credential-gated REAL S3 cells (D17/G08). Emulator green is not
//! qualification. Missing credentials is `NotRun`, never a waived pass.
//! Required env: `BUMBLEDB_S3_SMOKE_BUCKET`, `AWS_ACCESS_KEY_ID`,
//! `AWS_SECRET_ACCESS_KEY`. Optional: `BUMBLEDB_S3_SMOKE_REGION`,
//! `BUMBLEDB_S3_SMOKE_ENDPOINT`, `BUMBLEDB_S3_DENIED_PREFIX`,
//! `BUMBLEDB_S3_WRONG_REGION`. Verification: `NotRun`.

#![cfg(feature = "store")]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

use bumbledb_log::store::s3::{S3Config, S3Credentials, S3Store, StaticKeys};
use bumbledb::{ExecutionPolicy, WorkContext};
use bumbledb_log::store::{
    ConditionalOutcome, ConditionalStore as _, ObjectKind, ReceiveLimits, ReceivedHead,
    ReceivingStore, TransportContext, TransportObservation, get_verified, put_verified,
};

const REQUIRED: [&str; 3] = [
    "BUMBLEDB_S3_SMOKE_BUCKET",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
];

fn credentials_available() -> bool {
    REQUIRED.iter().all(|name| std::env::var(name).is_ok())
}

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

fn static_credentials() -> S3Credentials {
    S3Credentials::Static {
        access_key_id: std::env::var("AWS_ACCESS_KEY_ID").expect("key env"),
        secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY").expect("secret env"),
        session_token: std::env::var("AWS_SESSION_TOKEN").ok(),
    }
}

fn store_with(credentials: S3Credentials, region: Option<String>) -> S3Store {
    S3Store::new(&S3Config {
        endpoint: std::env::var("BUMBLEDB_S3_SMOKE_ENDPOINT").ok(),
        region: region
            .or_else(|| std::env::var("BUMBLEDB_S3_SMOKE_REGION").ok())
            .unwrap_or_else(|| "us-east-1".into()),
        bucket: std::env::var("BUMBLEDB_S3_SMOKE_BUCKET").expect("bucket env"),
        credentials,
    })
    .expect("S3 store constructs")
}

fn store() -> S3Store {
    store_with(static_credentials(), None)
}

fn work() -> WorkContext {
    ExecutionPolicy {
        input_bytes: 0,
        working_bytes: 1 << 20,
        scratch_bytes: 0,
        result_bytes: 0,
        rows: 0,
        work_units: 1_024,
        timeout: std::time::Duration::from_secs(30),
    }
    .start()
    .expect("work")
}

fn transport(work: &WorkContext) -> TransportContext<'_> {
    TransportContext::new(work, ReceiveLimits::capped(1 << 20))
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
fn s3_conditional_create_replace_and_lost_ack_are_typed_outcomes() {
    require_credentials!();
    let store = store();
    let prefix = fresh_prefix("cas");
    let head_key = format!("{prefix}/HEAD");
    let v1 = match store.create_head(&head_key, b"rev-1").expect("create") {
        ConditionalOutcome::Published { version } => version,
        other => panic!("first create publishes: {other:?}"),
    };
    let second = store.create_head(&head_key, b"rev-1-imposter").expect("second");
    assert!(
        matches!(
            second,
            ConditionalOutcome::PreconditionFailed | ConditionalOutcome::Indeterminate
        ),
        "a never-reused head is never a second publication: {second:?}"
    );
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
    assert!(wins <= 1, "at most one publication per observed version");
    match store.receive_head(
        &head_key,
        TransportContext {
            work: None,
            receive: ReceiveLimits::capped(64),
        },
    ) {
        Ok(ReceivedHead::Present { body, .. }) => {
            assert!(
                body.as_bytes() == b"rev-2-from-a"
                    || body.as_bytes() == b"rev-2-from-b"
                    || body.as_bytes() == b"rev-1",
                "resolution is by reading, never a manufactured winner"
            );
        }
        Ok(ReceivedHead::Absent) => panic!("the head exists"),
        Err(error) => panic!("receive_head observation: {:?}", error.observation),
    }
    cleanup(&store, &prefix);
}

#[test]
fn s3_receive_caps_missing_and_immutable_identity() {
    require_credentials!();
    let store = store();
    let prefix = fresh_prefix("receive");
    let reference = put_verified(
        &store,
        &prefix,
        1,
        ObjectKind::Chunk,
        b"real-s3 chunk bytes",
    )
    .expect("put");
    let ctx = work();
    assert_eq!(
        get_verified(&store, &prefix, &reference, transport(&ctx))
            .expect("verified")
            .as_bytes(),
        b"real-s3 chunk bytes"
    );
    put_verified(
        &store,
        &prefix,
        1,
        ObjectKind::Chunk,
        b"real-s3 chunk bytes",
    )
    .expect("identical bytes are idempotent");
    let conflict = put_verified(&store, &prefix, 1, ObjectKind::Chunk, b"different-bytes");
    assert!(
        conflict.is_err(),
        "immutable different bytes refuse: {conflict:?}"
    );
    let capped = store
        .receive_object(
            &reference.key(&prefix),
            TransportContext {
                work: None,
                receive: ReceiveLimits::capped(4),
            },
        )
        .expect_err("cap");
    assert_eq!(capped.observation, TransportObservation::Capped);
    let missing = store
        .receive_object(
            &format!("{prefix}/objects/1/chunk/{}", "0".repeat(64)),
            TransportContext {
                work: None,
                receive: ReceiveLimits::capped(32),
            },
        )
        .expect_err("missing");
    assert_eq!(missing.observation, TransportObservation::Missing);
    let page = store
        .list_objects(&format!("{prefix}/objects/"), None)
        .expect("list");
    assert_eq!(page.keys.len(), 1, "{:?}", page.keys);
    assert!(
        page.next.is_none(),
        "progress is the last canonical key; a short page has none"
    );
    cleanup(&store, &prefix);
}

#[test]
fn s3_listing_progress_is_the_last_canonical_key() {
    require_credentials!();
    let store = store();
    let prefix = fresh_prefix("list");
    for name in ["aa", "bb", "cc"] {
        store
            .put_object(&format!("{prefix}/objects/1/chunk/{name}"), b"x")
            .expect("put");
    }
    let first = store
        .list_objects(&format!("{prefix}/objects/"), None)
        .expect("list");
    assert!(first.keys.len() >= 3, "{:?}", first.keys);
    let after = first.keys[0].as_bytes();
    let rest = store
        .list_objects(&format!("{prefix}/objects/"), Some(after))
        .expect("resume");
    assert!(
        !rest.keys.contains(&first.keys[0]),
        "after is exclusive last processed key"
    );
    assert!(rest.keys.windows(2).all(|pair| pair[0] < pair[1]));
    cleanup(&store, &prefix);
}

#[test]
fn s3_credential_refresh_is_consulted_per_request() {
    require_credentials!();
    let calls = Arc::new(AtomicU64::new(0));
    let refresh = {
        let calls = Arc::clone(&calls);
        Arc::new(move || {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(StaticKeys {
                access_key_id: std::env::var("AWS_ACCESS_KEY_ID").expect("key"),
                secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY").expect("secret"),
                session_token: std::env::var("AWS_SESSION_TOKEN").ok(),
            })
        })
    };
    let store = store_with(S3Credentials::Refresh(refresh), None);
    let prefix = fresh_prefix("refresh");
    let _ = store.receive_object(
        &format!("{prefix}/objects/1/chunk/aa"),
        TransportContext {
            work: None,
            receive: ReceiveLimits::capped(16),
        },
    );
    let _ = store.list_objects(&format!("{prefix}/objects/"), None);
    assert!(
        calls.load(Ordering::SeqCst) >= 2,
        "one shared provider refreshes per signed request"
    );
    cleanup(&store, &prefix);
}

#[test]
fn s3_denied_and_region_cells_are_notrun_without_their_env() {
    require_credentials!();
    let store = store();
    if let Ok(denied_prefix) = std::env::var("BUMBLEDB_S3_DENIED_PREFIX") {
        let error = store
            .receive_object(
                &format!("{denied_prefix}/HEAD"),
                TransportContext {
                    work: None,
                    receive: ReceiveLimits::capped(32),
                },
            )
            .expect_err("denied");
        assert_eq!(
            error.observation,
            TransportObservation::Denied,
            "typed 403 is Denied, never a publication verdict"
        );
    } else {
        eprintln!("SKIP (NotRun, not passed): denied cell needs BUMBLEDB_S3_DENIED_PREFIX");
    }
    if let Ok(wrong_region) = std::env::var("BUMBLEDB_S3_WRONG_REGION") {
        let foreign = store_with(static_credentials(), Some(wrong_region));
        let error = foreign
            .receive_object(
                "t/objects/1/chunk/aa",
                TransportContext {
                    work: None,
                    receive: ReceiveLimits::capped(16),
                },
            )
            .expect_err("region");
        assert!(
            matches!(
                error.observation,
                TransportObservation::Region
                    | TransportObservation::Indeterminate
                    | TransportObservation::Denied
            ),
            "wrong region is not Missing and not a guessed publication: {:?}",
            error.observation
        );
    } else {
        eprintln!("SKIP (NotRun, not passed): region cell needs BUMBLEDB_S3_WRONG_REGION");
    }
}
