use crate::families::all;

pub(super) fn digest_over<'a>(items: impl Iterator<Item = (&'a str, String, &'a str)>) -> [u8; 32] {
    let mut digest = bumbledb::digest::Digest::new();
    for (name, query_debug, golden_sql) in items {
        digest.update(name.as_bytes());
        digest.update(query_debug.as_bytes());
        digest.update(golden_sql.as_bytes());
    }
    digest.finalize()
}

#[must_use]
pub fn digest() -> [u8; 32] {
    digest_over(
        all()
            .iter()
            .map(|f| (f.name, format!("{:?}", (f.query)()), f.golden_sql)),
    )
}
