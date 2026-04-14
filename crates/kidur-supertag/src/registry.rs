use std::collections::HashMap;
use std::path::Path;
use kidur_core::{KidurError, KidurResult, Node, SupertagDef};
use crate::loader::load_supertags_from_dir;
use crate::validator::validate_fields;

#[derive(Debug, Clone)]
pub struct SupertagRegistry {
    defs: HashMap<String, SupertagDef>,
}

impl SupertagRegistry {
    pub fn from_dir(dir: &Path) -> KidurResult<Self> {
        let defs = load_supertags_from_dir(dir)?;
        tracing::info!(count = defs.len(), "supertag registry loaded");
        Ok(Self { defs })
    }

    pub fn empty() -> Self {
        Self {
            defs: HashMap::new(),
        }
    }

    pub fn register(&mut self, def: SupertagDef) {
        self.defs.insert(def.name.clone(), def);
    }

    pub fn get(&self, name: &str) -> Option<&SupertagDef> {
        self.defs.get(name)
    }

    pub fn names(&self) -> Vec<&str> {
        self.defs.keys().map(|s| s.as_str()).collect()
    }

    pub fn validate_node(&self, node: &Node) -> KidurResult<()> {
        let tag_name = match &node.supertag {
            Some(t) => t,
            None => return Ok(()),
        };
        let def = self
            .get(tag_name)
            .ok_or_else(|| KidurError::UnknownSupertag(tag_name.clone()))?;
        validate_fields(def, &node.fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kidur_core::{FieldDef, FieldType, FieldValue, Node, SupertagDef};

    fn quest_registry() -> SupertagRegistry {
        let mut reg = SupertagRegistry::empty();
        reg.register(SupertagDef {
            name: "quest".to_string(),
            description: None,
            fields: vec![FieldDef {
                name: "status".to_string(),
                field_type: FieldType::Enum,
                required: true,
                options: vec!["active".to_string(), "completed".to_string()],
                ref_tag: None,
            }],
        });
        reg
    }

    #[test]
    fn validate_node_with_valid_tag() {
        let reg = quest_registry();
        let node = Node::new("My quest")
            .with_supertag("quest")
            .with_field("status", FieldValue::Enum("active".to_string()));
        assert!(reg.validate_node(&node).is_ok());
    }

    #[test]
    fn validate_node_missing_required() {
        let reg = quest_registry();
        // No "status" field — required
        let node = Node::new("My quest").with_supertag("quest");
        let err = reg.validate_node(&node).unwrap_err();
        assert!(
            matches!(err, KidurError::MissingRequiredField(ref f) if f == "status"),
            "expected MissingRequiredField(status), got: {:?}",
            err
        );
    }

    #[test]
    fn validate_node_unknown_tag() {
        let reg = quest_registry();
        let node = Node::new("My node").with_supertag("nonexistent");
        let err = reg.validate_node(&node).unwrap_err();
        assert!(
            matches!(err, KidurError::UnknownSupertag(ref t) if t == "nonexistent"),
            "expected UnknownSupertag(nonexistent), got: {:?}",
            err
        );
    }

    #[test]
    fn validate_node_no_tag() {
        let reg = quest_registry();
        // Node with no supertag → always passes
        let node = Node::new("plain node");
        assert!(reg.validate_node(&node).is_ok());
    }

    #[test]
    fn names_list() {
        let reg = quest_registry();
        let names = reg.names();
        assert_eq!(names.len(), 1);
        assert!(names.contains(&"quest"));
    }

    #[test]
    fn from_dir_loads_toml() {
        use std::io::Write;

        let dir = tempfile::tempdir().expect("tempdir");
        let content = r#"
name = "task"
[[fields]]
name = "done"
type = "bool"
required = true
"#;
        let mut f = std::fs::File::create(dir.path().join("task.toml")).unwrap();
        write!(f, "{}", content).unwrap();

        let reg = SupertagRegistry::from_dir(dir.path()).expect("from_dir");
        assert!(reg.get("task").is_some());
        assert_eq!(reg.names().len(), 1);
    }

    #[test]
    fn register_and_get() {
        let mut reg = SupertagRegistry::empty();
        assert!(reg.get("person").is_none());
        reg.register(SupertagDef::new("person"));
        assert!(reg.get("person").is_some());
    }
}
