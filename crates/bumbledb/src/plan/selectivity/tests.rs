//! The distinct ladder and occurrence pricing, over the C05 source seam:
//! key-exact (rung 0), schema containment bounds (rung 2), documented
//! floors (rung 3), plus the interior-occurrence planning floors. The
//! image-exact rung (1) needs a peekable store-generation memo — heap
//! fixtures never memoize — so its coverage lives with the prepared-query
//! store suites (`api/prepared/tests/selection.rs`).
use super::{
    ACCUMULATED_PLANNING_ROWS, DEFAULT_EQ_DISTINCT, DELTA_PLANNING_ROWS, occurrence_stats_on,
    relation_rows_on,
};
use crate::image::SourceImages;
use crate::image::cache::ImageCache;
use crate::image::testsupport::TestSource;
use crate::image::view::{Const, FilterPredicate};
use crate::ir::normalize::{OccBind, OccId, Occurrence, Role};
use crate::ir::{Value, WordCmp};
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, RelationDescriptor, RelationId, Row, SchemaDescriptor, Side,
    StatementDescriptor, ValueType,
};

const POSTING: RelationId = RelationId(0);
const ACCOUNT: RelationId = RelationId(1);
const STATUS: RelationId = RelationId(2);

fn schema() -> Schema {
    let field = |name: &str, value_type: ValueType| FieldDescriptor {
        name: name.into(),
        value_type,
    };
    let all = |relation: RelationId, projection: &[u16]| Side {
        relation,
        projection: projection.iter().map(|f| FieldId(*f)).collect(),
        selection: Box::new([]),
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Posting".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field("account", ValueType::U64),
                    field("flag", ValueType::Bool),
                    field("tag", ValueType::U64),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Account".into(),
                fields: vec![field("id", ValueType::U64)],
            },
            RelationDescriptor {
                extension: Some(Box::new([
                    Row {
                        handle: "Open".into(),
                        values: Box::new([]),
                    },
                    Row {
                        handle: "Frozen".into(),
                        values: Box::new([]),
                    },
                ])),
                name: "Status".into(),
                fields: vec![],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: POSTING,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Functionality {
                relation: ACCOUNT,
                projection: Box::new([FieldId(0)]),
            },
            // Posting.account ⊆ Account.id: the schema bound for the
            // account column's distincts when no image is resident.
            StatementDescriptor::Containment {
                source: all(POSTING, &[1]),
                target: all(ACCOUNT, &[0]),
            },
        ],
    }
    .validate()
    .expect("valid fixture")
}

fn fixture(schema: &Schema) -> TestSource {
    let postings: Vec<Vec<Value>> = (0..40u64)
        .map(|i| {
            vec![
                Value::U64(i),
                Value::U64(i % 3),
                Value::Bool(i % 2 == 0),
                Value::U64(i % 7),
            ]
        })
        .collect();
    let accounts: Vec<Vec<Value>> = (0..3u64).map(|i| vec![Value::U64(i)]).collect();
    TestSource::new(schema, &[(POSTING, postings), (ACCOUNT, accounts)])
}

fn occurrence(vars: &[(u16, u16)], filters: Vec<FilterPredicate>) -> Occurrence {
    Occurrence {
        occ_id: OccId(0),
        bind: OccBind::Edb(POSTING),
        role: Role::Positive,
        vars: vars
            .iter()
            .map(|(f, v)| (FieldId(*f), crate::ir::VarId(*v)))
            .collect(),
        filters,
        point_vars: vec![],
    }
}

fn distinct_of_var(stats: &crate::plan::planner::OccStats, var: u16) -> u64 {
    stats
        .var_distincts
        .iter()
        .find(|(v, _)| *v == crate::ir::VarId(var))
        .expect("var is bound")
        .1
}

