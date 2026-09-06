use super::{Answers, ParamArg, PreparedQuery};

use crate::api::db::ReadInstance;
use crate::error::Result;

impl<S> PreparedQuery<S> {
    /// # Errors
    pub(crate) fn introspect(
        &mut self,
        instance: &ReadInstance<'_, S>,
        params: &[ParamArg<'_>],
    ) -> Result<(Answers, String)> {
        let mut out = Answers::new();
        self.execute(instance, params, &mut out)?;
        let report = format!("query:\n{}\nsignature: {}\n", self.rendered, self.signature);
        Ok((out, report))
    }

    #[must_use]
    pub fn rendered_query(&self) -> &str {
        &self.rendered
    }

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

    #[must_use]
    pub fn signature(&self) -> &crate::ir::validate::Signature {
        &self.signature
    }
}
