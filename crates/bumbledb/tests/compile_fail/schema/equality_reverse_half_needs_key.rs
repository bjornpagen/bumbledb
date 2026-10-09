//! `==` is two containments: the reverse half needs a key on the left side.
//@ error: target relation Source (0) projection {a (0)} matches no declared key
//@ line: 11

bumbledb::schema! {
    pub InvalidEquality;
    relation Source { a: u64 }
    relation Target { x: u64 }

    Target(x) -> Target;
    Source(a) == Target(x);
}
