//! A newtype names one encoding, and `interval<u64, 7>` is not `interval<u64>`.
//@ error: newtype `Week` is declared over two encodings
//@ line: 9

bumbledb::schema! {
    pub Calendar;

    relation Sprint { span: interval<u64, 7> as Week }
    relation Leave  { span: interval<u64> as Week }
}
