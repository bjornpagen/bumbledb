use crate::families::{all, digest, Kind};
use crate::gen;

/// The human-readable versioned query list: IR + SQL + param policy per
/// family (emitted into the repo as QUERIES.md).
#[must_use]
pub fn render_queries_md() -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    out.push_str("# The read query families\n\n");
    let _ = writeln!(
        out,
        "Family-list digest: `{}`.\n",
        gen::digest_hex(&digest())
    );
    for family in all() {
        section(
            &mut out,
            family.name,
            family.kind,
            &(family.query)(),
            family.golden_sql,
            family.param_policy,
        );
    }
    out.push_str("# The calendar query families\n\n");
    let _ = writeln!(
        out,
        "Family-list digest: `{}`.\n",
        gen::digest_hex(&crate::calendar::families::digest())
    );
    for family in crate::calendar::families::all() {
        section(
            &mut out,
            family.name,
            family.kind,
            &(family.query)(),
            family.golden_sql,
            family.param_policy,
        );
    }
    out
}

fn section(
    out: &mut String,
    name: &str,
    kind: Kind,
    query: &bumbledb::Query,
    golden_sql: &str,
    param_policy: &str,
) {
    use std::fmt::Write as _;
    let _ = writeln!(out, "## {name}\n");
    let kind = match kind {
        Kind::Gate => "gate",
        Kind::Report => "report",
    };
    let _ = writeln!(out, "Kind: {kind}.\n");
    let _ = writeln!(out, "```text\n{query:#?}\n```\n");
    let _ = writeln!(out, "```sql\n{golden_sql}\n```\n");
    let _ = writeln!(out, "Params: {param_policy}\n");
}
