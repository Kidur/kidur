use kidur_core::KidurResult;

/// Trait abstracting a CRDT document.
pub trait CrdtDoc: Send + Sync {
    fn export_snapshot(&self) -> KidurResult<Vec<u8>>;
    fn import_snapshot(&mut self, bytes: &[u8]) -> KidurResult<()>;
    fn get_text(&self) -> KidurResult<String>;
    fn insert_text(&mut self, pos: usize, text: &str) -> KidurResult<()>;
    fn delete_text(&mut self, pos: usize, len: usize) -> KidurResult<()>;
}
