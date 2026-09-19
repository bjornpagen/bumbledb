//! Owned regions of explicitly admissible worlds.
//!
//! The same completed Boolean function supports values, dependencies and
//! relational operators. Probability is a separate observation. This crate has
//! no dependency on storage, the query engine, or a model provider.
#![forbid(unsafe_code)]
#[cfg(not(target_pointer_width = "64"))]
compile_error!("bumbledb-event currently requires a 64-bit target");

mod action;
mod algebraic;
mod arena;
mod boolean;
mod codec;
mod descriptor;
mod diagram;
mod error;
mod exact;
mod expectation;
mod fixed_point;
mod function;
mod information;
mod kernel;
mod map;
mod measure;
mod parameter;
mod partition;
mod polynomial;
mod product;
mod program;
mod registry;
mod relation;
mod revision;
mod space;

pub use action::{ActionArena, ReachStrategy, SafetyStrategy};
pub use algebraic::{AlgebraicLimits, AlgebraicRoot, RootLimits};
pub use boolean::{BoolOp4, Signature};
pub use descriptor::{
    AdmittedDescriptor, AdmittedSourceDescriptor, Descriptor, DescriptorLimits, FibreDescriptor,
    FunctionDescriptor, FunctionPieceDescriptor, KernelDescriptor, MapDescriptor,
    RevisionDescriptor, RevisionOutcomeDescriptor, RevisionReceiptDescriptor, SourceDescriptor,
    SourceDescriptorLimits,
};
pub use diagram::{Diagram, DiagramNode, DiagramView};
pub use error::{Capacity, Control, Error, Limits, Result};
pub use exact::{ArithmeticLimits, ExactArithmetic, ExactRational};
pub use expectation::ExpectationObservation;
pub use fixed_point::{
    FiniteCarrier, FixedPointLimits, FixedPointProgram, FixedPointResult, LayeredFixedPointResult,
};
pub use function::{FiniteFunction, FunctionLimits, FunctionPiece};
pub use information::InformationCases;
pub use kernel::{FiniteKernel, SourceExtension};
pub use map::{CoordinateMap, SurjectiveMap};
pub use measure::{DensityPiece, LawLimits, ProbabilityObservation};
pub use parameter::{
    GuardedRationalFunction, ParameterCell, ParameterCodecLimits, ParameterDensityPiece,
    ParameterDomain, ParameterFunction, ParameterGuard, ParameterLimits,
    ParameterProbabilityObservation, ParameterRefinement, ParameterRegion, ParameterSourceLimits,
    ParameterWorld, PolynomialSigns, RealWitness, WorldCardinality,
};
pub use partition::{EventPartition, PartitionLimits};
pub use polynomial::{ExactPolynomial, ParameterId, PolynomialLimits, PolynomialTerm};
pub use product::{CompleteFibreSquare, FaceProduct, FibreProduct};
pub use program::{
    EventProgram, EventProgramBuilder, MapOp, ModalOp, ProgramInstruction, ProgramOp, ProgramValue,
    Variance,
};
pub use registry::Registry;
pub use relation::{RelationalProduct, WorldRelation};
pub use revision::{
    RevisedSource, RevisionImpossible, RevisionOutcome, RevisionReceipt, SourceRevision,
};
pub use space::{Event, EventKey, Space, SpaceId, Statistics};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod map_tests;

#[cfg(test)]
mod relation_tests;

#[cfg(test)]
mod diagram_tests;

#[cfg(test)]
mod information_tests;

#[cfg(test)]
mod program_tests;

#[cfg(test)]
mod partition_tests;

#[cfg(test)]
mod action_tests;

#[cfg(test)]
mod descriptor_tests;

#[cfg(test)]
mod exact_tests;

#[cfg(test)]
mod measure_tests;

#[cfg(test)]
mod function_tests;

#[cfg(test)]
mod revision_tests;

#[cfg(test)]
mod source_descriptor_tests;

#[cfg(test)]
mod polynomial_tests;

#[cfg(test)]
mod algebraic_tests;

#[cfg(test)]
mod parameter_tests;

#[cfg(test)]
mod algebraic_identity_tests;

#[cfg(test)]
mod parameter_source_tests;

#[cfg(test)]
mod parameter_refinement_tests;
