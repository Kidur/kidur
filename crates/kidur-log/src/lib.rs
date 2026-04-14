//! Append-only .jsonl mutation log for Kidur.
//!
//! This crate provides the canonical data layer: every write to the Kidur graph
//! is first recorded as a line in a `.jsonl` file. The database (SurrealDB) is
//! a performance index that can be rebuilt from this log at any time.
//!
//! # Usage
//!
//! ```no_run
//! use kidur_log::{MutationLog, Mutation};
//! use kidur_core::Node;
//!
//! let mut log = MutationLog::open("data/kidur.jsonl").unwrap();
//! let node = Node::new("Hello Kidur");
//! log.append(Mutation::CreateNode { node }).unwrap();
//!
//! let entries = MutationLog::replay("data/kidur.jsonl").unwrap();
//! assert_eq!(entries.len(), 1);
//! ```

mod mutation;

pub use mutation::{LogEntry, Mutation};

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use chrono::Utc;
use kidur_core::{KidurError, KidurResult};

/// An append-only mutation log backed by a `.jsonl` file.
///
/// One file per Kidur instance. Each line is a [`LogEntry`] — a sequenced,
/// timestamped [`Mutation`].
pub struct MutationLog {
    path: PathBuf,
    next_seq: u64,
}

impl MutationLog {
    /// Open (or create) a mutation log at the given path.
    ///
    /// Reads the existing file to determine the next sequence number.
    /// If the file doesn't exist, it will be created on the first `append`.
    pub fn open(path: impl Into<PathBuf>) -> KidurResult<Self> {
        let path = path.into();
        let next_seq = if path.exists() {
            Self::read_max_seq(&path)? + 1
        } else {
            1
        };
        Ok(MutationLog { path, next_seq })
    }

