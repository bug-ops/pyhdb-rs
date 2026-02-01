//! Stored procedure configuration

use std::num::NonZeroU32;

/// Default maximum result sets from procedures (10)
const DEFAULT_MAX_RESULT_SETS: NonZeroU32 = NonZeroU32::new(10).unwrap();

/// Default maximum rows per result set (1000)
const DEFAULT_MAX_ROWS_PER_RESULT_SET: NonZeroU32 = NonZeroU32::new(1000).unwrap();

/// Stored procedure execution configuration.
///
/// # Security Note
///
/// When `allow_procedures` is enabled, stored procedures can execute any DML
/// operations internally, regardless of the `allow_dml` setting. This means:
///
/// - `allow_dml=false, allow_procedures=true`: Direct DML blocked, but procedures can execute DML
/// - To fully prevent DML modifications, ensure both flags are disabled
///
/// Consider using `require_confirmation=true` (default) to prompt users before procedure execution.
#[derive(Debug, Clone, Copy)]
pub struct ProcedureConfig {
    /// Allow stored procedure execution.
    ///
    /// **Warning:** When enabled, procedures can execute DML operations internally
    /// even if `allow_dml=false`. Default: false
    pub allow_procedures: bool,
    /// Require user confirmation before procedure execution. Default: true
    pub require_confirmation: bool,
    /// Maximum result sets to return (None = unlimited). Default: 10
    pub max_result_sets: Option<NonZeroU32>,
    /// Maximum rows per result set (None = unlimited). Default: 1000
    pub max_rows_per_result_set: Option<NonZeroU32>,
}

impl Default for ProcedureConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcedureConfig {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            allow_procedures: false,
            require_confirmation: true,
            max_result_sets: Some(DEFAULT_MAX_RESULT_SETS),
            max_rows_per_result_set: Some(DEFAULT_MAX_ROWS_PER_RESULT_SET),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_procedure_config_default() {
        let config = ProcedureConfig::default();
        assert!(!config.allow_procedures);
        assert!(config.require_confirmation);
        assert_eq!(config.max_result_sets, NonZeroU32::new(10));
        assert_eq!(config.max_rows_per_result_set, NonZeroU32::new(1000));
    }

    #[test]
    fn test_procedure_config_new() {
        let config = ProcedureConfig::new();
        assert!(!config.allow_procedures);
        assert!(config.require_confirmation);
        assert_eq!(config.max_result_sets, NonZeroU32::new(10));
        assert_eq!(config.max_rows_per_result_set, NonZeroU32::new(1000));
    }

    #[test]
    fn test_procedure_config_new_and_default_are_identical() {
        let from_new = ProcedureConfig::new();
        let from_default = ProcedureConfig::default();

        assert_eq!(from_new.allow_procedures, from_default.allow_procedures);
        assert_eq!(
            from_new.require_confirmation,
            from_default.require_confirmation
        );
        assert_eq!(from_new.max_result_sets, from_default.max_result_sets);
        assert_eq!(
            from_new.max_rows_per_result_set,
            from_default.max_rows_per_result_set
        );
    }
}
