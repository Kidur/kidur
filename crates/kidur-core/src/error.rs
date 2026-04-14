use thiserror::Error;
use crate::NodeId;

#[derive(Debug, Error)]
pub enum KidurError {
    #[error("node not found: {0}")]
    NodeNotFound(NodeId),
    #[error("edge not found: {from} -> {to} ({kind})")]
    EdgeNotFound { from: NodeId, to: NodeId, kind: String },
    #[error("unknown supertag: {0}")]
    UnknownSupertag(String),
    #[error("field validation failed on '{field}': {reason}")]
    FieldValidation { field: String, reason: String },
    #[error("missing required field: {0}")]
    MissingRequiredField(String),
    #[error("CRDT error: {0}")]
    Crdt(String),
    #[error("store error: {0}")]
    Store(String),
    #[error("supertag parse error: {0}")]
    SupertagParse(String),
    #[error("{0}")]
    Other(String),
}

pub type KidurResult<T> = Result<T, KidurError>;