    /// Append a mutation to the log.
    ///
    /// Assigns the next sequence number and current UTC timestamp, then
    /// writes one JSON line and flushes.
    pub fn append(&mut self, mutation: Mutation) -> KidurResult<LogEntry> {
        let entry = LogEntry {
            seq: self.next_seq,
            ts: Utc::now(),
            mutation,
        };

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| KidurError::Other(format!("log open: {}", e)))?;

        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &entry)
            .map_err(|e| KidurError::Other(format!("log write: {}", e)))?;
        writer
            .write_all(b"\n")
            .map_err(|e| KidurError::Other(format!("log newline: {}", e)))?;
        writer
            .flush()
            .map_err(|e| KidurError::Other(format!("log flush: {}", e)))?;

        self.next_seq += 1;
        Ok(entry)
    }

    /// Replay all entries from a log file, in order.
    ///
    /// This is a static method — it doesn't need a `MutationLog` instance.
    /// Use this to rebuild a database index from scratch.
    pub fn replay(path: impl AsRef<Path>) -> KidurResult<Vec<LogEntry>> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file =
            File::open(path).map_err(|e| KidurError::Other(format!("log read: {}", e)))?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line =
                line.map_err(|e| KidurError::Other(format!("log line {}: {}", line_num + 1, e)))?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let entry: LogEntry = serde_json::from_str(trimmed).map_err(|e| {
                KidurError::Other(format!("log parse line {}: {}", line_num + 1, e))
            })?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// The sequence number that will be assigned to the next `append`.
    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }

    /// Path to the backing .jsonl file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read the file and return the highest seq number found (0 if empty).
    fn read_max_seq(path: &Path) -> KidurResult<u64> {
        let file =
            File::open(path).map_err(|e| KidurError::Other(format!("log read: {}", e)))?;
        let reader = BufReader::new(file);
        let mut max_seq: u64 = 0;

        for line in reader.lines() {
            let line = line.map_err(|e| KidurError::Other(format!("log line: {}", e)))?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Parse just enough to get seq — avoid full deserialization
            if let Ok(entry) = serde_json::from_str::<LogEntry>(trimmed) {
                max_seq = max_seq.max(entry.seq);
            }
        }

        Ok(max_seq)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kidur_core::{Edge, Node, NodeId};
    use tempfile::TempDir;

    fn log_path(dir: &TempDir) -> PathBuf {
        dir.path().join("kidur.jsonl")
    }

    #[test]
    fn open_creates_empty_log() {
        let dir = TempDir::new().unwrap();
        let log = MutationLog::open(log_path(&dir)).unwrap();
        assert_eq!(log.next_seq(), 1);
    }

    #[test]
    fn append_and_replay_single() {
        let dir = TempDir::new().unwrap();
        let path = log_path(&dir);
        let mut log = MutationLog::open(&path).unwrap();

        let node = Node::new("first");
        let entry = log
            .append(Mutation::CreateNode { node: node.clone() })
            .unwrap();
        assert_eq!(entry.seq, 1);
        assert_eq!(log.next_seq(), 2);

        let entries = MutationLog::replay(&path).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].seq, 1);
        match &entries[0].mutation {
            Mutation::CreateNode { node: n } => {
                assert_eq!(n.id, node.id);
                assert_eq!(n.content, "first");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn append_multiple_preserves_order() {
        let dir = TempDir::new().unwrap();
        let path = log_path(&dir);
        let mut log = MutationLog::open(&path).unwrap();

        let n1 = Node::new("one");
        let n2 = Node::new("two");
        let n3 = Node::new("three");

        log.append(Mutation::CreateNode { node: n1 }).unwrap();
        log.append(Mutation::CreateNode { node: n2 }).unwrap();
        log.append(Mutation::CreateNode { node: n3 }).unwrap();

        assert_eq!(log.next_seq(), 4);

        let entries = MutationLog::replay(&path).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].seq, 1);
        assert_eq!(entries[1].seq, 2);
        assert_eq!(entries[2].seq, 3);
    }

    #[test]
    fn reopen_continues_sequence() {
        let dir = TempDir::new().unwrap();
        let path = log_path(&dir);

        {
            let mut log = MutationLog::open(&path).unwrap();
            log.append(Mutation::CreateNode {
                node: Node::new("a"),
            })
            .unwrap();
            log.append(Mutation::CreateNode {
                node: Node::new("b"),
            })
            .unwrap();
        }

        // Reopen — should continue from seq 3
        let mut log = MutationLog::open(&path).unwrap();
        assert_eq!(log.next_seq(), 3);

        log.append(Mutation::CreateNode {
            node: Node::new("c"),
        })
        .unwrap();

        let entries = MutationLog::replay(&path).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[2].seq, 3);
    }

    #[test]
    fn all_mutation_variants() {
        let dir = TempDir::new().unwrap();
        let path = log_path(&dir);
        let mut log = MutationLog::open(&path).unwrap();

        let node = Node::new("test");
        let node_id = node.id;
        let from = NodeId::new();
        let to = NodeId::new();

        log.append(Mutation::CreateNode { node: node.clone() })
            .unwrap();

        let mut updated = node;
        updated.content = "updated".into();
        log.append(Mutation::UpdateNode { node: updated })
            .unwrap();

        log.append(Mutation::CreateEdge {
            edge: Edge::new(from, to, "refs"),
        })
        .unwrap();

        log.append(Mutation::DeleteEdge {
            from_id: from,
            to_id: to,
            kind: "refs".into(),
        })
        .unwrap();

        log.append(Mutation::DeleteNode { id: node_id }).unwrap();

        let entries = MutationLog::replay(&path).unwrap();
        assert_eq!(entries.len(), 5);

        // Verify variant types in order
        assert!(matches!(&entries[0].mutation, Mutation::CreateNode { .. }));
        assert!(matches!(&entries[1].mutation, Mutation::UpdateNode { .. }));
        assert!(matches!(&entries[2].mutation, Mutation::CreateEdge { .. }));
        assert!(matches!(&entries[3].mutation, Mutation::DeleteEdge { .. }));
        assert!(matches!(&entries[4].mutation, Mutation::DeleteNode { .. }));
    }

    #[test]
    fn replay_nonexistent_file_returns_empty() {
        let entries = MutationLog::replay("/tmp/does-not-exist-kidur.jsonl").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn replay_skips_blank_lines() {
        let dir = TempDir::new().unwrap();
        let path = log_path(&dir);

        // Write one entry, then a blank line
        let mut log = MutationLog::open(&path).unwrap();
        log.append(Mutation::CreateNode {
            node: Node::new("x"),
        })
        .unwrap();

        // Manually append a blank line
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"\n")
            .unwrap();

        let entries = MutationLog::replay(&path).unwrap();
        assert_eq!(entries.len(), 1);
    }
}
