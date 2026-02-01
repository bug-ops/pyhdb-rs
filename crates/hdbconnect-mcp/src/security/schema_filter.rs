//! Schema access filtering

use std::collections::HashSet;

use crate::Error;

/// Schema access filter configuration
#[derive(Debug, Clone, Default)]
pub enum SchemaFilter {
    /// Allow access to all schemas (default, backward compatible)
    #[default]
    AllowAll,
    /// Only allow access to specified schemas
    Whitelist(HashSet<String>),
    /// Deny access to specified schemas, allow all others
    Blacklist(HashSet<String>),
}

impl SchemaFilter {
    /// Check if access to a schema is allowed
    pub fn is_allowed(&self, schema: &str) -> bool {
        let schema_upper = schema.to_uppercase();
        match self {
            Self::AllowAll => true,
            Self::Whitelist(allowed) => allowed.contains(&schema_upper),
            Self::Blacklist(denied) => !denied.contains(&schema_upper),
        }
    }

    /// Validate schema access, returning an error if denied
    pub fn validate(&self, schema: &str) -> Result<(), Error> {
        if self.is_allowed(schema) {
            Ok(())
        } else {
            Err(Error::SchemaAccessDenied(schema.to_string()))
        }
    }

    /// Create a filter from configuration strings
    pub fn from_config(mode: &str, schemas: &[String]) -> Result<Self, Error> {
        let schemas_set: HashSet<String> = schemas.iter().map(|s| s.to_uppercase()).collect();

        match mode.to_lowercase().as_str() {
            "whitelist" | "allow" => {
                if schemas_set.is_empty() {
                    return Err(Error::Config(
                        "Whitelist mode requires at least one schema".into(),
                    ));
                }
                Ok(Self::Whitelist(schemas_set))
            }
            "blacklist" | "deny" => Ok(Self::Blacklist(schemas_set)),
            "none" | "all" | "" => Ok(Self::AllowAll),
            _ => Err(Error::Config(format!(
                "Invalid schema filter mode: {mode}. Use 'whitelist', 'blacklist', or 'none'"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_all() {
        let filter = SchemaFilter::AllowAll;
        assert!(filter.is_allowed("ANY_SCHEMA"));
        assert!(filter.is_allowed("SYS"));
        assert!(filter.is_allowed("system"));
    }

    #[test]
    fn test_whitelist() {
        let allowed: HashSet<String> = ["ALLOWED_SCHEMA", "APP"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let filter = SchemaFilter::Whitelist(allowed);

        assert!(filter.is_allowed("ALLOWED_SCHEMA"));
        assert!(filter.is_allowed("allowed_schema")); // case insensitive
        assert!(filter.is_allowed("APP"));
        assert!(!filter.is_allowed("OTHER"));
        assert!(!filter.is_allowed("SYS"));
    }

    #[test]
    fn test_blacklist() {
        let denied: HashSet<String> = ["SYS", "SYSTEM"].iter().map(|s| (*s).to_string()).collect();
        let filter = SchemaFilter::Blacklist(denied);

        assert!(!filter.is_allowed("SYS"));
        assert!(!filter.is_allowed("sys")); // case insensitive
        assert!(!filter.is_allowed("SYSTEM"));
        assert!(filter.is_allowed("APP"));
        assert!(filter.is_allowed("MY_SCHEMA"));
    }

    #[test]
    fn test_from_config_whitelist() {
        let schemas = vec!["SCHEMA1".to_string(), "SCHEMA2".to_string()];
        let filter = SchemaFilter::from_config("whitelist", &schemas).unwrap();

        assert!(filter.is_allowed("SCHEMA1"));
        assert!(filter.is_allowed("schema2"));
        assert!(!filter.is_allowed("OTHER"));
    }

    #[test]
    fn test_from_config_blacklist() {
        let schemas = vec!["SYS".to_string()];
        let filter = SchemaFilter::from_config("blacklist", &schemas).unwrap();

        assert!(!filter.is_allowed("SYS"));
        assert!(filter.is_allowed("APP"));
    }

    #[test]
    fn test_from_config_none() {
        let filter = SchemaFilter::from_config("none", &[]).unwrap();
        assert!(filter.is_allowed("ANY"));
    }

    #[test]
    fn test_from_config_whitelist_requires_schemas() {
        let result = SchemaFilter::from_config("whitelist", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_config_invalid_mode() {
        let result = SchemaFilter::from_config("invalid", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate() {
        let denied: HashSet<String> = ["SYS"].iter().map(|s| (*s).to_string()).collect();
        let filter = SchemaFilter::Blacklist(denied);

        assert!(filter.validate("APP").is_ok());
        assert!(filter.validate("SYS").is_err());
    }
}
