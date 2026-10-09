//! The bench and oracle suite: independent oracles that judge the engine, the
//! data worlds both engines load, and the harness that times them.
pub mod cli;
pub(crate) mod fixture;
pub mod harness;
pub mod json;
pub mod oracle;
pub mod worlds;
