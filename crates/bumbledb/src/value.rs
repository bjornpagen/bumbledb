//! The one literal-value sum (`docs/architecture/30-dependencies.md`:
//! dependencies and queries share one representation).
//!
//! Query literals ([`crate::ir::Term::Literal`]) and statement selection
//! literals ([`crate::schema::Side::selection`]) are the same type — this
//! module is the zero-dependency home both `ir` and `schema` import, so
//! neither layer owes the other anything.
//!
//! `Value` is dumb data everywhere: `start < end` for intervals and UTF-8
//! for strings are boundary rules — IR validation for query literals,
//! schema validation for selections — never constructor invariants.
//! Encoding lives in `encoding`, rendering in `schema::render`; nothing a
//! consumer owns lives here.

/// A literal value. Exactly one variant per data-model type — no universal
/// integer (U64 and I64 literals are exact-typed; out-of-range is
/// unrepresentable rather than truncated), and Bytes exists by construction
/// (the v5 hole, post-mortem §13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    U64(u64),
    I64(i64),
    /// Declaration-order ordinal; range-checked against the bound field's
    /// variant list at validation.
    Enum(u8),
    /// Raw UTF-8 bytes; interning is the engine's job (resolved to an
    /// intern id per execution — a dictionary miss means empty result).
    String(Box<[u8]>),
    /// Raw bytes; interning as above.
    Bytes(Box<[u8]>),
    /// A half-open `[start, end)` over U64 (`docs/architecture/20-query-ir.md`).
    /// Dumb data by decision: `start < end` is a validation-boundary rule,
    /// not a constructor invariant — hosts construct through the checked
    /// [`crate::Interval`] type.
    IntervalU64(u64, u64),
    /// A half-open `[start, end)` over I64; bounds as [`Value::IntervalU64`].
    IntervalI64(i64, i64),
    /// An Allen mask — the interval-pair relation as a value
    /// (`docs/architecture/10-data-model.md` § the mask value shape).
    /// Not a field type: it anchors nothing but the `Allen` comparison's
    /// mask position, and exists so the temporal relation is a bind-time
    /// argument like any other param. Carried as the checked
    /// [`crate::AllenMask`] (bits above the low 13 are unrepresentable);
    /// the vacuous ∅/full masks are query-boundary rules, exactly like
    /// `start < end`.
    AllenMask(crate::allen::AllenMask),
}
