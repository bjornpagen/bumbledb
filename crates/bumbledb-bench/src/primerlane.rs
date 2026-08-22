pub mod corpus;
pub mod report;
pub mod run;
#[cfg(test)]
mod tests;

pub use run::run;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimerConfig {
    pub relations: u32,

    pub facts: u64,
    pub seed: u64,
}
