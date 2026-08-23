//! Braid-derivation goldens: for every fixture schema, the component
//! map and the serial-at-statements, pinned as JSON and mirrored by
//! the TypeScript implementation. `BUMBLEDB_LOG_BLESS=1` rewrites.

#[path = "lane_a_support/mod.rs"]
mod support;

use bumbledb_log::braids::braids;
use serde_json::Value as Json;

fn rendered(schema: &str) -> Json {
    let descriptor = support::schema(schema);
    let derived = braids(&descriptor);
    let components: serde_json::Map<String, Json> = derived
        .components()
        .into_iter()
        .map(|(braid, relations)| {
            (
                braid.to_string(),
                Json::Array(
                    relations
                        .into_iter()
                        .map(|relation| Json::from(relation.0))
                        .collect(),
                ),
            )
        })
        .collect();
    serde_json::json!({
        "schema": schema,
        "braids": components,
        "serialAt": derived
            .serial_at()
            .iter()
            .map(|statement| Json::from(statement.0))
            .collect::<Vec<_>>(),
    })
}

#[test]
fn braid_goldens_match() {
    let dir = support::corpus_dir().join("braids");
    if support::bless() {
        std::fs::create_dir_all(&dir).expect("braids dir");
    }
    for schema in support::load_schemas().keys() {
        let golden = rendered(schema);
        let path = dir.join(format!("{schema}.json"));
        if support::bless() {
            let mut text = serde_json::to_string_pretty(&golden).expect("render");
            text.push('\n');
            std::fs::write(&path, text).expect("write golden");
        } else {
            let disk: Json =
                serde_json::from_str(&std::fs::read_to_string(&path).expect("golden present"))
                    .expect("golden parses");
            assert_eq!(disk, golden, "schema {schema}: braid golden pinned");
        }
    }
}

#[test]
fn braid_ids_render_and_parse() {
    let descriptor = support::schema("multi");
    let derived = braids(&descriptor);
    let braid = derived.parse(0).expect("component head");
    assert_eq!(braid.to_string().len(), 9);
    assert!(braid.to_string().starts_with('c'));
    assert_eq!(braid.raw(), 0);
    // A member that is not its component's smallest id is not a braid.
    assert!(derived.parse(1).is_none());
    // Closed relations belong to no braid.
    assert!(derived.parse(4).is_none());
    assert_eq!(derived.braid_of(bumbledb::RelationId(4)), None);
    // Membership: both component members share the head's id.
    assert_eq!(
        derived.braid_of(bumbledb::RelationId(1)),
        derived.braid_of(bumbledb::RelationId(0))
    );
}
