//! Native evidence for an imperative schema transition. A caller commitment
//! identifies intent; it is never executable code or a proof of preservation.

pub mod hosted;
pub mod local;
pub mod namespace;

use bumbledb::Uuid;

use crate::history::frame::{Reader, begin_frame, frame_len, put_identity, put_stamp, put_state};
use crate::history::{DatabaseIdentity, DecisionStamp, FrameError, OperationId, StateStamp};

const FAMILY: &[u8] = b"bumbledb.transition.v1\0";
const LAYOUT: u16 = 1;
const CONTRACT: u8 = 1;
const CAPTURED: u8 = 2;
const INSTALLED: u8 = 3;
const CONTRACT_BYTES: usize = 16 + 64 + 64 + 32;
const CAPTURED_BYTES: usize = CONTRACT_BYTES + 40 + 24;

/// Retain before dispatch. Both identities include their complete native
/// schema fingerprint. Admission recompiles the actual supplied bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contract {
    pub operation: OperationId,
    pub source: DatabaseIdentity,
    pub target: DatabaseIdentity,
    pub commitment: [u8; 32],
}

/// Exact authority position frozen by the operation, independent of the
/// lifetime of a query reader or an unsuccessful population attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Captured {
    pub contract: Contract,
    pub decision: DecisionStamp,
    pub state: StateStamp,
}

/// Retained installed content. Activation and all later writes preserve
/// this evidence; resolution never compares mutable facts with genesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Installed {
    pub captured: Captured,
    pub application_digest: [u8; 32],
}

fn put_contract(out: &mut Vec<u8>, contract: &Contract) {
    out.extend_from_slice(contract.operation.as_core().as_bytes());
    put_identity(out, contract.source);
    put_identity(out, contract.target);
    out.extend_from_slice(&contract.commitment);
}

fn read_contract(input: &mut Reader<'_>) -> Result<Contract, FrameError> {
    Ok(Contract {
        operation: OperationId::from_core(Uuid::from_bytes(input.array()?)),
        source: input.identity()?,
        target: input.identity()?,
        commitment: input.array()?,
    })
}

fn put_captured(out: &mut Vec<u8>, captured: &Captured) {
    put_contract(out, &captured.contract);
    put_stamp(out, captured.decision);
    put_state(out, captured.state);
}

fn read_captured(input: &mut Reader<'_>) -> Result<Captured, FrameError> {
    let captured = Captured {
        contract: read_contract(input)?,
        decision: input.stamp()?,
        state: input.state()?,
    };
    if captured.contract.source.incarnation_id != captured.state.incarnation {
        return Err(FrameError::StateIdentityMismatch);
    }
    Ok(captured)
}

impl Contract {
    /// Native operation data, never a transformation program.
    /// # Errors
    /// Bounded frame allocation refusal.
    pub fn encode(&self, cap: usize) -> Result<Vec<u8>, FrameError> {
        let len = frame_len(FAMILY.len(), &[CONTRACT_BYTES])?;
        let mut bytes = begin_frame(FAMILY, LAYOUT, CONTRACT, len, cap)?;
        put_contract(&mut bytes, self);
        Ok(bytes)
    }

    /// # Errors
    /// Wrong families/kinds, truncation and trailing bytes refuse.
    pub fn decode(bytes: &[u8], cap: usize) -> Result<Self, FrameError> {
        let mut input = Reader::begin(bytes, FAMILY, LAYOUT, CONTRACT, cap)?;
        let contract = read_contract(&mut input)?;
        input.end()?;
        Ok(contract)
    }
    /// Hash the entire retained contract in its own format domain.
    /// # Errors
    /// Bounded frame allocation refusal.
    pub fn digest(&self) -> Result<[u8; 32], FrameError> {
        let bytes = self.encode(usize::MAX)?;
        Ok(blake3::derive_key(
            "bumbledb.transition.v1/contract",
            &bytes,
        ))
    }
}

impl Captured {
    /// Encode the frozen source evidence. This format admits no plan chain.
    /// # Errors
    /// Invalid source stamp or bounded allocation refusal.
    pub fn encode(&self, cap: usize) -> Result<Vec<u8>, FrameError> {
        if self.contract.source.incarnation_id != self.state.incarnation {
            return Err(FrameError::StateIdentityMismatch);
        }
        let len = frame_len(FAMILY.len(), &[CAPTURED_BYTES])?;
        let mut bytes = begin_frame(FAMILY, LAYOUT, CAPTURED, len, cap)?;
        put_captured(&mut bytes, self);
        Ok(bytes)
    }

    /// # Errors
    /// Wrong families (including retired migration artifacts), malformed
    /// source stamps, truncation and trailing bytes refuse.
    pub fn decode(bytes: &[u8], cap: usize) -> Result<Self, FrameError> {
        let mut input = Reader::begin(bytes, FAMILY, LAYOUT, CAPTURED, cap)?;
        let captured = read_captured(&mut input)?;
        input.end()?;
        Ok(captured)
    }
}

