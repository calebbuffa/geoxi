//! Error type and crate-wide `Result` alias.

use crate::internal_prelude::*;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("GEOS returned null while {0}")]
    Null(&'static str),
    #[error("GEOS reported failure while {0}")]
    Failure(&'static str),
    #[error("invalid string contains an interior NUL byte")]
    Nul(#[from] NulError),
    #[error("invalid geometry input: {0}")]
    InvalidInput(&'static str),
    #[error("GEOS predicate returned an invalid value {value} while {operation}")]
    Predicate { operation: &'static str, value: i8 },
    #[error("{0}")]
    Transform(String),
}
