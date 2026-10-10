//! Synchronous query validation against a compiled schema: the engine's
//! validator is the only one, and its refusal crosses as a diagnostic.
use std::sync::Arc;

use bumbledb::{Query, ValidationError};
use napi::bindgen_prelude::External;
use napi_derive::napi;

use crate::input::query::QueryIn;
use crate::input::{Malformed, decode};
use crate::schema::SchemaHandle;

/// A query the engine validated against one compiled schema.
pub struct QueryHandle {
    pub(crate) query: Query,
}

/// The engine's refusal: `code` is its `ValidationError` variant, `refusal`
/// the cited position's specific refusal (for example `Field` with
/// `Unknown`, `Comparison` with `Unordered.Interval`), plus whichever
/// rule-local coordinates it cites. `comparison` indexes the rule's
/// flattened comparisons.
#[napi(object, object_from_js = false)]
#[derive(Debug, Default)]
pub struct QueryDiagnostic {
    pub code: String,
    pub refusal: Option<String>,
    pub message: String,
    pub rule: Option<u32>,
    pub atom: Option<u32>,
    pub find: Option<u32>,
    pub field: Option<u32>,
    pub relation: Option<u32>,
    pub var: Option<u32>,
    pub param: Option<u32>,
    pub comparison: Option<u32>,
    pub interior: Option<u32>,
}

