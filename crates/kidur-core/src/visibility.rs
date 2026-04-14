use std::fmt;
use serde::{Deserialize, Serialize};

/// Controls who can see a node.
///
/// Default is `Private`, meaning the node is only visible to its creator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// Only the owner can see the node.
    Private,
    /// Shared with specific collaborators.
    Shared,
    /// Visible to everyone.
    Public,
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::Private
    }
}

impl Visibility {
    /// Return a static string representation of the variant.
    pub fn as_str(&self) -> &'static str {
        match self {
            Visibility::Private => "private",
            Visibility::Shared => "shared",
            Visibility::Public => "public",
        }
    }
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_private() {
        assert_eq!(Visibility::default(), Visibility::Private);
    }

    #[test]
    fn test_json_roundtrip() {
        for v in [Visibility::Private, Visibility::Shared, Visibility::Public] {
            let json = serde_json::to_string(&v).unwrap();
            let restored: Visibility = serde_json::from_str(&json).unwrap();
            assert_eq!(v, restored);
        }
    }

    #[test]
    fn test_as_str() {
        assert_eq!(Visibility::Private.as_str(), "private");
        assert_eq!(Visibility::Shared.as_str(), "shared");
        assert_eq!(Visibility::Public.as_str(), "public");
    }

    #[test]
    fn test_display() {
        assert_eq!(Visibility::Public.to_string(), "public");
    }

    #[test]
    fn test_snake_case_serde() {
        let json = serde_json::to_string(&Visibility::Public).unwrap();
        assert_eq!(json, "\"public\"");
    }
}
