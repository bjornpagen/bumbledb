use super::*;
use crate::error::ValidationError;
use crate::ir::{FindTerm, Term};
use crate::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, Schema, SchemaDescriptor,
};

mod accept;
mod reject;

/// The fixture schema:
/// Holder(id serial, name string);
/// Account(id serial, holder u64, status enum, validity interval<u64>);
/// Posting(id serial, account u64, amount i64, at i64, memo bytes,
///         flag bool, span interval<u64>).
fn schema() -> Schema {
    let field = |name: &str, ty: ValueType| FieldDescriptor {
        name: name.into(),
        value_type: ty,
        generation: Generation::None,
    };
    let serial = |name: &str| FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::Serial,
    };
    let interval_u64 = ValueType::Interval {
        element: IntervalElement::U64,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Holder".into(),
                fields: vec![serial("id"), field("name", ValueType::String)],
            },
            RelationDescriptor {
                name: "Account".into(),
                fields: vec![
                    serial("id"),
                    field("holder", ValueType::U64),
                    field(
                        "status",
                        ValueType::Enum {
                            variants: ["Active", "Closed"].iter().map(|v| Box::from(*v)).collect(),
                        },
                    ),
                    field("validity", interval_u64.clone()),
                ],
            },
            RelationDescriptor {
                name: "Posting".into(),
                fields: vec![
                    serial("id"),
                    field("account", ValueType::U64),
                    field("amount", ValueType::I64),
                    field("at", ValueType::I64),
                    field("memo", ValueType::Bytes),
                    field("flag", ValueType::Bool),
                    field("span", interval_u64),
                ],
            },
        ],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const HOLDER: RelationId = RelationId(0);
const ACCOUNT: RelationId = RelationId(1);
const POSTING: RelationId = RelationId(2);

/// Interval fields, by fixture position.
const VALIDITY: u16 = 3; // Account.validity
const SPAN: u16 = 6; // Posting.span

fn atom(relation: RelationId, bindings: Vec<(u16, Term)>) -> crate::ir::Atom {
    crate::ir::Atom {
        relation,
        bindings: bindings.into_iter().map(|(f, t)| (FieldId(f), t)).collect(),
    }
}

fn var(id: u16) -> Term {
    Term::Var(VarId(id))
}

fn simple(finds: Vec<FindTerm>, atoms: Vec<crate::ir::Atom>) -> Query {
    Query {
        finds,
        atoms,
        negated: vec![],
        predicates: vec![],
    }
}

fn expect_err(query: &Query) -> ValidationError {
    validate(&schema(), query).expect_err("must reject")
}
