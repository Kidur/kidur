//! Kidur core types. Pure data — no IO, no async, no external workspace deps.

mod id;
mod visibility;
mod field_value;
mod node;
mod edge;
mod supertag_def;
mod error;

pub use id::NodeId;
pub use visibility::Visibility;
pub use field_value::FieldValue;
pub use node::Node;
pub use edge::Edge;
pub use supertag_def::{FieldDef, FieldType, SupertagDef};
pub use error::{KidurError, KidurResult};
