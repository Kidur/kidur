use loro::{ExportMode, LoroDoc};
use kidur_core::{KidurError, KidurResult};
use crate::CrdtDoc;

const TEXT_CONTAINER: &str = "content";

/// A CRDT document backed by [Loro](https://loro.dev).
pub struct LoroCrdtDoc {
    doc: LoroDoc,
}

impl std::fmt::Debug for LoroCrdtDoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoroCrdtDoc").finish()
    }
}

impl LoroCrdtDoc {
    /// Create a fresh, empty document.
    pub fn new() -> Self {
        Self { doc: LoroDoc::new() }
    }

    /// Restore a document from a previously exported snapshot.
    pub fn from_snapshot(bytes: &[u8]) -> KidurResult<Self> {
        let doc = LoroDoc::new();
        doc.import(bytes).map_err(|e| KidurError::Crdt(e.to_string()))?;
        Ok(Self { doc })
    }
}

impl Default for LoroCrdtDoc {
    fn default() -> Self {
        Self::new()
    }
}

impl CrdtDoc for LoroCrdtDoc {
    fn export_snapshot(&self) -> KidurResult<Vec<u8>> {
        self.doc
            .export(ExportMode::Snapshot)
            .map_err(|e| KidurError::Crdt(e.to_string()))
    }

    fn import_snapshot(&mut self, bytes: &[u8]) -> KidurResult<()> {
        // Replace state: create a fresh doc and import the snapshot into it.
        let doc = LoroDoc::new();
        doc.import(bytes).map_err(|e| KidurError::Crdt(e.to_string()))?;
        self.doc = doc;
        Ok(())
    }

    fn get_text(&self) -> KidurResult<String> {
        Ok(self.doc.get_text(TEXT_CONTAINER).to_string())
    }

    fn insert_text(&mut self, pos: usize, text: &str) -> KidurResult<()> {
        self.doc
            .get_text(TEXT_CONTAINER)
            .insert(pos, text)
            .map_err(|e| KidurError::Crdt(e.to_string()))
    }

    fn delete_text(&mut self, pos: usize, len: usize) -> KidurResult<()> {
        self.doc
            .get_text(TEXT_CONTAINER)
            .delete(pos, len)
            .map_err(|e| KidurError::Crdt(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CrdtDoc;

    #[test]
    fn empty_doc_has_empty_text() {
        let doc = LoroCrdtDoc::new();
        assert_eq!(doc.get_text().unwrap(), "");
    }

    #[test]
    fn insert_and_read() {
        let mut doc = LoroCrdtDoc::new();
        doc.insert_text(0, "hello").unwrap();
        assert_eq!(doc.get_text().unwrap(), "hello");
    }

    #[test]
    fn insert_and_delete() {
        let mut doc = LoroCrdtDoc::new();
        doc.insert_text(0, "hello world").unwrap();
        doc.delete_text(5, 6).unwrap();
        assert_eq!(doc.get_text().unwrap(), "hello");
    }

    #[test]
    fn snapshot_roundtrip() {
        let mut doc = LoroCrdtDoc::new();
        doc.insert_text(0, "snapshot test").unwrap();
        let bytes = doc.export_snapshot().unwrap();

        let doc2 = LoroCrdtDoc::from_snapshot(&bytes).unwrap();
        assert_eq!(doc2.get_text().unwrap(), "snapshot test");
    }

    #[test]
    fn import_snapshot_replaces_state() {
        let mut doc = LoroCrdtDoc::new();
        doc.insert_text(0, "original").unwrap();
        let bytes = doc.export_snapshot().unwrap();

        let mut doc2 = LoroCrdtDoc::new();
        doc2.insert_text(0, "will be replaced").unwrap();
        doc2.import_snapshot(&bytes).unwrap();
        assert_eq!(doc2.get_text().unwrap(), "original");
    }
}
