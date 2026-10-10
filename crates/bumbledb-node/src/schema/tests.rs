use bumbledb::schema::{
    FieldDescriptor, FieldId, RelationDescriptor, Row, Side, StatementDescriptor, ValueType,
};
use bumbledb::{RelationId, Theory as _, Value};

use super::*;

bumbledb::schema! {
    pub Mini;
    relation Item { a: u64, b: u64 }
    Item(a) -> Item;
}

fn refusal(descriptor: SchemaDescriptor) -> SchemaDiagnostic {
    match SchemaHandle::seal(descriptor, Vec::new()) {
        Ok(_) => panic!("descriptor must refuse"),
        Err(diagnostic) => diagnostic,
    }
}

#[test]
fn a_duplicate_statement_cites_both_statements() {
    let mut descriptor = Mini.descriptor();
    descriptor.statements.push(descriptor.statements[0].clone());
    let SchemaDiagnostic::Schema {
        code,
        statement: Some(statement),
        conflict: Some(conflict),
        ..
    } = refusal(descriptor)
    else {
        panic!("statement diagnostic")
    };
    assert_eq!(code, "DuplicateFunctionality");
    assert_eq!(
        (statement.id, statement.spelling.as_str()),
        (1, "Item(a) -> Item")
    );
    assert_eq!(conflict.id, 0);
}

#[test]
fn closed_relations_cite_their_synthetic_identity_key() {
    let mut descriptor = Mini.descriptor();
    descriptor.relations.push(RelationDescriptor {
        name: "Kind".into(),
        fields: vec![FieldDescriptor {
            name: "code".into(),
            value_type: ValueType::U64,
        }],
        extension: Some(Box::new([Row {
            handle: "First".into(),
            values: Box::new([Value::U64(7)]),
        }])),
    });
    descriptor
        .statements
        .push(StatementDescriptor::Functionality {
            relation: RelationId(1),
            projection: Box::new([FieldId(0)]),
        });
    let SchemaDiagnostic::Schema {
        statement: Some(statement),
        conflict: Some(conflict),
        ..
    } = refusal(descriptor.clone())
    else {
        panic!("synthetic key diagnostic")
    };
    assert_eq!(
        (statement.id, statement.spelling.as_str()),
        (2, "Kind(id) -> Kind")
    );
    assert_eq!(conflict.id, 0);

    descriptor.statements[1] = StatementDescriptor::Containment {
        source: Side {
            relation: RelationId(0),
            projection: Box::new([FieldId(0)]),
            selection: Box::default(),
        },
        target: Side {
            relation: RelationId(1),
            projection: Box::new([FieldId(1)]),
            selection: Box::default(),
        },
    };
    let SchemaDiagnostic::Schema {
        statement: Some(statement),
        conflict: None,
        message,
        ..
    } = refusal(descriptor)
    else {
        panic!("closed target diagnostic")
    };
    assert_eq!(
        (statement.id, statement.spelling.as_str()),
        (2, "Item(a) <= Kind(code)")
    );
    assert!(
        message.contains("Kind (1) is addressed by its handle id only"),
        "{message}"
    );
}

const SPEC: &str = r#"{"relations":[
    {"name":"Kind","fields":[],"closed":{"newtype":"KindId","rows":[{"handle":"A","values":[]}]}},
    {"name":"Item","fields":[
        {"name":"id","valueType":{"kind":"U64"},"newtype":"ItemId"},
        {"name":"kind","valueType":{"kind":"U64"},"newtype":"KindId"}]}],
  "statements":[
    {"kind":"Fd","relation":"Item","projection":["id"]},
    {"kind":"Containment","bidirectional":false,
     "source":{"relation":"Item","projection":["kind"],"selection":[]},
     "target":{"relation":"Kind","projection":["id"],"selection":[]}}]}"#;

#[test]
fn a_spec_compiles_to_a_descriptor_with_newtypes_and_materialized_statements() {
    let SchemaCompiled::Compiled { schema, descriptor } = compile_schema(SPEC.into()) else {
        panic!("spec compiles")
    };
    assert_eq!(descriptor.fingerprint, schema.fingerprint);
    assert_eq!(descriptor.fingerprint.len(), 64);
    let kind = &descriptor.relations[0];
    assert_eq!(kind.fields[0].newtype.as_deref(), Some("KindId"));
    assert_eq!(kind.extension.as_ref().map(Vec::len), Some(1));
    let item = &descriptor.relations[1];
    assert_eq!(
        item.fields
            .iter()
            .map(|field| field.newtype.as_deref())
            .collect::<Vec<_>>(),
        [Some("ItemId"), Some("KindId")]
    );
    assert_eq!(descriptor.statements.len(), 3);
    assert!(matches!(
        descriptor.statements[0],
        StatementOut::Functionality {
            id: 0,
            relation: 0,
            ..
        }
    ));
    assert!(matches!(
        descriptor.statements[2],
        StatementOut::Containment { id: 2, .. }
    ));
    let SchemaBindings::Bindings { source } = schema_bindings(&schema) else {
        panic!("bindings emit")
    };
    assert!(source.contains("db.closed(\"Kind\""), "{source}");
}

#[test]
fn unresolved_names_cite_the_spec_statement() {
    let spec = SPEC.replace(r#""projection":["id"]}"#, r#""projection":["nope"]}"#);
    let SchemaCompiled::Invalid {
        diagnostic: SchemaDiagnostic::Spec { issues },
    } = compile_schema(spec)
    else {
        panic!("unknown field refuses")
    };
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, "UnknownField");
    assert_eq!(issues[0].statement, Some(0));
}

#[test]
fn malformed_specs_cite_their_path() {
    let spec = SPEC.replace(r#""name":"Item","#, r#""name":"Item","extra":1,"#);
    let SchemaCompiled::Malformed { path, message } = compile_schema(spec) else {
        panic!("unknown key refuses")
    };
    assert_eq!(path, "relations[1].extra");
    assert!(message.contains("extra"), "{message}");
}
