use super::{Answers, ParamArg, PreparedQuery};

use crate::error::Result;
use crate::image::view::{Const, FilterPredicate};
use crate::storage::env::ReadTxn;

impl<S> PreparedQuery<S> {
    /// Executes the query and returns the answers alongside the rendered
    /// query header — K10's one remaining rendering door.
    ///
    /// # Errors
    ///
    /// As [`Self::execute`].
    pub(crate) fn introspect(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &crate::image::cache::ImageCache,
        params: &[ParamArg<'_>],
    ) -> Result<(Answers, String)> {
        let catalog = txn.catalog();
        let images = crate::image::LmdbSource::bind(txn, cache);
        let mut out = Answers::new();
        self.execute_on(txn.identity(), &catalog, &images, params, &mut out)?;
        let mut report = format!("query:\n{}\nsignature: {}\n", self.rendered, self.signature);
        if let Some(pending) = self.pending_literal_note() {
            report.push_str(&pending);
        }
        Ok((out, report))
    }

    /// The pending-literal explanation is derived from the mutable plan
    /// templates after execution: a hit has already latched to `Word` and
    /// disappears; a dictionary miss remains owned raw bytes here.
    fn pending_literal_note(&self) -> Option<String> {
        if self.latch.is_latched() {
            return None;
        }
        let mut literals = Vec::new();
        self.visit_free_join(|rule| {
            let plan = &rule.plan;
            for occurrence in plan
                .occurrences()
                .iter()
                .filter(|occurrence| !occurrence.role.discharged())
            {
                for selection in &occurrence.selections {
                    if let Const::PendingIntern { bytes } = &selection.value {
                        let label = pending_literal_label(bytes);
                        if !literals.contains(&label) {
                            literals.push(label);
                        }
                    }
                }
                for filter in &occurrence.filters {
                    if let FilterPredicate::Compare {
                        value: Const::PendingIntern { bytes },
                        ..
                    } = filter
                    {
                        let label = pending_literal_label(bytes);
                        if !literals.contains(&label) {
                            literals.push(label);
                        }
                    }
                }
            }
        });
        Some(format!(
            "pending literals: {} — an unresolved Eq literal empties its rule at execution until latched\n",
            literals.join(", ")
        ))
    }

    /// The query in the rule notation, rendered at prepare
    /// ([`crate::ir::render`] — one rendered block per rule, `;`-terminated):
    /// the diagnostic twin of the introspection report's header.
    #[must_use]
    pub fn rendered_query(&self) -> &str {
        &self.rendered
    }

    /// Whether the aggregate sink's binding seen-set is elided — the
    /// regime observable for the batch-fold fast path. A single-rule
    /// query may elide under its plan's distinct-bindings proof. A
    /// multi-rule query always returns false: its spanning
    /// head-projection seen-set is the union representation.
    #[must_use]
    pub fn distinct_bindings(&self) -> bool {
        match &self.pipeline {
            super::PreparedPipeline::PointProbe { rule, .. } => rule.distinct_witness.is_some(),
            super::PreparedPipeline::Cq { .. } | super::PreparedPipeline::Reach { .. } => {
                match self.pipeline.main_rules() {
                    [rule] => rule.distinct_witness().is_some(),
                    _ => false,
                }
            }
        }
    }

    /// The signature this query defines — the buffer-typing authority:
    /// one column per head position, the metadata a generic host needs
    /// to type an (even empty) result. The buffer itself stays typeless:
    /// stamping owned types per execution would allocate on the warm path.
    #[must_use]
    pub fn signature(&self) -> &crate::ir::validate::Signature {
        &self.signature
    }
}

fn pending_literal_label(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}