impl Installed {
    /// # Errors
    /// Invalid source stamp or bounded allocation refusal.
    pub fn encode(&self, cap: usize) -> Result<Vec<u8>, FrameError> {
        if self.captured.contract.source.incarnation_id != self.captured.state.incarnation {
            return Err(FrameError::StateIdentityMismatch);
        }
        let len = frame_len(FAMILY.len(), &[CAPTURED_BYTES, 32])?;
        let mut bytes = begin_frame(FAMILY, LAYOUT, INSTALLED, len, cap)?;
        put_captured(&mut bytes, &self.captured);
        bytes.extend_from_slice(&self.application_digest);
        Ok(bytes)
    }

    /// # Errors
    /// Wrong families/kinds, invalid source stamps and malformed frames.
    pub fn decode(bytes: &[u8], cap: usize) -> Result<Self, FrameError> {
        let mut input = Reader::begin(bytes, FAMILY, LAYOUT, INSTALLED, cap)?;
        let installed = Self {
            captured: read_captured(&mut input)?,
            application_digest: input.array()?,
        };
        input.end()?;
        Ok(installed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{DatabaseId, DecisionDigest, IncarnationId, SchemaId};

    fn evidence() -> Installed {
        let source = DatabaseIdentity {
            database_id: DatabaseId::from_core(Uuid::from_bytes([1; 16])),
            incarnation_id: IncarnationId::from_core(Uuid::from_bytes([2; 16])),
            schema_id: SchemaId([3; 32]),
        };
        Installed {
            captured: Captured {
                contract: Contract {
                    operation: OperationId::from_core(Uuid::from_bytes([4; 16])),
                    source,
                    target: DatabaseIdentity {
                        incarnation_id: IncarnationId::from_core(Uuid::from_bytes([5; 16])),
                        schema_id: SchemaId([6; 32]),
                        ..source
                    },
                    commitment: [7; 32],
                },
                decision: DecisionStamp {
                    seq: 42,
                    hash: DecisionDigest::from_bytes([8; 32]),
                },
                state: StateStamp {
                    incarnation: source.incarnation_id,
                    data_revision: 31,
                },
            },
            application_digest: [9; 32],
        }
    }

    #[test]
    fn complete_contract_identity_changes_with_every_coordinate() {
        let original = evidence().captured.contract;
        let mut variants = [original; 8];
        variants[0].operation = OperationId::from_core(Uuid::from_bytes([99; 16]));
        variants[1].source.database_id = DatabaseId::from_core(Uuid::from_bytes([99; 16]));
        variants[2].source.incarnation_id = IncarnationId::from_core(Uuid::from_bytes([99; 16]));
        variants[3].source.schema_id = SchemaId([99; 32]);
        variants[4].target.database_id = DatabaseId::from_core(Uuid::from_bytes([99; 16]));
        variants[5].target.incarnation_id = IncarnationId::from_core(Uuid::from_bytes([99; 16]));
        variants[6].target.schema_id = SchemaId([99; 32]);
        variants[7].commitment = [99; 32];
        for variant in variants {
            assert_ne!(variant.digest().unwrap(), original.digest().unwrap());
        }
    }

    #[test]
    fn new_evidence_roundtrips_and_every_truncation_refuses() {
        let installed = evidence();
        let bytes = installed.encode(4096).unwrap();
        assert_eq!(Installed::decode(&bytes, bytes.len()).unwrap(), installed);
        for length in 0..bytes.len() {
            assert!(Installed::decode(&bytes[..length], 4096).is_err());
        }
        let contract = installed.captured.contract.encode(4096).unwrap();
        assert_eq!(
            Contract::decode(&contract, contract.len()).unwrap(),
            installed.captured.contract
        );
        for length in 0..contract.len() {
            assert!(Contract::decode(&contract[..length], 4096).is_err());
        }
        assert!(Contract::decode(&bytes, 4096).is_err());
        assert!(Installed::decode(&contract, 4096).is_err());
        let captured = installed.captured.encode(4096).unwrap();
        assert_eq!(
            Captured::decode(&captured, captured.len()).unwrap(),
            installed.captured
        );
        for length in 0..captured.len() {
            assert!(Captured::decode(&captured[..length], 4096).is_err());
        }
        assert!(Captured::decode(&bytes, 4096).is_err());
        assert!(Installed::decode(&captured, 4096).is_err());
        assert!(
            matches!(installed.encode(bytes.len() - 1), Err(FrameError::LimitExceeded { required, limit, .. }) if required == bytes.len() && limit == bytes.len() - 1)
        );
        let mut trailing = bytes;
        trailing.push(0);
        assert!(matches!(
            Installed::decode(&trailing, 4096),
            Err(FrameError::TrailingBytes { .. })
        ));
    }

    #[test]
    fn old_migration_records_are_not_caller_commitments() {
        for kind in 1..=8 {
            let mut old = b"bumbledb.migration.v1\0\0\x01".to_vec();
            old.push(kind);
            old.extend_from_slice(&[0; 512]);
            assert_eq!(Contract::decode(&old, 4096), Err(FrameError::Family));
            assert_eq!(Captured::decode(&old, 4096), Err(FrameError::Family));
            assert_eq!(Installed::decode(&old, 4096), Err(FrameError::Family));
        }
    }
}
