//! The one literal-value sum.
//! Query literals (the engine IR's `Term::Literal`) and statement
//! selection literals ([`crate::schema::Side::selection`]) are the same
//! type — this module is the shared value home both the IR and
//! `schema` import, so neither layer owes the other anything.
//! denotation: interval variants carry the checked [`crate::Interval`] type,
//! and [`Value::Uuid`] carries the application-owned identity bytes —
//! ordinary canonical data, never database-issued authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    U64(u64),
    I64(i64),
    F64(crate::F64),

    /// An application-owned 128-bit identity value: sixteen exact bytes,
    /// chosen once before a command seals and reused unchanged across
    /// retries. No reserved patterns, no issuance, no history authority.
    Uuid(crate::Uuid),

    String(Box<str>),

    FixedBytes(Box<[u8]>),

    /// ```compile_fail
    /// use bumbledb_theory::Value;
    /// let _ = Value::IntervalU64(7, 7);
    /// ```
    IntervalU64(crate::Interval<u64>),

    IntervalI64(crate::Interval<i64>),

    /// A checked dense-line interval: canonical binary64 endpoints,
    /// NaN-free, strictly ordered; `±Infinity` are unbounded endpoints.
    IntervalF64(crate::Interval<crate::F64>),
}

impl From<crate::Uuid> for Value {
    fn from(id: crate::Uuid) -> Self {
        Self::Uuid(id)
    }
}
