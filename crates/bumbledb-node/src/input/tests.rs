use bumbledb::{
    AtomSource, CmpOp, ConditionTree, F64, FindTerm, Interval, Query, RelationId, Term, Uuid,
    Value, VarId,
};

use super::options::RuntimeOptionsIn;
use super::query::QueryIn;
use super::schema::SchemaSpecIn;
use super::value::Literal;
use super::{Malformed, decode};

fn literal(json: &str) -> Result<Value, Malformed> {
    decode::<Literal>(json).map(|literal| literal.0)
}

#[test]
fn values_use_exact_text_encodings() {
    assert_eq!(
        literal(r#"{"kind":"U64","value":"18446744073709551615"}"#),
        Ok(Value::U64(u64::MAX))
    );
    assert_eq!(
        literal(r#"{"kind":"I64","value":"-9223372036854775808"}"#),
        Ok(Value::I64(i64::MIN))
    );
    assert_eq!(
        literal(r#"{"kind":"F64","value":"8000000000000000"}"#),
        Ok(Value::F64(F64::from(-0.0)))
    );
    assert_eq!(
        literal(r#"{"kind":"F64","value":"fff0000000000000"}"#),
        Ok(Value::F64(F64::from(f64::NEG_INFINITY)))
    );
    assert_eq!(
        literal(r#"{"kind":"FixedBytes","value":"00ff10"}"#),
        Ok(Value::FixedBytes(Box::new([0x00, 0xff, 0x10])))
    );
    assert_eq!(
        literal(r#"{"kind":"Uuid","value":"00000000-0000-0000-0000-000000000001"}"#),
        Ok(Value::Uuid(Uuid::from_u128(1)))
    );
    assert_eq!(
        literal(r#"{"kind":"IntervalU64","start":"3","end":"5"}"#),
        Ok(Value::IntervalU64(Interval::new(3, 5).unwrap()))
    );
}

#[test]
fn non_canonical_spellings_refuse() {
    for json in [
        r#"{"kind":"U64","value":"01"}"#,
        r#"{"kind":"U64","value":"-1"}"#,
        r#"{"kind":"U64","value":"18446744073709551616"}"#,
        r#"{"kind":"U64","value":5}"#,
        r#"{"kind":"I64","value":"-0"}"#,
        r#"{"kind":"I64","value":"+1"}"#,
        r#"{"kind":"F64","value":"3FF0000000000000"}"#,
        r#"{"kind":"F64","value":"3ff00000"}"#,
        r#"{"kind":"FixedBytes","value":"0"}"#,
        r#"{"kind":"FixedBytes","value":"AB"}"#,
        r#"{"kind":"Uuid","value":"00000000-0000-0000-0000-00000000000A"}"#,
        r#"{"kind":"IntervalU64","start":"5","end":"5"}"#,
    ] {
        let refused = literal(json).expect_err(json);
        assert!(
            refused.message.starts_with("expected") || refused.message.starts_with("invalid type"),
            "{json}: {refused:?}"
        );
    }
}

#[test]
fn unknown_and_missing_fields_refuse_at_their_path() {
    let refused = literal(r#"{"kind":"U64","value":"1","extra":true}"#).unwrap_err();
    assert!(refused.message.contains("extra"), "{refused:?}");
    let refused = decode::<QueryIn>(
        r#"{"interiors":[],"head":[{"kind":"Var"}],"rules":[{"finds":[{"kind":"Var","var":0}],
            "atoms":[{"source":{"kind":"Edb","relation":0},"bindings":[{"field":0}]}],
            "negated":[],"conditions":[]}]}"#,
    )
    .unwrap_err();
    assert_eq!(refused.path, "rules[0].atoms[0].bindings[0]");
    assert!(refused.message.contains("term"), "{refused:?}");
    let refused = decode::<RuntimeOptionsIn>(r#"{"workerz":2}"#).unwrap_err();
    assert!(refused.message.contains("workerz"), "{refused:?}");
}

#[test]
fn deep_nesting_and_lone_surrogates_refuse() {
    let mut deep = String::new();
    for _ in 0..200 {
        deep.push_str(r#"{"kind":"And","children":["#);
    }
    for _ in 0..200 {
        deep.push_str("]}");
    }
    let refused = decode::<super::query::ConditionIn>(&deep).unwrap_err();
    assert!(refused.message.contains("recursion"), "{refused:?}");
    assert!(literal(r#"{"kind":"String","value":"\ud800"}"#).is_err());
    assert_eq!(
        literal(r#"{"kind":"String","value":"🐝"}"#),
        Ok(Value::String("\u{1f41d}".into()))
    );
    assert!(decode::<RuntimeOptionsIn>("{} {}").is_err());
}

#[test]
fn query_ir_lowers_node_for_node() {
    let query: Query = decode::<QueryIn>(
        r#"{"interiors":[],"head":[{"kind":"Var"}],"rules":[{
            "finds":[{"kind":"Var","var":0}],
            "atoms":[{"source":{"kind":"Edb","relation":2},
                      "bindings":[{"field":1,"term":{"kind":"Var","var":0}}]}],
            "negated":[],
            "conditions":[{"kind":"Leaf","op":{"kind":"Allen","mask":1},
                           "lhs":{"kind":"Var","var":0},"rhs":{"kind":"Param","param":0}}]}]}"#,
    )
    .unwrap()
    .into();
    let rule = &query.rules[0];
    assert_eq!(rule.finds, vec![FindTerm::Var(VarId(0))]);
    assert_eq!(rule.atoms[0].source, AtomSource::Edb(RelationId(2)));
    assert_eq!(rule.atoms[0].bindings[0].1, Term::Var(VarId(0)));
    assert!(matches!(
        &rule.conditions[0],
        ConditionTree::Leaf(comparison) if matches!(comparison.op, CmpOp::Allen { .. })
    ));
    assert!(query.rec.is_none());
}

#[test]
fn recursive_stages_need_arms_and_masks_need_bits() {
    let refused =
        decode::<QueryIn>(r#"{"interiors":[],"head":[],"rules":[],"rec":{"base":[],"rec":[]}}"#)
            .unwrap_err();
    assert_eq!(refused.path, "rec.base");
    let refused = decode::<super::query::CmpOpIn>(r#"{"kind":"Allen","mask":16384}"#).unwrap_err();
    assert!(refused.message.contains("Allen"), "{refused:?}");
}

#[test]
fn a_refusal_inside_a_tagged_node_cites_the_enclosing_node() {
    let refused = decode::<QueryIn>(
        r#"{"interiors":[],"head":[{"kind":"Var"}],"rules":[{
            "finds":[{"kind":"Var","var":0}],
            "atoms":[{"source":{"kind":"Edb","relation":0},
                      "bindings":[{"field":0,"term":{"kind":"Var","var":0}}]}],
            "negated":[],
            "conditions":[{"kind":"Leaf","op":{"kind":"Eq"},"lhs":{"kind":"Var","var":0},
                           "rhs":{"kind":"Literal","value":{"kind":"U64","value":"007"}}}]}]}"#,
    )
    .unwrap_err();
    assert_eq!(refused.path, "rules[0].conditions[0]");
    assert!(
        refused.message.contains("canonical decimal u64"),
        "{refused:?}"
    );
}

#[test]
fn schema_spec_lowers_to_the_engine_spec() {
    let spec: bumbledb::SchemaSpec = decode::<SchemaSpecIn>(
        r#"{"relations":[
              {"name":"Kind","fields":[],"closed":{"newtype":"KindId","rows":[
                 {"handle":"A","values":[]}]}},
              {"name":"Item","fields":[
                 {"name":"id","valueType":{"kind":"U64"},"newtype":"ItemId"},
                 {"name":"kind","valueType":{"kind":"U64"},"newtype":"KindId"},
                 {"name":"span","valueType":{"kind":"FixedInterval","element":"I64","width":"7"}}]}],
            "statements":[
              {"kind":"Fd","relation":"Item","projection":["id"]},
              {"kind":"Containment","bidirectional":false,
               "source":{"relation":"Item","projection":["kind"],"selection":[]},
               "target":{"relation":"Kind","projection":["id"],"selection":[]}}]}"#,
    )
    .unwrap()
    .into();
    assert_eq!(spec.relations.len(), 2);
    assert_eq!(
        spec.relations[1].fields[0].newtype.as_deref(),
        Some("ItemId")
    );
    assert!(spec.relations[0].closed.is_some());
    assert!(spec.descriptor().is_ok());
}
