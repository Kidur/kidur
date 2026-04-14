use std::collections::BTreeMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::{FieldValue, NodeId, Visibility};

/// A node in the Kidur outliner graph.
///
/// Nodes are the fundamental unit of content. Every node has an identity,
/// optional tree position (via parent_id), typed fields, and visibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Unique identifier for this node.
    pub id: NodeId,
    /// Parent node, or None if this is a root node.
    pub parent_id: Option<NodeId>,
    /// Fractional index used to order siblings.
    pub sort_order: f64,
    /// The main text content of this node.
    pub content: String,
    /// Optional supertag name (e.g. "person", "task").
    pub supertag: Option<String>,
    /// Typed key-value fields attached to this node.
    pub fields: BTreeMap<String, FieldValue>,
    /// When this node was first created (UTC).
    pub created_at: DateTime<Utc>,
    /// When this node was last modified (UTC).
    pub updated_at: DateTime<Utc>,
    /// Username or ID of the user who created this node.
    pub created_by: String,
    /// Who can see this node.
    pub visibility: Visibility,
}

impl Node {
    /// Create a new node with the given content.
    ///
    /// All optional fields are set to sensible defaults:
    /// - `parent_id` → None
    /// - `sort_order` → 0.0
    /// - `supertag` → None
    /// - `fields` → empty BTreeMap
    /// - `created_at` / `updated_at` → now (UTC)
    /// - `created_by` → empty string
    /// - `visibility` → Private
    pub fn new(content: impl Into<String>) -> Self {
        let now = Utc::now();
        Node {
            id: NodeId::new(),
            parent_id: None,
            sort_order: 0.0,
            content: content.into(),
            supertag: None,
            fields: BTreeMap::new(),
            created_at: now,
            updated_at: now,
            created_by: String::new(),
            visibility: Visibility::default(),
        }
    }

    /// Set the parent node ID (builder-style).
    pub fn with_parent(mut self, parent_id: NodeId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Set the supertag (builder-style).
    pub fn with_supertag(mut self, supertag: impl Into<String>) -> Self {
        self.supertag = Some(supertag.into());
        self
    }

    /// Add or overwrite a field (builder-style).
    pub fn with_field(mut self, key: impl Into<String>, value: FieldValue) -> Self {
        self.fields.insert(key.into(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let node = Node::new("hello");
        assert_eq!(node.content, "hello");
        assert!(node.parent_id.is_none());
        assert!(node.supertag.is_none());
        assert!(node.fields.is_empty());
        assert_eq!(node.visibility, Visibility::Private);
        assert_eq!(node.sort_order, 0.0);
        assert!(!node.id.is_nil());
    }

    #[test]
    fn test_builder_chain() {
        let parent_id = NodeId::new();
        let node = Node::new("task content")
            .with_parent(parent_id)
            .with_supertag("task")
            .with_field("priority", FieldValue::Number(1.0));

        assert_eq!(node.parent_id, Some(parent_id));
        assert_eq!(node.supertag.as_deref(), Some("task"));
        assert!(node.fields.contains_key("priority"));
        assert_eq!(node.content, "task content");
    }

    #[test]
    fn test_json_roundtrip() {
        let parent = NodeId::new();
        let node = Node::new("roundtrip")
            .with_parent(parent)
            .with_supertag("test")
            .with_field("note", FieldValue::Text("value".to_string()));

        let json = serde_json::to_string(&node).unwrap();
        let restored: Node = serde_json::from_str(&json).unwrap();

        assert_eq!(node.id, restored.id);
        assert_eq!(node.content, restored.content);
        assert_eq!(node.parent_id, restored.parent_id);
        assert_eq!(node.supertag, restored.supertag);
        assert_eq!(node.visibility, restored.visibility);
    }

    #[test]
    fn test_each_node_has_unique_id() {
        let a = Node::new("a");
        let b = Node::new("b");
        assert_ne!(a.id, b.id);
    }
}
