//! SQL validation utilities

use crate::Error;

/// Validate SQL query for read-only mode
///
/// Blocks INSERT, UPDATE, DELETE, DROP, CREATE, ALTER, TRUNCATE statements
pub fn validate_read_only_sql(sql: &str) -> Result<(), Error> {
    let sql_upper = sql.trim().to_uppercase();

    if sql_upper.starts_with("INSERT")
        || sql_upper.starts_with("UPDATE")
        || sql_upper.starts_with("DELETE")
        || sql_upper.starts_with("DROP")
        || sql_upper.starts_with("CREATE")
        || sql_upper.starts_with("ALTER")
        || sql_upper.starts_with("TRUNCATE")
    {
        return Err(Error::read_only_violation(format!(
            "DML/DDL not allowed, query starts with: {}",
            &sql[..sql.len().min(30)]
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allows_select() {
        assert!(validate_read_only_sql("SELECT * FROM users").is_ok());
        assert!(validate_read_only_sql("  select id from t").is_ok());
    }

    #[test]
    fn test_blocks_insert() {
        assert!(validate_read_only_sql("INSERT INTO users VALUES (1)").is_err());
    }

    #[test]
    fn test_blocks_update() {
        assert!(validate_read_only_sql("UPDATE users SET name = 'x'").is_err());
    }

    #[test]
    fn test_blocks_delete() {
        assert!(validate_read_only_sql("DELETE FROM users").is_err());
    }

    #[test]
    fn test_blocks_drop() {
        assert!(validate_read_only_sql("DROP TABLE users").is_err());
    }

    #[test]
    fn test_blocks_create() {
        assert!(validate_read_only_sql("CREATE TABLE users (id INT)").is_err());
    }

    #[test]
    fn test_blocks_alter() {
        assert!(validate_read_only_sql("ALTER TABLE users ADD COLUMN x").is_err());
    }

    #[test]
    fn test_blocks_truncate() {
        assert!(validate_read_only_sql("TRUNCATE TABLE users").is_err());
    }
}
