//! SQL validation utilities

use crate::Error;

/// Keywords that indicate write operations
const WRITE_KEYWORDS: &[&str] = &[
    "INSERT", "UPDATE", "DELETE", "DROP", "CREATE", "ALTER", "TRUNCATE", "MERGE", "UPSERT", "CALL",
    "EXEC", "EXECUTE",
];

/// Maximum length for SQL identifiers (HANA limit is 127)
const MAX_IDENTIFIER_LENGTH: usize = 127;

/// Validate SQL identifier (schema/table name) to prevent injection
pub fn is_valid_identifier(name: &str) -> bool {
    if name.is_empty() || name.len() > MAX_IDENTIFIER_LENGTH {
        return false;
    }

    let first_char = name.chars().next().unwrap();
    if first_char.is_ascii_digit() {
        return false;
    }

    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '#')
}

/// Validate identifier and return error if invalid
pub fn validate_identifier(name: &str, context: &str) -> Result<(), Error> {
    if is_valid_identifier(name) {
        Ok(())
    } else {
        Err(Error::Config(format!(
            "Invalid {context}: '{name}'. \
             Must be 1-127 alphanumeric characters (a-z, A-Z, 0-9, _, $, #), \
             cannot start with a digit."
        )))
    }
}

/// Validate SQL query for read-only mode
///
/// Blocks INSERT, UPDATE, DELETE, DROP, CREATE, ALTER, TRUNCATE,
/// MERGE, UPSERT, CALL, EXEC, EXECUTE statements.
/// Handles comments and CTE (WITH) clauses.
pub fn validate_read_only_sql(sql: &str) -> Result<(), Error> {
    let cleaned = strip_sql_comments(sql);
    let sql_upper = cleaned.trim().to_uppercase();

    if sql_upper.is_empty() {
        return Ok(());
    }

    // Check each statement for write operations
    for statement in sql_upper.split(';') {
        let trimmed = statement.trim();
        if trimmed.is_empty() {
            continue;
        }

        if contains_write_operation(trimmed) {
            return Err(Error::read_only_violation(
                "DML/DDL operations not allowed in read-only mode".into(),
            ));
        }
    }

    Ok(())
}

/// Strip SQL comments (both -- and /* */ style)
fn strip_sql_comments(sql: &str) -> String {
    let mut result = String::with_capacity(sql.len());
    let mut chars = sql.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while let Some(c) = chars.next() {
        // Track string literals to avoid stripping inside them
        if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            result.push(c);
            continue;
        }
        if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            result.push(c);
            continue;
        }

        if in_single_quote || in_double_quote {
            result.push(c);
            continue;
        }

        // Handle -- style comments
        if c == '-' && chars.peek() == Some(&'-') {
            chars.next();
            for ch in chars.by_ref() {
                if ch == '\n' {
                    result.push(' ');
                    break;
                }
            }
            continue;
        }

        // Handle /* */ style comments
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(ch) = chars.next() {
                if ch == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    result.push(' ');
                    break;
                }
            }
            continue;
        }

        result.push(c);
    }

    result
}

/// Check if SQL contains any write operation keyword
fn contains_write_operation(sql: &str) -> bool {
    // Handle WITH clauses by checking what follows
    let sql_to_check = if sql.starts_with("WITH ") || sql.starts_with("WITH\t") {
        // Find the actual operation after WITH clause(s)
        find_main_operation(sql)
    } else {
        sql.to_string()
    };

    for keyword in WRITE_KEYWORDS {
        // Check if statement starts with keyword
        if sql_to_check.starts_with(keyword)
            && sql_to_check
                .chars()
                .nth(keyword.len())
                .is_some_and(|c| c.is_whitespace() || c == '(')
        {
            return true;
        }

        // Check for keyword after WITH clause or in subqueries
        let patterns = [
            format!(" {keyword} "),
            format!(" {keyword}("),
            format!("\t{keyword} "),
            format!("\t{keyword}("),
            format!("\n{keyword} "),
            format!("\n{keyword}("),
        ];

        for pattern in &patterns {
            if sql.contains(pattern) {
                return true;
            }
        }
    }

    false
}

