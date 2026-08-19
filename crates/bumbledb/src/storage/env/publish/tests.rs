use super::{PublishCatalog, PublishStep};
use crate::error::Error;
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use crate::storage::catalog::FrozenCatalog;
use crate::storage::env::{Environment, FORMAT_VERSION, StoreKind};
use crate::testutil::TempDir;
use bumbledb_theory::schema::{
    FieldDescriptor, Generation, RelationDescriptor, SchemaDescriptor, ValueType,
};

fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "R".into(),
            fields: vec![FieldDescriptor {
                name: "x".into(),
                value_type: ValueType::U64,
                generation: Generation::Fresh,
            }],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

#[test]
fn publish_step_all_is_the_protocol() {
    assert_eq!(
        PublishStep::ALL,
        [
            PublishStep::CreateStaging,
            PublishStep::WriteCatalog,
            PublishStep::CommitAndClose,
            PublishStep::SyncStagingFiles,
            PublishStep::Rename,
            PublishStep::SyncParent,
        ]
    );
}

#[test]
fn every_prefix_before_rename_hides_the_destination() {
    let schema = schema();
    let empty = FrozenCatalog::empty();
    for step in PublishStep::ALL
        .into_iter()
        .filter(|step| step.before_rename())
    {
        let dir = TempDir::new(&format!("publish-prefix-{step:?}"));
        let dest = dir.path().join("store");
        let prefix = Environment::publish_until(
            &dest,
            StoreKind::Durable,
            &PublishCatalog::frozen(&empty, &schema),
            step,
        )
        .expect("prefix");
        assert!(!dest.exists(), "{step:?} must not expose dest");
        assert!(!prefix.dest_exists);
        assert!(
            prefix.staging.exists(),
            "{step:?} leaves a staging directory"
        );
        assert!(
            prefix
                .staging
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains(".staging.")),
            "staging name pattern: {:?}",
            prefix.staging
        );
    }
}

#[test]
fn prefix_at_or_after_rename_is_openable_format_8() {
    let schema = schema();
    let empty = FrozenCatalog::empty();
    for step in [PublishStep::Rename, PublishStep::SyncParent] {
        let dir = TempDir::new(&format!("publish-renamed-{step:?}"));
        let dest = dir.path().join("store");
        let prefix = Environment::publish_until(
            &dest,
            StoreKind::Durable,
            &PublishCatalog::frozen(&empty, &schema),
            step,
        )
        .expect("prefix");
        assert!(dest.exists(), "{step:?} exposes dest");
        assert!(prefix.dest_exists);
        let env = Environment::open(&dest, &schema).expect("openable format 8");
        let meta = env.read_store_meta().expect("parse_meta");
        assert_eq!(meta.version.word(), FORMAT_VERSION);
        assert_eq!(meta.kind, StoreKind::Durable);
        assert_eq!(meta.generation.value(), 0);
        drop(env);
        drop(prefix);
    }
}

#[test]
fn post_rename_sync_failure_is_published_but_unsynced() {
    let schema = schema();
    let empty = FrozenCatalog::empty();
    let dir = TempDir::new("publish-unsynced");
    let dest = dir.path().join("store");
    let err = Environment::publish_failing_parent_sync(
        &dest,
        StoreKind::Durable,
        &PublishCatalog::frozen(&empty, &schema),
    )
    .expect_err("injected parent-sync failure");
    assert!(matches!(err, Error::PublishedButUnsynced { .. }), "{err:?}");
    assert!(dest.exists(), "never removes the visible destination");
    Environment::open(&dest, &schema).expect("complete format-8 store");
}

#[test]
fn parse_meta_reads_six_keys() {
    use crate::storage::env::MetaKey;
    assert_eq!(MetaKey::PARSE_ORDER.len(), 6);
    let schema = schema();
    let dir = TempDir::new("publish-parse-meta");
    let env = Environment::create(dir.path(), &schema).expect("create");
    let meta = env.read_store_meta().expect("parse");
    assert_eq!(meta.version.word(), 8);
    assert_eq!(meta.kind, StoreKind::Durable);
    assert_eq!(meta.generation.value(), 0);
    assert_eq!(meta.dict_next.raw(), 0);
}
