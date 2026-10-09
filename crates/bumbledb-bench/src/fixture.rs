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

/// A seeded sweep's case count: `n`, or sixteen times `n` under
/// `BUMBLEDB_DEEP=1`. The extra cases extend the same seed sequence.
#[cfg(test)]
pub(crate) fn sweep(n: u64) -> u64 {
    if std::env::var_os("BUMBLEDB_DEEP").is_some_and(|deep| deep == "1") {
        n * 16
    } else {
        n
    }
}

/// A scratch path unique to this test process, so concurrent test runs never
/// share a store.
#[cfg(test)]
pub(crate) fn scratch_path(tag: impl std::fmt::Display) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("bumbledb-bench-{}-{tag}", std::process::id()))
}

#[cfg(test)]
pub(crate) struct TempDir(std::path::PathBuf);

#[cfg(test)]
impl TempDir {
    pub(crate) fn new(tag: &str) -> Self {
        // Process id and clock keep concurrent and wedged runs off one LMDB lock.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();

        let root = std::env::var_os("BUMBLEDB_SCRATCH_DIR")
            .map_or_else(std::env::temp_dir, std::path::PathBuf::from);
        let path = root.join(format!(
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

#[cfg(test)]
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
