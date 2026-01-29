//! TLS configuration for secure SAP HANA connections.
//!
//! Provides `PyTlsConfig`, a Python-facing wrapper for TLS certificate configuration.
//! Supports all certificate sources from `hdbconnect::ServerCerts`.
//!
//! # Example
//!
//! ```python
//! from pyhdb_rs import TlsConfig, ConnectionBuilder
//!
//! # TLS with certificates from directory
//! tls = TlsConfig.from_directory("/path/to/certs")
//! conn = ConnectionBuilder().host("...").credentials("user", "pass").tls(tls).build()
//!
//! # TLS with system root certificates
//! tls = TlsConfig.with_system_roots()
//!
//! # Development only: TLS without server verification
//! tls = TlsConfig.insecure()
//! ```

use pyo3::prelude::*;
use pyo3::types::PyType;

use crate::utils::apply_tls_to_sync_builder;

/// Internal TLS configuration variant.
#[derive(Debug, Clone)]
pub(crate) enum TlsConfigInner {
    /// Load certificates from PEM files in a directory.
    Directory(String),
    /// Load certificate from an environment variable.
    Environment(String),
    /// Use certificate content directly from a PEM string.
    Direct(String),
    /// Use system root certificates (mkcert.org roots).
    RootCertificates,
    /// TLS without server verification (insecure, development only).
    Insecure,
}

/// TLS configuration for secure connections.
///
/// Choose ONE of the factory methods to specify the certificate source.
/// This class is immutable (frozen) once created.
///
/// # Example
///
/// ```python
/// # Load certificates from a directory
/// tls = TlsConfig.from_directory("/etc/hana/certs")
///
/// # Load from environment variable
/// tls = TlsConfig.from_environment("HANA_CA_CERT")
///
/// # Use certificate content directly
/// with open("ca.pem") as f:
///     tls = TlsConfig.from_certificate(f.read())
///
/// # Use system root certificates
/// tls = TlsConfig.with_system_roots()
///
/// # Development only: skip verification (INSECURE)
/// tls = TlsConfig.insecure()
/// ```
#[pyclass(name = "TlsConfig", module = "pyhdb_rs._core", frozen)]
#[derive(Debug, Clone)]
pub struct PyTlsConfig {
    pub(crate) inner: TlsConfigInner,
}

impl PyTlsConfig {
    /// Apply TLS configuration to a mutable `ConnectParamsBuilder`.
    pub(crate) fn apply_to_builder_mut(&self, builder: &mut hdbconnect::ConnectParamsBuilder) {
        apply_tls_to_sync_builder(&self.inner, builder);
    }
}

#[pymethods]
impl PyTlsConfig {
    /// Load server certificates from PEM files in a directory.
    ///
    /// The directory should contain one or more `.pem` files with CA certificates.
    ///
    /// Args:
    ///     path: Path to directory containing PEM certificate files.
    ///
    /// Returns:
    ///     `TlsConfig` configured to use certificates from the directory.
    ///
    /// Example:
    ///     ```python
    ///     tls = TlsConfig.from_directory("/etc/hana/certs")
    ///     ```
    #[classmethod]
    #[pyo3(text_signature = "(cls, path)")]
    fn from_directory(_cls: &Bound<'_, PyType>, path: &str) -> Self {
        Self {
            inner: TlsConfigInner::Directory(path.to_string()),
        }
    }

    /// Load server certificate from an environment variable.
    ///
    /// The environment variable should contain the PEM-encoded certificate content.
    ///
    /// Args:
    ///     `env_var`: Name of environment variable containing PEM certificate.
    ///
    /// Returns:
    ///     `TlsConfig` configured to read certificate from environment.
    ///
    /// Example:
    ///     ```python
    ///     # Set HANA_CA_CERT="-----BEGIN CERTIFICATE-----\n..."
    ///     tls = TlsConfig.from_environment("HANA_CA_CERT")
    ///     ```
    #[classmethod]
    #[pyo3(text_signature = "(cls, env_var)")]
    fn from_environment(_cls: &Bound<'_, PyType>, env_var: &str) -> Self {
        Self {
            inner: TlsConfigInner::Environment(env_var.to_string()),
        }
    }

    /// Use certificate directly from a PEM string.
    ///
    /// Args:
    ///     `pem_content`: PEM-encoded certificate content.
    ///
    /// Returns:
    ///     `TlsConfig` configured with the provided certificate.
    ///
    /// Example:
    ///     ```python
    ///     with open("ca.pem") as f:
    ///         tls = TlsConfig.from_certificate(f.read())
    ///     ```
    #[classmethod]
    #[pyo3(text_signature = "(cls, pem_content)")]
    fn from_certificate(_cls: &Bound<'_, PyType>, pem_content: &str) -> Self {
        Self {
            inner: TlsConfigInner::Direct(pem_content.to_string()),
        }
    }

