//! The one literal-value sum.
//! Query literals (the engine IR's `Term::Literal`) and statement
//! selection literals ([`crate::schema::Side::selection`]) are the same
//! type — this module is the zero-dependency home both the IR and
//! `schema` import, so neither layer owes the other anything.
//! denotation: interval variants carry the checked [`crate::Interval`] type,
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    U64(u64),
    I64(i64),
    F64(crate::F64),

    String(Box<str>),

    FixedBytes(Box<[u8]>),

    /// ```compile_fail
    /// use bumbledb_theory::Value;
    /// let _ = Value::IntervalU64(7, 7);
    /// ```
    IntervalU64(crate::Interval<u64>),

    IntervalI64(crate::Interval<i64>),
}
