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
        let _ = writeln!(out, "## {}\n", family.name);
        let kind = match family.kind {
            Kind::Gate => "gate",
            Kind::Report => "report",
        };
        let _ = writeln!(out, "Kind: {kind}.\n");
        let _ = writeln!(out, "```text\n{:#?}\n```\n", (family.query)());
        let _ = writeln!(out, "```sql\n{}\n```\n", family.golden_sql);
        let _ = writeln!(out, "Params: {}\n", family.param_policy);
    }
    out
}