    /// Use system root certificates (webpki-roots / mkcert.org).
    ///
    /// This uses the bundled Mozilla root certificates, suitable for
    /// connections to HANA instances with certificates signed by
    /// well-known certificate authorities.
    ///
    /// Returns:
    ///     `TlsConfig` configured to use system root certificates.
    ///
    /// Example:
    ///     ```python
    ///     tls = TlsConfig.with_system_roots()
    ///     ```
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    #[allow(clippy::missing_const_for_fn)]
    fn with_system_roots(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: TlsConfigInner::RootCertificates,
        }
    }

    /// TLS without server certificate verification (INSECURE).
    ///
    /// **Warning:** This disables server certificate verification and should
    /// only be used for development/testing with self-signed certificates.
    /// Never use in production.
    ///
    /// Returns:
    ///     `TlsConfig` configured for insecure TLS.
    ///
    /// Example:
    ///     ```python
    ///     # Development only!
    ///     tls = TlsConfig.insecure()
    ///     ```
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    #[allow(clippy::missing_const_for_fn)]
    fn insecure(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: TlsConfigInner::Insecure,
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            TlsConfigInner::Directory(path) => {
                format!("TlsConfig.from_directory({path:?})")
            }
            TlsConfigInner::Environment(var) => {
                format!("TlsConfig.from_environment({var:?})")
            }
            TlsConfigInner::Direct(_) => "TlsConfig.from_certificate(<pem>)".to_string(),
            TlsConfigInner::RootCertificates => "TlsConfig.with_system_roots()".to_string(),
            TlsConfigInner::Insecure => "TlsConfig.insecure()".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_directory() {
        let config = PyTlsConfig {
            inner: TlsConfigInner::Directory("/path/to/certs".to_string()),
        };
        assert!(matches!(config.inner, TlsConfigInner::Directory(_)));
    }

    #[test]
    fn test_tls_config_environment() {
        let config = PyTlsConfig {
            inner: TlsConfigInner::Environment("HANA_CA_CERT".to_string()),
        };
        assert!(matches!(config.inner, TlsConfigInner::Environment(_)));
    }

    #[test]
    fn test_tls_config_direct() {
        let pem = "-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----";
        let config = PyTlsConfig {
            inner: TlsConfigInner::Direct(pem.to_string()),
        };
        assert!(matches!(config.inner, TlsConfigInner::Direct(_)));
    }

    #[test]
    fn test_tls_config_root_certificates() {
        let config = PyTlsConfig {
            inner: TlsConfigInner::RootCertificates,
        };
        assert!(matches!(config.inner, TlsConfigInner::RootCertificates));
    }

    #[test]
    fn test_tls_config_insecure() {
        let config = PyTlsConfig {
            inner: TlsConfigInner::Insecure,
        };
        assert!(matches!(config.inner, TlsConfigInner::Insecure));
    }

    #[test]
    fn test_tls_config_clone() {
        let config = PyTlsConfig {
            inner: TlsConfigInner::Directory("/path".to_string()),
        };
        let cloned = config.clone();
        assert!(matches!(cloned.inner, TlsConfigInner::Directory(_)));
    }

    #[test]
    fn test_tls_config_debug() {
        let config = PyTlsConfig {
            inner: TlsConfigInner::RootCertificates,
        };
        let debug_str = format!("{config:?}");
        assert!(debug_str.contains("RootCertificates"));
    }

    #[test]
    fn test_tls_config_repr_directory() {
        let config = PyTlsConfig {
            inner: TlsConfigInner::Directory("/path/to/certs".to_string()),
        };
        let repr = config.__repr__();
        assert!(repr.contains("from_directory"));
        assert!(repr.contains("/path/to/certs"));
    }

    #[test]
    fn test_tls_config_repr_environment() {
        let config = PyTlsConfig {
            inner: TlsConfigInner::Environment("HANA_CA_CERT".to_string()),
        };
        let repr = config.__repr__();
        assert!(repr.contains("from_environment"));
        assert!(repr.contains("HANA_CA_CERT"));
    }

    #[test]
    fn test_tls_config_repr_direct() {
        let config = PyTlsConfig {
            inner: TlsConfigInner::Direct("pem content".to_string()),
        };
        let repr = config.__repr__();
        assert!(repr.contains("from_certificate"));
        assert!(repr.contains("<pem>"));
        assert!(!repr.contains("pem content"));
    }

    #[test]
    fn test_tls_config_repr_root_certificates() {
        let config = PyTlsConfig {
            inner: TlsConfigInner::RootCertificates,
        };
        let repr = config.__repr__();
        assert!(repr.contains("with_system_roots"));
    }

    #[test]
    fn test_tls_config_repr_insecure() {
        let config = PyTlsConfig {
            inner: TlsConfigInner::Insecure,
        };
        let repr = config.__repr__();
        assert!(repr.contains("insecure"));
    }

    #[test]
    fn test_apply_to_builder_mut() {
        let config = PyTlsConfig {
            inner: TlsConfigInner::RootCertificates,
        };
        let mut builder = hdbconnect::ConnectParams::builder();
        config.apply_to_builder_mut(&mut builder);
    }
}
