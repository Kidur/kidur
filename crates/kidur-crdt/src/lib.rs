//! CRDT wrapper around Loro. Hides Loro behind the `CrdtDoc` trait.

mod doc;
mod loro_doc;

pub use doc::CrdtDoc;
pub use loro_doc::LoroCrdtDoc;
