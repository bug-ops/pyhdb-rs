//! Role-based access control

use super::claims::AuthenticatedUser;
use super::config::RbacConfig;
use super::error::{AuthError, Result};

/// Permission levels (hierarchical)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Permission {
    None,
    Read,
    Execute,
    Write,
    Admin,
}

impl Permission {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Read => "read",
            Self::Execute => "execute",
            Self::Write => "write",
            Self::Admin => "admin",
        }
    }
}

/// RBAC enforcer
#[derive(Debug, Clone)]
pub struct RbacEnforcer {
    config: RbacConfig,
}

impl RbacEnforcer {
    #[must_use]
    pub const fn new(config: RbacConfig) -> Self {
        Self { config }
    }

    /// Check if user has required permission
    pub fn check(&self, user: &AuthenticatedUser, required: Permission) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let user_permission = self.highest_permission(user);

        if user_permission >= required {
            Ok(())
        } else {
            tracing::warn!(
                user = %user.sub,
                required = required.as_str(),
                actual = user_permission.as_str(),
                "RBAC check failed"
            );
            Err(AuthError::InsufficientPermissions)
        }
    }

    fn highest_permission(&self, user: &AuthenticatedUser) -> Permission {
        if let Some(ref admin_role) = self.config.admin_role
            && user.has_role(admin_role)
        {
            return Permission::Admin;
        }

        if let Some(ref write_role) = self.config.write_role
            && user.has_role(write_role)
        {
            return Permission::Write;
        }

        if let Some(ref execute_role) = self.config.execute_role
            && user.has_role(execute_role)
        {
            return Permission::Execute;
        }

        if let Some(ref read_role) = self.config.read_role
            && user.has_role(read_role)
        {
            return Permission::Read;
        }

        Permission::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_user(roles: Vec<&str>) -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "user123".to_string(),
            email: None,
            name: None,
            tenant_id: None,
            tenant_schema: None,
            roles: roles.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn test_permission_ordering() {
        assert!(Permission::Admin > Permission::Write);
        assert!(Permission::Write > Permission::Execute);
        assert!(Permission::Execute > Permission::Read);
        assert!(Permission::Read > Permission::None);
    }

    #[test]
    fn test_rbac_disabled() {
        let enforcer = RbacEnforcer::new(RbacConfig {
            enabled: false,
            ..Default::default()
        });
        let user = create_user(vec![]);
        assert!(enforcer.check(&user, Permission::Admin).is_ok());
    }

    #[test]
    fn test_rbac_admin_role() {
        let enforcer = RbacEnforcer::new(RbacConfig {
            enabled: true,
            admin_role: Some("admin".to_string()),
            ..Default::default()
        });
        let user = create_user(vec!["admin"]);
        assert!(enforcer.check(&user, Permission::Admin).is_ok());
        assert!(enforcer.check(&user, Permission::Write).is_ok());
        assert!(enforcer.check(&user, Permission::Read).is_ok());
    }

    #[test]
    fn test_rbac_write_role() {
        let enforcer = RbacEnforcer::new(RbacConfig {
            enabled: true,
            write_role: Some("writer".to_string()),
            ..Default::default()
        });
        let user = create_user(vec!["writer"]);
        assert!(enforcer.check(&user, Permission::Write).is_ok());
        assert!(enforcer.check(&user, Permission::Read).is_ok());
        assert!(enforcer.check(&user, Permission::Admin).is_err());
    }

    #[test]
    fn test_rbac_read_role() {
        let enforcer = RbacEnforcer::new(RbacConfig {
            enabled: true,
            read_role: Some("reader".to_string()),
            ..Default::default()
        });
        let user = create_user(vec!["reader"]);
        assert!(enforcer.check(&user, Permission::Read).is_ok());
        assert!(enforcer.check(&user, Permission::Write).is_err());
    }

    #[test]
    fn test_rbac_no_matching_role() {
        let enforcer = RbacEnforcer::new(RbacConfig {
            enabled: true,
            read_role: Some("reader".to_string()),
            ..Default::default()
        });
        let user = create_user(vec!["other"]);
        assert!(enforcer.check(&user, Permission::Read).is_err());
    }

    #[test]
    fn test_permission_as_str() {
        assert_eq!(Permission::None.as_str(), "none");
        assert_eq!(Permission::Read.as_str(), "read");
        assert_eq!(Permission::Execute.as_str(), "execute");
        assert_eq!(Permission::Write.as_str(), "write");
        assert_eq!(Permission::Admin.as_str(), "admin");
    }
}
