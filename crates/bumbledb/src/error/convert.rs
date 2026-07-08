//! `From` conversions into [`Error`] and the `std::error::Error` impl.

use super::{CorruptionError, Error, FactShapeError, SchemaError, ValidationError};

impl From<heed::Error> for Error {
    fn from(err: heed::Error) -> Self {
        Self::Lmdb(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<SchemaError> for Error {
    fn from(err: SchemaError) -> Self {
        Self::Schema(err)
    }
}

impl From<ValidationError> for Error {
    fn from(err: ValidationError) -> Self {
        Self::Validation(err)
    }
}

impl From<FactShapeError> for Error {
    fn from(err: FactShapeError) -> Self {
        Self::FactShape(err)
    }
}

impl From<CorruptionError> for Error {
    fn from(err: CorruptionError) -> Self {
        Self::Corruption(err)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Lmdb(err) => Some(err),
            _ => None,
        }
    }
}
