use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::NodeId;

/// A typed directed edge between two nodes.
///
/// Edges represent semantic relationships between nodes beyond the implicit
/// parent/child tree structure. Examples: "references", "blocks", "related".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    /// The source node of this edge.
    pub from_id: NodeId,
    /// The target node of this edge.
    pub to_id: NodeId,
    /// A string label describing the relationship type.
    pub kind: String,
    /// When this edge was created (UTC).
    pub created_at: DateTime<Utc>,
}

impl Edge {
    /// Create a new edge between two nodes with the given relationship kind.
    pub fn new(from_id: NodeId, to_id: NodeId, kind: impl Into<String>) -> Self {
        Edge {
            from_id,
            to_id,
            kind: kind.into(),
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creation() {
        let from = NodeId::new();
        let to = NodeId::new();
        let edge = Edge::new(from, to, "references");

        assert_eq!(edge.from_id, from);
        assert_eq!(edge.to_id, to);
        assert_eq!(edge.kind, "references");
    }

    #[test]
    fn test_json_roundtrip() {
        let from = NodeId::new();
        let to = NodeId::new();
        let edge = Edge::new(from, to, "blocks");

        let json = serde_json::to_string(&edge).unwrap();
        let restored: Edge = serde_json::from_str(&json).unwrap();

        assert_eq!(edge.from_id, restored.from_id);
        assert_eq!(edge.to_id, restored.to_id);
        assert_eq!(edge.kind, restored.kind);
        assert_eq!(edge.created_at, restored.created_at);
    }

    #[test]
    fn test_equality() {
        let from = NodeId::new();
        let to = NodeId::new();
        let edge1 = Edge::new(from, to, "link");
        let edge2 = edge1.clone();
        assert_eq!(edge1, edge2);
    }
}
