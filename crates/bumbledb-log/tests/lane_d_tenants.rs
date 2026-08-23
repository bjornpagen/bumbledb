//! Per-tenant replicas: the LRU, eviction as the disposable law, and
//! the pinned `_shared` tenant.

mod lane_d_support;

use bumbledb::Value;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::tenants::{
    SHARED_TENANT, Tenant, TenantOptions, TenantRefusal, Tenants, open_tenants,
};
use lane_d_support::{NOTE, TestLog, insert_note, note_braid, temp_dir, theory};

fn seeded_store(root: &std::path::Path, tenants: &[&str]) {
    for (index, tenant) in tenants.iter().enumerate() {
        let mut log = TestLog::new(root.to_path_buf(), &format!("t/{tenant}"));
        let notes = note_braid(&log.codec);
        let id = u64::try_from(index).expect("small index");
        log.publish(notes, &[insert_note(id, tenant)], 100 + id);
    }
}

fn ready(
    outcome: Tenant<'_, bumbledb::SchemaDescriptor, FsStore>,
) -> &mut bumbledb_log::replica::Replica<
    bumbledb::SchemaDescriptor,
    bumbledb_log::tenants::Shared<FsStore>,
> {
    match outcome {
        Tenant::Ready(replica) => replica,
        Tenant::Refused(refusal) => panic!("tenant refused: {refusal:?}"),
    }
}

fn lru(
    root: std::path::PathBuf,
    dir: &std::path::Path,
    options: TenantOptions,
) -> Tenants<bumbledb::SchemaDescriptor, FsStore> {
    open_tenants(FsStore::new(root), "", dir, theory(), options)
}

#[test]
fn tenants_open_lazily_and_serve_their_own_prefixes() {
    let root = temp_dir("ten_open");
    let local = temp_dir("ten_open_local");
    seeded_store(&root, &["acme", "bravo"]);
    let mut tenants = lru(
        root,
        &local,
        TenantOptions {
            budget_bytes: u64::MAX,
            max_open: 8,
        },
    );

    let acme = ready(tenants.tenant("acme").expect("acme"));
    assert!(
        acme.db()
            .read(|instance| instance
                .contains_dyn(NOTE, &[Value::U64(0), Value::String("acme".into())]))
            .expect("read")
    );
    let bravo = ready(tenants.tenant("bravo").expect("bravo"));
    assert!(
        bravo
            .db()
            .read(|instance| instance
                .contains_dyn(NOTE, &[Value::U64(1), Value::String("bravo".into())]))
            .expect("read")
    );
    assert_eq!(tenants.open_count(), 2);
}

#[test]
fn eviction_closes_and_deletes_the_least_recent_dir() {
    let root = temp_dir("ten_evict");
    let local = temp_dir("ten_evict_local");
    seeded_store(&root, &["acme", "bravo", "carol"]);
    let mut tenants = lru(
        root,
        &local,
        TenantOptions {
            budget_bytes: u64::MAX,
            max_open: 2,
        },
    );

    let _ = ready(tenants.tenant("acme").expect("acme"));
    let _ = ready(tenants.tenant("bravo").expect("bravo"));
    assert!(local.join("acme").exists());

    let _ = ready(tenants.tenant("carol").expect("carol"));
    assert_eq!(tenants.open_count(), 2);
    assert_eq!(tenants.open_ids(), vec!["bravo", "carol"]);
    assert!(
        !local.join("acme").exists(),
        "eviction deletes the directory — the disposable law"
    );

    // A re-open after eviction is an ordinary fresh pull.
    let _ = ready(tenants.tenant("acme").expect("acme again"));
    assert_eq!(tenants.open_ids(), vec!["carol", "acme"]);
}

#[test]
fn recency_updates_on_every_touch() {
    let root = temp_dir("ten_lru");
    let local = temp_dir("ten_lru_local");
    seeded_store(&root, &["acme", "bravo", "carol"]);
    let mut tenants = lru(
        root,
        &local,
        TenantOptions {
            budget_bytes: u64::MAX,
            max_open: 2,
        },
    );

    let _ = ready(tenants.tenant("acme").expect("acme"));
    let _ = ready(tenants.tenant("bravo").expect("bravo"));
    // Touching acme makes bravo the eviction candidate.
    let _ = ready(tenants.tenant("acme").expect("acme touch"));
    let _ = ready(tenants.tenant("carol").expect("carol"));
    assert_eq!(tenants.open_ids(), vec!["acme", "carol"]);
}

#[test]
fn the_shared_tenant_is_pinned() {
    let root = temp_dir("ten_shared");
    let local = temp_dir("ten_shared_local");
    seeded_store(&root, &[SHARED_TENANT, "acme", "bravo"]);
    let mut tenants = lru(
        root,
        &local,
        TenantOptions {
            budget_bytes: u64::MAX,
            max_open: 2,
        },
    );

    let _ = ready(tenants.tenant(SHARED_TENANT).expect("shared"));
    let _ = ready(tenants.tenant("acme").expect("acme"));
    let _ = ready(tenants.tenant("bravo").expect("bravo"));
    assert!(
        tenants.open_ids().contains(&SHARED_TENANT),
        "the control plane never evicts"
    );
    assert!(local.join(SHARED_TENANT).exists());

    // An explicit evict of the pinned tenant is a no-op too.
    tenants.evict(SHARED_TENANT).expect("evict");
    assert!(tenants.open_ids().contains(&SHARED_TENANT));
}

#[test]
fn the_byte_budget_evicts_like_the_count_budget() {
    let root = temp_dir("ten_bytes");
    let local = temp_dir("ten_bytes_local");
    seeded_store(&root, &["acme", "bravo"]);
    let mut tenants = lru(
        root,
        &local,
        TenantOptions {
            // One open store always exceeds a one-byte budget, so every
            // new tenant evicts every older unpinned one.
            budget_bytes: 1,
            max_open: 8,
        },
    );

    let _ = ready(tenants.tenant("acme").expect("acme"));
    let _ = ready(tenants.tenant("bravo").expect("bravo"));
    assert_eq!(tenants.open_ids(), vec!["bravo"]);
}

#[test]
fn tenant_ids_parse_at_the_boundary() {
    let root = temp_dir("ten_ids");
    let local = temp_dir("ten_ids_local");
    let mut tenants = lru(
        root,
        &local,
        TenantOptions {
            budget_bytes: u64::MAX,
            max_open: 2,
        },
    );
    for hostile in ["", "a/b", ".", ".."] {
        match tenants.tenant(hostile).expect("lookup") {
            Tenant::Refused(TenantRefusal::Id) => {}
            Tenant::Refused(other) => {
                panic!("hostile id {hostile:?} hit the wrong refusal: {other:?}")
            }
            Tenant::Ready(_) => panic!("hostile id {hostile:?} opened"),
        }
    }
}
