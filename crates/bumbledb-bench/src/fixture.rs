use bumbledb::schema::{FieldDescriptor, ValueType};
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
        source: bumbledb::AtomSource::Edb(relation),
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
    }
}

#[cfg(test)]
pub(crate) fn side(relation: RelationId, projection: &[u16], selection: &[(u16, Value)]) -> Side {
    Side {
        relation,
        projection: projection.iter().map(|field| FieldId(*field)).collect(),
        selection: selection
            .iter()
            .map(|(field, value)| {
                (
                    FieldId(*field),
                    bumbledb::schema::LiteralSet::One(value.clone()),
                )
            })
            .collect(),
    }
}

#[cfg(test)]
pub(crate) fn string(text: &str) -> Value {
    Value::String(text.into())
}

/// A scratch directory under the system temp dir, unique to this process and
/// moment, removed on drop.
pub(crate) struct TempDir(std::path::PathBuf);

impl TempDir {
    pub(crate) fn new(tag: impl std::fmt::Display) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "bumbledb-bench-{tag}-{}-{nanos}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        Self(path)
    }

    pub(crate) fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl std::ops::Deref for TempDir {
    type Target = std::path::Path;

    fn deref(&self) -> &std::path::Path {
        &self.0
    }
}

impl AsRef<std::path::Path> for TempDir {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
