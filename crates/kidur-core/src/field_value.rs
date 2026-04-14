use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::NodeId;

/// A typed value that can be stored in a node's field.
///
/// Uses adjacently-tagged serde representation: `{"kind": "text", "value": "..."}`.
/// FieldValue deliberately does NOT derive Eq because f64 (Number variant) does not implement Eq.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum FieldValue {
    /// Plain text content.
    Text(String),
    /// Rich text (may contain markup or markdown).
    RichText(String),
    /// Numeric value stored as f64.
    Number(f64),
    /// Boolean flag.
    Bool(bool),
    /// Single choice from an enumeration.
    Enum(String),
    /// Multiple choices selected.
    MultiSelect(Vec<String>),
    /// Reference to another node by ID.
    Reference(NodeId),
    /// A point in time (UTC).
    Timestamp(DateTime<Utc>),
    /// An email address.
    Email(String),
    /// A URL.
    Url(String),
    /// Geographic location with optional label.
    Geo {
        lat: f64,
        lng: f64,
        label: Option<String>,
    },
}

impl FieldValue {
    /// Return the discriminant name as a static string.
    pub fn kind(&self) -> &'static str {
        match self {
            FieldValue::Text(_) => "text",
            FieldValue::RichText(_) => "rich_text",
            FieldValue::Number(_) => "number",
            FieldValue::Bool(_) => "bool",
            FieldValue::Enum(_) => "enum",
            FieldValue::MultiSelect(_) => "multi_select",
            FieldValue::Reference(_) => "reference",
            FieldValue::Timestamp(_) => "timestamp",
            FieldValue::Email(_) => "email",
            FieldValue::Url(_) => "url",
            FieldValue::Geo { .. } => "geo",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(v: &FieldValue) -> FieldValue {
        let json = serde_json::to_string(v).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn test_text_roundtrip() {
        let v = FieldValue::Text("hello world".to_string());
        let rt = roundtrip(&v);
        assert!(matches!(rt, FieldValue::Text(s) if s == "hello world"));
    }

    #[test]
    fn test_geo_roundtrip() {
        let v = FieldValue::Geo {
            lat: 48.2082,
            lng: 16.3738,
            label: Some("Vienna".to_string()),
        };
        let rt = roundtrip(&v);
        match rt {
            FieldValue::Geo { lat, lng, label } => {
                assert!((lat - 48.2082).abs() < 1e-9);
                assert!((lng - 16.3738).abs() < 1e-9);
                assert_eq!(label.as_deref(), Some("Vienna"));
            }
            _ => panic!("expected Geo variant"),
        }
    }

    #[test]
    fn test_reference_roundtrip() {
        let id = NodeId::new();
        let v = FieldValue::Reference(id);
        let rt = roundtrip(&v);
        assert!(matches!(rt, FieldValue::Reference(rid) if rid == id));
    }

    #[test]
    fn test_multiselect_roundtrip() {
        let v = FieldValue::MultiSelect(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        let rt = roundtrip(&v);
        match rt {
            FieldValue::MultiSelect(items) => assert_eq!(items, vec!["a", "b", "c"]),
            _ => panic!("expected MultiSelect"),
        }
    }

    #[test]
    fn test_kind_check() {
        assert_eq!(FieldValue::Text("x".to_string()).kind(), "text");
        assert_eq!(FieldValue::RichText("x".to_string()).kind(), "rich_text");
        assert_eq!(FieldValue::Number(1.0).kind(), "number");
        assert_eq!(FieldValue::Bool(true).kind(), "bool");
        assert_eq!(FieldValue::Enum("x".to_string()).kind(), "enum");
        assert_eq!(
            FieldValue::MultiSelect(vec![]).kind(),
            "multi_select"
        );
        assert_eq!(FieldValue::Reference(NodeId::nil()).kind(), "reference");
        assert_eq!(FieldValue::Timestamp(Utc::now()).kind(), "timestamp");
        assert_eq!(FieldValue::Email("a@b.com".to_string()).kind(), "email");
        assert_eq!(FieldValue::Url("https://example.com".to_string()).kind(), "url");
        assert_eq!(
            FieldValue::Geo { lat: 0.0, lng: 0.0, label: None }.kind(),
            "geo"
        );
    }
}
