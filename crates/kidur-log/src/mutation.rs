use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use kidur_core::{Edge, Node, NodeId};

/// A single mutation to the Kidur graph.
///
/// Each variant captures the full state needed to replay the operation.
/// Stored as one JSON line in the .jsonl log file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Mutation {
    CreateNode { node: Node },
    UpdateNode { node: Node },
    DeleteNode { id: NodeId },
    CreateEdge { edge: Edge },
    DeleteEdge {
        from_id: NodeId,
        to_id: NodeId,
        kind: String,
    },
}

/// A timestamped, sequenced log entry wrapping a [`Mutation`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Monotonically increasing sequence number (1-based).
    pub seq: u64,
    /// Wall-clock time when the mutation was recorded.
    pub ts: DateTime<Utc>,
    /// The mutation payload.
    #[serde(flatten)]
    pub mutation: Mutation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_json_roundtrip() {
        let node = Node::new("test node");
        let m = Mutation::CreateNode { node: node.clone() };

        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains(r#""op":"create_node""#));

        let restored: Mutation = serde_json::from_str(&json).unwrap();
        match restored {
            Mutation::CreateNode { node: n } => {
                assert_eq!(n.id, node.id);
                assert_eq!(n.content, "test node");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn log_entry_flattens_mutation() {
        let node = Node::new("flat");
        let entry = LogEntry {
            seq: 1,
            ts: Utc::now(),
            mutation: Mutation::CreateNode { node },
        };

        let json = serde_json::to_string(&entry).unwrap();
        // Flattened: seq, ts, and op all at top level
        assert!(json.contains(r#""seq":1"#));
        assert!(json.contains(r#""op":"create_node""#));
        assert!(json.contains(r#""node""#));

        let restored: LogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.seq, 1);
    }

    #[test]
    fn delete_node_variant() {
        let id = NodeId::new();
        let m = Mutation::DeleteNode { id };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains(r#""op":"delete_node""#));

        let restored: Mutation = serde_json::from_str(&json).unwrap();
        match restored {
            Mutation::DeleteNode { id: restored_id } => assert_eq!(restored_id, id),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn edge_variants() {
        let from = NodeId::new();
        let to = NodeId::new();

        let create = Mutation::CreateEdge {
            edge: Edge::new(from, to, "references"),
        };
        let json = serde_json::to_string(&create).unwrap();
        assert!(json.contains(r#""op":"create_edge""#));

        let delete = Mutation::DeleteEdge {
            from_id: from,
            to_id: to,
            kind: "references".into(),
        };
        let json = serde_json::to_string(&delete).unwrap();
        assert!(json.contains(r#""op":"delete_edge""#));
    }
}