#[test]
fn relation_rows_counts_the_source_and_prices_closed_extensions() {
    let schema = schema();
    let fixture = fixture(&schema);
    let source = fixture.source();
    assert_eq!(
        relation_rows_on(&source, &schema, POSTING).expect("rows"),
        40
    );
    assert_eq!(
        relation_rows_on(&source, &schema, ACCOUNT).expect("rows"),
        3
    );
    assert_eq!(
        relation_rows_on(&source, &schema, STATUS).expect("rows"),
        2,
        "a closed relation's rows are its sealed extension"
    );
}

#[test]
fn the_cold_ladder_prices_keys_bounds_and_floors() {
    let schema = schema();
    let fixture = fixture(&schema);
    let source = fixture.source();
    let cache = ImageCache::new(&schema);
    let images = SourceImages::bind(&source, &cache);
    let rows = relation_rows_on(&source, &schema, POSTING).expect("rows");

    let occ = occurrence(&[(0, 0), (1, 1), (2, 2), (3, 3)], vec![]);
    let stats = occurrence_stats_on(&images, &schema, &occ, rows).expect("stats");
    assert_eq!(stats.rows, 40, "no filters keep every row");

    // Rung 0: the declared key's distincts are the row count, exactly.
    assert_eq!(distinct_of_var(&stats, 0), 40);
    // Rung 2: the containment bound caps account at |Account| = 3.
    assert_eq!(distinct_of_var(&stats, 1), 3);
    // Rung 3 floors: Bool is 2; an unkeyed unbounded scalar is the
    // documented constant.
    assert_eq!(distinct_of_var(&stats, 2), 2);
    assert_eq!(distinct_of_var(&stats, 3), DEFAULT_EQ_DISTINCT);
}

#[test]
fn an_eq_selection_on_the_key_prices_a_point_lookup() {
    let schema = schema();
    let fixture = fixture(&schema);
    let source = fixture.source();
    let cache = ImageCache::new(&schema);
    let images = SourceImages::bind(&source, &cache);
    let rows = relation_rows_on(&source, &schema, POSTING).expect("rows");

    let occ = occurrence(
        &[(1, 0)],
        vec![FilterPredicate::Compare {
            field: FieldId(0).into(),
            op: WordCmp::Eq,
            value: Const::Word(7),
        }],
    );
    let stats = occurrence_stats_on(&images, &schema, &occ, rows).expect("stats");
    assert_eq!(stats.rows, 1, "rows / key-distincts = one row");
}

#[test]
fn a_bool_eq_selection_keeps_half_by_the_floor() {
    let schema = schema();
    let fixture = fixture(&schema);
    let source = fixture.source();
    let cache = ImageCache::new(&schema);
    let images = SourceImages::bind(&source, &cache);
    let rows = relation_rows_on(&source, &schema, POSTING).expect("rows");

    let occ = occurrence(
        &[(0, 0)],
        vec![FilterPredicate::Compare {
            field: FieldId(2).into(),
            op: WordCmp::Eq,
            value: Const::Byte(1),
        }],
    );
    let stats = occurrence_stats_on(&images, &schema, &occ, rows).expect("stats");
    assert_eq!(stats.rows, 20, "40 rows / 2 bool values");
}

#[test]
fn interior_occurrences_price_by_their_planning_floors() {
    let schema = schema();
    let fixture = fixture(&schema);
    let source = fixture.source();
    let cache = ImageCache::new(&schema);
    let images = SourceImages::bind(&source, &cache);

    let mut delta = occurrence(&[(0, 0)], vec![]);
    delta.bind = OccBind::RecDelta(crate::ir::InteriorId(0));
    let stats = occurrence_stats_on(&images, &schema, &delta, 40).expect("stats");
    assert_eq!(stats.rows, DELTA_PLANNING_ROWS.max(1));
    assert_eq!(distinct_of_var(&stats, 0), DELTA_PLANNING_ROWS.max(1));

    let mut finished = occurrence(&[(0, 0)], vec![]);
    finished.bind = OccBind::Finished(crate::ir::InteriorId(0));
    let stats = occurrence_stats_on(&images, &schema, &finished, 40).expect("stats");
    assert_eq!(stats.rows, ACCUMULATED_PLANNING_ROWS);
}
