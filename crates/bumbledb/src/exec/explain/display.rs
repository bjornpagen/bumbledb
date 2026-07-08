use super::Report;
use std::fmt;

impl fmt::Display for Report<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GuardProbe { plan } => {
                writeln!(f, "access path: guard probe")?;
                writeln!(f, "  relation: {}", plan.relation.0)?;
                match plan.constraint {
                    Some(c) => writeln!(f, "  unique constraint: {}", c.0)?,
                    None => writeln!(f, "  full-fact membership probe")?,
                }
                writeln!(
                    f,
                    "  key fields: {:?}",
                    plan.key
                        .iter()
                        .map(|(field, _)| field.0)
                        .collect::<Vec<_>>()
                )?;
                writeln!(f, "  remaining filters: {}", plan.remaining_filters.len())?;
                Ok(())
            }
            Self::FreeJoin { plan, stats } => {
                writeln!(f, "access path: free join ({} nodes)", plan.nodes().len())?;
                for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
                    writeln!(
                        f,
                        "  occurrence {occ_idx}: relation {} trie schema {:?} ({} filters)",
                        occurrence.relation.0,
                        occurrence
                            .trie_schema
                            .iter()
                            .map(|level| level.iter().map(|v| v.0).collect::<Vec<_>>())
                            .collect::<Vec<_>>(),
                        occurrence.filters.len(),
                    )?;
                }
                for (node_idx, node) in plan.nodes().iter().enumerate() {
                    let node_stats = &stats.nodes[node_idx];
                    writeln!(f, "  node {node_idx}:")?;
                    for (sub_idx, subatom) in node.subatoms.iter().enumerate() {
                        let cover = &node_stats.covers[sub_idx];
                        writeln!(
                            f,
                            "    subatom {sub_idx}: occ {} vars {:?} cover({}) chosen \
                             exact={} estimate={} probes hit={} miss={}",
                            subatom.occ.0,
                            subatom.vars.iter().map(|v| v.0).collect::<Vec<_>>(),
                            node.covers.contains(
                                &u8::try_from(sub_idx).expect("subatoms per node fit u8")
                            ),
                            cover.chosen_exact,
                            cover.chosen_estimate,
                            cover.probes_hit,
                            cover.probes_miss,
                        )?;
                    }
                    writeln!(
                        f,
                        "    residuals: {} placed, pass={} fail={}",
                        node.residuals.len(),
                        node_stats.residual_pass,
                        node_stats.residual_fail,
                    )?;
                    writeln!(
                        f,
                        "    estimated={} actual={} entries={} skips={}",
                        node_stats.estimate,
                        node_stats.actual,
                        node_stats.entries,
                        node_stats.skips,
                    )?;
                }
                writeln!(f, "  emitted bindings: {}", stats.emits)?;
                Ok(())
            }
        }
    }
}
