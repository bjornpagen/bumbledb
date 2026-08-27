//! Prints the generated identity table
//! ([`bumbledb_log::identities::emit`]) to stdout. The checked-in
//! copies live at `crates/bumbledb-log/conformance/v3/identities.json`
//! and `ts/crate/log-identities.json`; the census diffs a fresh
//! emission against both.

fn main() {
    print!("{}", bumbledb_log::identities::emit());
}
