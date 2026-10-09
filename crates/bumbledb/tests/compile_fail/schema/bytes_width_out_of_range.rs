//! A bytes<N> width is 1..=64.
//@ error: `Blob.digest` is bytes<65>: a bytes<N> width is 1..=64
//@ line: 7

bumbledb::schema! {
    pub Store;
    relation Blob { digest: bytes<65> }
}
