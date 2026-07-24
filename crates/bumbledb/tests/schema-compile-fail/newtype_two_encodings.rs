//! One newtype name = one encoding (`docs/architecture/70-api.md` § the
//! `schema!` grammar — rustc polices domains): the duplicate check keys
//! on the DECLARED encoding, not the rendered Rust type, which is lossy
//! exactly where the width is the type — `interval<u64, 7>` and
//! `interval<u64>` share one host `Interval<u64>`, and a label spanning
//! both would void the compile-time half of the nominal-safety promise.
//@ error: newtype `Week` declared twice with different encodings: interval<u64, 7> vs interval<u64>

bumbledb::schema! {
    pub Calendar;

    relation Sprint { span: interval<u64, 7> as Week }
    relation Leave  { span: interval<u64> as Week }
}
