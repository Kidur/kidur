use std::collections::HashMap;
use std::path::Path;
use kidur_core::{KidurError, KidurResult, SupertagDef};

pub fn parse_supertag(toml_str: &str) -> KidurResult<SupertagDef> {
    toml::from_str(toml_str).map_err(|e| KidurError::SupertagParse(e.to_string()))
}

pub fn load_supertags_from_dir(dir: &Path) -> KidurResult<HashMap<String, SupertagDef>> {
    let mut registry = HashMap::new();
    let entries = std::fs::read_dir(dir).map_err(|e| {
        KidurError::SupertagParse(format!(
            "cannot read supertag dir {}: {}",
            dir.display(),
            e
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| KidurError::SupertagParse(e.to_string()))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let content = std::fs::read_to_string(&path).map_err(|e| {
            KidurError::SupertagParse(format!("cannot read {}: {}", path.display(), e))
        })?;
        let def = parse_supertag(&content)?;
        tracing::debug!(name = %def.name, path = %path.display(), "loaded supertag");
        registry.insert(def.name.clone(), def);
    }
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kidur_core::FieldType;

    const QUEST_TOML: &str = r#"
name = "quest"
description = "A quest, project, or initiative"

[[fields]]
name = "status"
type = "enum"
required = true
options = ["active", "completed", "paused", "blocked"]

[[fields]]
name = "owner"
type = "reference"
required = false
ref_tag = "person"
"#;

    #[test]
    fn parse_quest() {
        let def = parse_supertag(QUEST_TOML).expect("should parse");
        assert_eq!(def.name, "quest");
        assert_eq!(def.description.as_deref(), Some("A quest, project, or initiative"));
        assert_eq!(def.fields.len(), 2);
        assert_eq!(def.fields[0].name, "status");
        assert_eq!(def.fields[0].field_type, FieldType::Enum);
        assert!(def.fields[0].required);
        assert_eq!(def.fields[0].options, vec!["active", "completed", "paused", "blocked"]);
        assert_eq!(def.fields[1].name, "owner");
        assert_eq!(def.fields[1].field_type, FieldType::Reference);
        assert!(!def.fields[1].required);
        assert_eq!(def.fields[1].ref_tag.as_deref(), Some("person"));
    }

    #[test]
    fn parse_minimal() {
        let def = parse_supertag("name = \"empty\"\n").expect("should parse minimal");
        assert_eq!(def.name, "empty");
        assert!(def.fields.is_empty());
        assert!(def.description.is_none());
    }

    #[test]
    fn parse_invalid() {
        let result = parse_supertag("this is not valid toml ][[[");
        assert!(result.is_err(), "garbage TOML should return error");
    }

    #[test]
    fn load_from_dir() {
        use std::io::Write;

        let dir = tempfile::tempdir().expect("tempdir");
        let toml_path = dir.path().join("quest.toml");
        let mut f = std::fs::File::create(&toml_path).unwrap();
        write!(f, "{}", QUEST_TOML).unwrap();

        // add a non-toml file that should be ignored
        std::fs::write(dir.path().join("notes.txt"), "ignore me").unwrap();

        let registry = load_supertags_from_dir(dir.path()).expect("load should succeed");
        assert_eq!(registry.len(), 1);
        let quest = registry.get("quest").expect("quest should be present");
        assert_eq!(quest.fields.len(), 2);
    }
}
