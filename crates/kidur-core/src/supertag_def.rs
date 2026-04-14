use serde::{Deserialize, Serialize};

/// The type system for fields within a SupertagDef.
///
/// Mirrors the variants of `FieldValue` but without data — this is the schema,
/// not an instance value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Text,
    RichText,
    Number,
    Bool,
    Enum,
    MultiSelect,
    Reference,
    Timestamp,
    Email,
    Url,
    Geo,
}

/// Schema definition for a single field within a supertag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    /// The field name (used as the key in Node.fields).
    pub name: String,
    /// The expected type of values stored in this field.
    #[serde(rename = "type")]
    pub field_type: FieldType,
    /// If true, nodes with this supertag must have this field set.
    #[serde(default)]
    pub required: bool,
    /// For Enum / MultiSelect types: the allowed option strings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    /// For Reference type: the supertag name that referenced nodes must carry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ref_tag: Option<String>,
}

impl FieldDef {
    /// Create a minimal FieldDef with just a name and type.
    pub fn new(name: impl Into<String>, field_type: FieldType) -> Self {
        FieldDef {
            name: name.into(),
            field_type,
            required: false,
            options: Vec::new(),
            ref_tag: None,
        }
    }
}

/// A supertag definition — the schema template applied to nodes.
///
/// When a node is tagged with a supertag, it inherits all field definitions
/// from the corresponding SupertagDef.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupertagDef {
    /// The supertag name (used in Node.supertag and FieldDef.ref_tag).
    pub name: String,
    /// Optional human-readable description of this supertag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The ordered list of field definitions.
    #[serde(default)]
    pub fields: Vec<FieldDef>,
}

impl SupertagDef {
    /// Create a minimal SupertagDef with just a name and no fields.
    pub fn new(name: impl Into<String>) -> Self {
        SupertagDef {
            name: name.into(),
            description: None,
            fields: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_type_roundtrip() {
        let types = [
            FieldType::Text,
            FieldType::RichText,
            FieldType::Number,
            FieldType::Bool,
            FieldType::Enum,
            FieldType::MultiSelect,
            FieldType::Reference,
            FieldType::Timestamp,
            FieldType::Email,
            FieldType::Url,
            FieldType::Geo,
        ];
        for ft in &types {
            let json = serde_json::to_string(ft).unwrap();
            let restored: FieldType = serde_json::from_str(&json).unwrap();
            assert_eq!(ft, &restored);
        }
    }

    #[test]
    fn test_minimal_def() {
        let def = SupertagDef::new("task");
        assert_eq!(def.name, "task");
        assert!(def.description.is_none());
        assert!(def.fields.is_empty());

        let json = serde_json::to_string(&def).unwrap();
        let restored: SupertagDef = serde_json::from_str(&json).unwrap();
        assert_eq!(def.name, restored.name);
    }

    #[test]
    fn test_def_with_fields() {
        let def = SupertagDef {
            name: "person".to_string(),
            description: Some("A human being".to_string()),
            fields: vec![
                FieldDef {
                    name: "email".to_string(),
                    field_type: FieldType::Email,
                    required: true,
                    options: vec![],
                    ref_tag: None,
                },
                FieldDef {
                    name: "status".to_string(),
                    field_type: FieldType::Enum,
                    required: false,
                    options: vec!["active".to_string(), "inactive".to_string()],
                    ref_tag: None,
                },
            ],
        };

        let json = serde_json::to_string(&def).unwrap();
        let restored: SupertagDef = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.name, "person");
        assert_eq!(restored.fields.len(), 2);
        assert_eq!(restored.fields[0].name, "email");
        assert!(restored.fields[0].required);
        assert_eq!(restored.fields[1].field_type, FieldType::Enum);
        assert_eq!(restored.fields[1].options, vec!["active", "inactive"]);
    }

    #[test]
    fn test_field_def_rename_type() {
        // Ensure the "type" rename is applied in JSON
        let fd = FieldDef::new("priority", FieldType::Number);
        let json = serde_json::to_string(&fd).unwrap();
        assert!(json.contains("\"type\""), "expected 'type' key in JSON: {}", json);
    }
}
