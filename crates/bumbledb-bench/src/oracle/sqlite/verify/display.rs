use super::VerifyFailure;

impl std::fmt::Display for VerifyFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "verify FAILED: {} mismatch(es)", self.bundles.len())?;
        for bundle in &self.bundles {
            writeln!(f, "  {}", bundle.display())?;
        }
        Ok(())
    }
}
