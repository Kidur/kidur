use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A strongly typed node identifier backed by UUID v7.
///
/// UUID v7 provides monotonically increasing IDs that sort by creation time,
/// which is important for ordering nodes in a local-first context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(Uuid);

impl NodeId {
    /// Create a new, unique NodeId using UUID v7 (time-ordered).
    pub fn new() -> Self {
        NodeId(Uuid::now_v7())
    }

    /// Return the nil (all-zeros) NodeId, useful as a sentinel value.
    pub fn nil() -> Self {
        NodeId(Uuid::nil())
    }

    /// Wrap an existing Uuid value as a NodeId.
    pub fn from_uuid(uuid: Uuid) -> Self {
        NodeId(uuid)
    }

    /// Returns true if this is the nil UUID.
    pub fn is_nil(&self) -> bool {
        self.0.is_nil()
    }

    /// Returns the inner Uuid.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for NodeId {
    fn default() -> Self {
        NodeId::new()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for NodeId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(NodeId(Uuid::parse_str(s)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique() {
        let a = NodeId::new();
        let b = NodeId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn test_time_sorted() {
        let a = NodeId::new();
        let b = NodeId::new();
        // UUID v7 is monotonically increasing
        assert!(a < b);
    }

    #[test]
    fn test_json_roundtrip() {
        let id = NodeId::new();
        let json = serde_json::to_string(&id).unwrap();
        let restored: NodeId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, restored);
    }

    #[test]
    fn test_string_roundtrip() {
        let id = NodeId::new();
        let s = id.to_string();
        let restored: NodeId = s.parse().unwrap();
        assert_eq!(id, restored);
    }

    #[test]
    fn test_nil_check() {
        let nil = NodeId::nil();
        assert!(nil.is_nil());
        let real = NodeId::new();
        assert!(!real.is_nil());
    }

    #[test]
    fn test_from_uuid() {
        let uuid = Uuid::now_v7();
        let id = NodeId::from_uuid(uuid);
        assert_eq!(id.as_uuid(), uuid);
    }
}
