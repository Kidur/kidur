use std::collections::BTreeMap;
use kidur_core::{FieldType, FieldValue, KidurError, KidurResult, SupertagDef};

/// Validate that `fields` conforms to the schema defined in `def`.
///
/// Rules:
/// - Every required field in the schema must be present.
/// - Every supplied field that appears in the schema must have a value whose
///   type matches the declared `FieldType`.
/// - For `Enum` fields: the value must be one of the declared options (if
///   options are non-empty).
/// - For `MultiSelect` fields: every selected value must be in the declared
///   options list (if options are non-empty).
/// - Extra fields NOT in the schema are allowed (open-world assumption).
pub fn validate_fields(
    def: &SupertagDef,
    fields: &BTreeMap<String, FieldValue>,
) -> KidurResult<()> {
    // Check required fields are present.
    for field_def in &def.fields {
        if field_def.required && !fields.contains_key(&field_def.name) {
            return Err(KidurError::MissingRequiredField(field_def.name.clone()));
        }
    }

    // Build a lookup map for schema fields.
    let schema: std::collections::HashMap<&str, _> =
        def.fields.iter().map(|fd| (fd.name.as_str(), fd)).collect();

    // Validate type and options for each supplied field that's in the schema.
    for (key, value) in fields {
        let field_def = match schema.get(key.as_str()) {
            Some(fd) => *fd,
            None => continue, // unknown fields are allowed (open-world)
        };

        let type_ok = match (&field_def.field_type, value) {
            (FieldType::Text, FieldValue::Text(_)) => true,
            (FieldType::RichText, FieldValue::RichText(_)) => true,
            (FieldType::Number, FieldValue::Number(_)) => true,
            (FieldType::Bool, FieldValue::Bool(_)) => true,
            (FieldType::Reference, FieldValue::Reference(_)) => true,
            (FieldType::Timestamp, FieldValue::Timestamp(_)) => true,
            (FieldType::Email, FieldValue::Email(_)) => true,
            (FieldType::Url, FieldValue::Url(_)) => true,
            (FieldType::Geo, FieldValue::Geo { .. }) => true,
            (FieldType::Enum, FieldValue::Enum(val)) => {
                // Type matches; validate options if non-empty.
                if !field_def.options.is_empty() && !field_def.options.contains(val) {
                    return Err(KidurError::FieldValidation {
                        field: key.clone(),
                        reason: format!(
                            "enum value '{}' is not in options {:?}",
                            val, field_def.options
                        ),
                    });
                }
                true
            }
            (FieldType::MultiSelect, FieldValue::MultiSelect(vals)) => {
                // Type matches; validate each selected value against options if non-empty.
                if !field_def.options.is_empty() {
                    for v in vals {
                        if !field_def.options.contains(v) {
                            return Err(KidurError::FieldValidation {
                                field: key.clone(),
                                reason: format!(
                                    "multi_select value '{}' is not in options {:?}",
                                    v, field_def.options
                                ),
                            });
                        }
                    }
                }
                true
            }
            _ => false,
        };

        if !type_ok {
            return Err(KidurError::FieldValidation {
                field: key.clone(),
                reason: format!(
                    "expected type {:?}, got kind '{}'",
                    field_def.field_type,
                    value.kind()
                ),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kidur_core::{FieldDef, FieldType, FieldValue, NodeId, SupertagDef};

    fn quest_def() -> SupertagDef {
        SupertagDef {
            name: "quest".to_string(),
            description: None,
            fields: vec![
                FieldDef {
                    name: "status".to_string(),
                    field_type: FieldType::Enum,
                    required: true,
                    options: vec![
                        "active".to_string(),
                        "completed".to_string(),
                        "paused".to_string(),
                    ],
                    ref_tag: None,
                },
                FieldDef {
                    name: "owner".to_string(),
                    field_type: FieldType::Reference,
                    required: false,
                    options: vec![],
                    ref_tag: Some("person".to_string()),
                },
                FieldDef {
                    name: "priority".to_string(),
                    field_type: FieldType::Enum,
                    required: false,
                    options: vec![
                        "low".to_string(),
                        "medium".to_string(),
                        "high".to_string(),
                        "critical".to_string(),
                    ],
                    ref_tag: None,
                },
                FieldDef {
                    name: "title".to_string(),
                    field_type: FieldType::Text,
                    required: false,
                    options: vec![],
                    ref_tag: None,
                },
            ],
        }
    }

    #[test]
    fn valid_fields_pass() {
        let def = quest_def();
        let mut fields = BTreeMap::new();
        fields.insert("status".to_string(), FieldValue::Enum("active".to_string()));
        fields.insert("priority".to_string(), FieldValue::Enum("high".to_string()));
        assert!(validate_fields(&def, &fields).is_ok());
    }

    #[test]
    fn missing_required_field_fails() {
        let def = quest_def();
        let fields = BTreeMap::new(); // no status
        let err = validate_fields(&def, &fields).unwrap_err();
        assert!(
            matches!(err, KidurError::MissingRequiredField(ref f) if f == "status"),
            "expected MissingRequiredField(status), got: {:?}",
            err
        );
    }

    #[test]
    fn wrong_type_fails() {
        let def = quest_def();
        let mut fields = BTreeMap::new();
        fields.insert("status".to_string(), FieldValue::Enum("active".to_string()));
        // priority expects Enum but we supply Number
        fields.insert("priority".to_string(), FieldValue::Number(1.0));
        let err = validate_fields(&def, &fields).unwrap_err();
        assert!(
            matches!(err, KidurError::FieldValidation { ref field, .. } if field == "priority"),
            "expected FieldValidation on 'priority', got: {:?}",
            err
        );
    }

    #[test]
    fn invalid_enum_option_fails() {
        let def = quest_def();
        let mut fields = BTreeMap::new();
        // "deleted" is not in the options list
        fields.insert("status".to_string(), FieldValue::Enum("deleted".to_string()));
        let err = validate_fields(&def, &fields).unwrap_err();
        assert!(
            matches!(err, KidurError::FieldValidation { ref field, .. } if field == "status"),
            "expected FieldValidation on 'status', got: {:?}",
            err
        );
    }

    #[test]
    fn extra_fields_allowed() {
        let def = quest_def();
        let mut fields = BTreeMap::new();
        fields.insert("status".to_string(), FieldValue::Enum("active".to_string()));
        // "extra_field" is not in the schema — open-world, should be fine
        fields.insert(
            "extra_field".to_string(),
            FieldValue::Text("anything".to_string()),
        );
        assert!(validate_fields(&def, &fields).is_ok());
    }

    #[test]
    fn optional_reference_valid() {
        let def = quest_def();
        let mut fields = BTreeMap::new();
        fields.insert("status".to_string(), FieldValue::Enum("active".to_string()));
        fields.insert("owner".to_string(), FieldValue::Reference(NodeId::new()));
        assert!(validate_fields(&def, &fields).is_ok());
    }

    #[test]
    fn multi_select_valid() {
        let mut def = quest_def();
        def.fields.push(FieldDef {
            name: "tags".to_string(),
            field_type: FieldType::MultiSelect,
            required: false,
            options: vec!["urgent".to_string(), "exploration".to_string()],
            ref_tag: None,
        });
        let mut fields = BTreeMap::new();
        fields.insert("status".to_string(), FieldValue::Enum("active".to_string()));
        fields.insert(
            "tags".to_string(),
            FieldValue::MultiSelect(vec!["urgent".to_string()]),
        );
        assert!(validate_fields(&def, &fields).is_ok());
    }

    #[test]
    fn multi_select_invalid_option_fails() {
        let mut def = quest_def();
        def.fields.push(FieldDef {
            name: "tags".to_string(),
            field_type: FieldType::MultiSelect,
            required: false,
            options: vec!["urgent".to_string(), "exploration".to_string()],
            ref_tag: None,
        });
        let mut fields = BTreeMap::new();
        fields.insert("status".to_string(), FieldValue::Enum("active".to_string()));
        fields.insert(
            "tags".to_string(),
            FieldValue::MultiSelect(vec!["urgent".to_string(), "unknown-tag".to_string()]),
        );
        let err = validate_fields(&def, &fields).unwrap_err();
        assert!(
            matches!(err, KidurError::FieldValidation { ref field, .. } if field == "tags"),
            "expected FieldValidation on 'tags', got: {:?}",
            err
        );
    }

    #[test]
    fn optional_field_absent_passes() {
        let def = quest_def();
        let mut fields = BTreeMap::new();
        // Only supply the required field; all optional fields absent
        fields.insert("status".to_string(), FieldValue::Enum("active".to_string()));
        assert!(validate_fields(&def, &fields).is_ok());
    }

    #[test]
    fn enum_with_no_options_accepts_any_value() {
        // FieldDef with Enum type but empty options list → any string value accepted
        let def = SupertagDef {
            name: "open".to_string(),
            description: None,
            fields: vec![FieldDef {
                name: "state".to_string(),
                field_type: FieldType::Enum,
                required: true,
                options: vec![], // no constraints
                ref_tag: None,
            }],
        };
        let mut fields = BTreeMap::new();
        fields.insert("state".to_string(), FieldValue::Enum("anything".to_string()));
        assert!(validate_fields(&def, &fields).is_ok());
    }
}
