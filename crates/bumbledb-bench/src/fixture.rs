//! Shared benchmark query and test-fixture shorthands.
//!
//! Query builders are compiled for production benchmark families; filesystem
//! and literal helpers used only by unit tests are gated at the item boundary.

use bumbledb::schema::{FieldDescriptor, Generation, ValueType};
use bumbledb::{Term, VarId};

#[cfg(test)]
use bumbledb::schema::{FieldId, Side};
#[cfg(test)]
use bumbledb::{Atom, RelationId, Value};

pub(crate) fn var(id: u16) -> Term {
    Term::Var(VarId(id))
}

#[cfg(test)]
pub(crate) fn atom(relation: RelationId, bindings: &[(u16, Term)]) -> Atom {
    Atom {
        relation,
        bindings: bindings
            .iter()
            .map(|(field, term)| (FieldId(*field), term.clone()))
            .collect(),
    }
}

pub(crate) fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    }
}

pub(crate) fn fresh(name: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::Fresh,
    }
}

#[cfg(test)]
pub(crate) fn side(relation: RelationId, projection: &[u16], selection: &[(u16, Value)]) -> Side {
    Side {
        relation,
        projection: projection.iter().map(|field| FieldId(*field)).collect(),
        selection: selection
            .iter()
            .map(|(field, value)| (FieldId(*field), value.clone()))
            .collect(),
    }
}

#[cfg(test)]
pub(crate) fn string(text: &str) -> Value {
    Value::String(text.as_bytes().to_vec().into())
}

#[cfg(test)]
pub(crate) struct TempDir(std::path::PathBuf);

#[cfg(test)]
impl TempDir {
    pub(crate) fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("bumbledb-bench-{tag}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create test dir");
        Self(path)
    }

    pub(crate) fn path(&self) -> &std::path::Path {
        &self.0
    }
}

#[cfg(test)]
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
