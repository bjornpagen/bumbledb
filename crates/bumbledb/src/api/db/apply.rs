//! Parsed-row token shared by the collection applicator.

/// A parsed/encoded row ready to enter the delta, or a proven no-op skip
/// (delete of a never-interned string: the fact cannot exist).
pub(super) enum ApplyRow {
    Ready,
    Skip,
}