fn at(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// A refusal's derived `Debug` rendering, `Outer(Inner)` spelled
/// `Outer.Inner`.
fn refusal(value: &impl std::fmt::Debug) -> String {
    format!("{value:?}").replace('(', ".").replace(')', "")
}

pub(crate) fn query_diagnostic(error: &ValidationError) -> QueryDiagnostic {
    use ValidationError as E;
    let mut out = QueryDiagnostic {
        code: crate::schema::code(error),
        message: error.to_string(),
        ..QueryDiagnostic::default()
    };
    match error {
        E::EmptyRuleSet
        | E::TooMany { .. }
        | E::DnfExceedsRules { .. }
        | E::CountAcrossRules { .. }
        | E::EmptyFinds
        | E::NoPositiveAtoms
        | E::Unplannable => {}
        E::Rec(why) => out.refusal = Some(refusal(why)),
        E::ScalarExpression { find, .. } => {
            out.find = Some(at(find.0));
        }
        E::Aggregate { find, refusal: why } => {
            out.find = Some(at(find.0));
            out.refusal = Some(refusal(why));
        }
        E::ConditionNestingTooDeep { rule, .. } | E::HeadArityMismatch { rule, .. } => {
            out.rule = Some(at(rule.0));
        }
        E::Head {
            rule,
            position,
            mismatch,
        } => {
            out.rule = Some(at(rule.0));
            out.find = Some(at(position.0));
            out.refusal = Some(refusal(mismatch));
        }
        E::UnknownRelation { atom, relation } => {
            out.atom = Some(at(atom.0));
            out.relation = Some(relation.0);
        }
        E::UnknownInterior { atom, interior } => {
            out.atom = Some(at(atom.0));
            out.interior = Some(interior.0);
        }
        E::Field {
            atom,
            field,
            refusal: why,
        } => {
            out.atom = Some(at(atom.0));
            out.field = Some(u32::from(field.0));
            out.refusal = Some(refusal(why));
        }
        E::Variable { var, refusal: why } => {
            out.var = Some(u32::from(var.0));
            out.refusal = Some(refusal(why));
        }
        E::Param {
            param,
            refusal: why,
        } => {
            out.param = Some(u32::from(param.0));
            out.refusal = Some(refusal(why));
        }
        E::Comparison {
            index,
            refusal: why,
        } => {
            out.comparison = Some(at(*index));
            out.refusal = Some(refusal(why));
        }
        E::DuplicateFindTerm { index } => out.find = Some(at(*index)),
        E::EmptyInterior { interior }
        | E::InteriorNotPrior { interior, .. }
        | E::AggregateInInterior { interior } => out.interior = Some(interior.0),
    }
    out
}

#[napi(discriminant = "_tag", object_from_js = false)]
pub enum QueryValidated {
    Valid { query: External<Arc<QueryHandle>> },
    Invalid { diagnostic: QueryDiagnostic },
    Malformed { path: String, message: String },
}

/// Validate one query IR (JSON) against a compiled schema.
#[napi]
#[must_use]
pub fn validate_query(schema: &External<Arc<SchemaHandle>>, ir_json: String) -> QueryValidated {
    let query: Query = match decode::<QueryIn>(&ir_json) {
        Ok(query) => query.into(),
        Err(Malformed { path, message }) => return QueryValidated::Malformed { path, message },
    };
    match bumbledb::ir::validate::validate(&schema.schema, &query) {
        Ok(_) => QueryValidated::Valid {
            query: External::new(Arc::new(QueryHandle { query })),
        },
        Err(error) => QueryValidated::Invalid {
            diagnostic: query_diagnostic(&error),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{SchemaCompiled, compile_schema};

    fn schema() -> External<Arc<SchemaHandle>> {
        let SchemaCompiled::Compiled { schema, .. } = compile_schema(
            r#"{"relations":[{"name":"Item","fields":[
                {"name":"a","valueType":{"kind":"U64"}},
                {"name":"b","valueType":{"kind":"String"}}]}],"statements":[]}"#
                .into(),
        ) else {
            panic!("schema compiles")
        };
        schema
    }

    fn validate(rule: &str) -> QueryValidated {
        validate_query(
            &schema(),
            format!(r#"{{"interiors":[],"head":[{{"kind":"Var"}}],"rules":[{rule}]}}"#),
        )
    }

    #[test]
    fn a_valid_query_yields_a_handle() {
        assert!(matches!(
            validate(
                r#"{"finds":[{"kind":"Var","var":0}],
                    "atoms":[{"source":{"kind":"Edb","relation":0},
                              "bindings":[{"field":0,"term":{"kind":"Var","var":0}}]}],
                    "negated":[],"conditions":[]}"#
            ),
            QueryValidated::Valid { .. }
        ));
    }

    #[test]
    fn engine_refusals_cite_their_coordinates() {
        let QueryValidated::Invalid { diagnostic } = validate(
            r#"{"finds":[{"kind":"Var","var":0}],
                "atoms":[{"source":{"kind":"Edb","relation":0},
                          "bindings":[{"field":7,"term":{"kind":"Var","var":0}}]}],
                "negated":[],"conditions":[]}"#,
        ) else {
            panic!("unknown field refuses")
        };
        assert_eq!(diagnostic.code, "Field");
        assert_eq!(diagnostic.refusal.as_deref(), Some("Unknown"));
        assert_eq!((diagnostic.atom, diagnostic.field), (Some(0), Some(7)));

        let QueryValidated::Invalid { diagnostic } = validate(
            r#"{"finds":[{"kind":"Var","var":0}],
                "atoms":[{"source":{"kind":"Edb","relation":0},
                          "bindings":[{"field":0,"term":{"kind":"Var","var":0}},
                                      {"field":1,"term":{"kind":"Var","var":0}}]}],
                "negated":[],"conditions":[]}"#,
        ) else {
            panic!("conflicting variable types refuse")
        };
        assert_eq!(diagnostic.code, "Variable");
        assert_eq!(diagnostic.refusal.as_deref(), Some("TypeConflict"));
        assert_eq!(diagnostic.var, Some(0));
    }

    #[test]
    fn malformed_ir_cites_its_path() {
        let QueryValidated::Malformed { path, .. } =
            validate(r#"{"finds":[],"atoms":[],"negated":[]}"#)
        else {
            panic!("missing conditions refuse")
        };
        assert_eq!(path, "rules[0]");
    }
}