/// Find the main SQL operation after WITH clause(s)
fn find_main_operation(sql: &str) -> String {
    let mut depth: u32 = 0;

    for (pos, c) in sql.chars().enumerate() {
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth = depth.saturating_sub(1);
        }

        // When at depth 0 and we find SELECT/INSERT/UPDATE/DELETE after WITH
        if depth == 0 && c.is_whitespace() {
            // pos is 0-indexed, but we want the character after current one
            let remaining = &sql[pos + 1..].trim_start();
            for keyword in WRITE_KEYWORDS.iter().chain(&["SELECT"]) {
                if remaining.starts_with(keyword)
                    && remaining
                        .chars()
                        .nth(keyword.len())
                        .is_some_and(|ch| ch.is_whitespace() || ch == '(')
                {
                    return remaining.to_string();
                }
            }
        }
    }

    sql.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Identifier validation tests
    #[test]
    fn test_valid_identifier_simple() {
        assert!(is_valid_identifier("USERS"));
        assert!(is_valid_identifier("my_table"));
        assert!(is_valid_identifier("Schema1"));
    }

    #[test]
    fn test_valid_identifier_special_chars() {
        assert!(is_valid_identifier("$system"));
        assert!(is_valid_identifier("#temp"));
        assert!(is_valid_identifier("table_$1"));
    }

    #[test]
    fn test_invalid_identifier_empty() {
        assert!(!is_valid_identifier(""));
    }

    #[test]
    fn test_invalid_identifier_starts_with_digit() {
        assert!(!is_valid_identifier("1table"));
        assert!(!is_valid_identifier("123"));
    }

    #[test]
    fn test_invalid_identifier_special_chars() {
        assert!(!is_valid_identifier("table-name"));
        assert!(!is_valid_identifier("table.name"));
        assert!(!is_valid_identifier("table name"));
        assert!(!is_valid_identifier("table;drop"));
        assert!(!is_valid_identifier("table'--"));
    }

    #[test]
    fn test_invalid_identifier_too_long() {
        let long_name = "a".repeat(128);
        assert!(!is_valid_identifier(&long_name));
    }

    #[test]
    fn test_validate_identifier_ok() {
        assert!(validate_identifier("users", "table name").is_ok());
    }

    #[test]
    fn test_validate_identifier_error() {
        let result = validate_identifier("user;--", "table name");
        assert!(result.is_err());
    }

    // Read-only SQL validation tests
    #[test]
    fn test_allows_select() {
        assert!(validate_read_only_sql("SELECT * FROM users").is_ok());
        assert!(validate_read_only_sql("  select id from t").is_ok());
    }

    #[test]
    fn test_allows_with() {
        assert!(validate_read_only_sql("WITH cte AS (SELECT 1) SELECT * FROM cte").is_ok());
    }

    #[test]
    fn test_allows_explain() {
        assert!(validate_read_only_sql("EXPLAIN PLAN FOR SELECT * FROM t").is_ok());
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

    #[test]
    fn test_blocks_merge() {
        assert!(validate_read_only_sql("MERGE INTO t USING s ON t.id = s.id").is_err());
    }

    #[test]
    fn test_blocks_upsert() {
        assert!(validate_read_only_sql("UPSERT t VALUES (1, 'a')").is_err());
    }

    #[test]
    fn test_blocks_call() {
        assert!(validate_read_only_sql("CALL my_procedure()").is_err());
    }

    #[test]
    fn test_blocks_exec() {
        assert!(validate_read_only_sql("EXEC my_procedure").is_err());
    }

    #[test]
    fn test_blocks_execute() {
        assert!(validate_read_only_sql("EXECUTE my_procedure").is_err());
    }

    // New tests for comment and CTE bypass prevention
    #[test]
    fn test_blocks_insert_with_leading_comment() {
        assert!(validate_read_only_sql("-- comment\nINSERT INTO users VALUES (1)").is_err());
    }

    #[test]
    fn test_blocks_insert_with_block_comment() {
        assert!(validate_read_only_sql("/* comment */ INSERT INTO users VALUES (1)").is_err());
    }

    #[test]
    fn test_blocks_with_cte_insert() {
        assert!(
            validate_read_only_sql("WITH cte AS (SELECT 1) INSERT INTO users SELECT * FROM cte")
                .is_err()
        );
    }

    #[test]
    fn test_blocks_with_cte_delete() {
        assert!(
            validate_read_only_sql(
                "WITH cte AS (SELECT 1) DELETE FROM users WHERE id IN (SELECT * FROM cte)"
            )
            .is_err()
        );
    }

    #[test]
    fn test_blocks_with_cte_update() {
        assert!(
            validate_read_only_sql(
                "WITH cte AS (SELECT 1) UPDATE users SET x = 1 WHERE id IN (SELECT * FROM cte)"
            )
            .is_err()
        );
    }

    #[test]
    fn test_allows_select_with_comment() {
        assert!(validate_read_only_sql("-- select data\nSELECT * FROM users").is_ok());
    }

    #[test]
    fn test_allows_nested_cte_select() {
        let sql = "WITH a AS (SELECT 1), b AS (SELECT * FROM a) SELECT * FROM b";
        assert!(validate_read_only_sql(sql).is_ok());
    }

    #[test]
    fn test_strip_comments_preserves_string_literals() {
        let sql = "SELECT '--not a comment' FROM t";
        let cleaned = strip_sql_comments(sql);
        assert!(cleaned.contains("'--not a comment'"));
    }

    #[test]
    fn test_multiple_statements_blocks_any_write() {
        assert!(validate_read_only_sql("SELECT 1; INSERT INTO t VALUES (1)").is_err());
    }

    #[test]
    fn test_empty_sql_allowed() {
        assert!(validate_read_only_sql("").is_ok());
        assert!(validate_read_only_sql("   ").is_ok());
    }
}
