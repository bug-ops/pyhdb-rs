//! TLS configuration helpers for connection builders.

use crate::tls::TlsConfigInner;

/// Apply TLS configuration to a sync `ConnectParamsBuilder`.
pub fn apply_tls_to_sync_builder(
    tls: &TlsConfigInner,
    builder: &mut hdbconnect::ConnectParamsBuilder,
) {
    match tls {
        TlsConfigInner::Directory(path) => {
            builder.tls_with(hdbconnect::ServerCerts::Directory(path.clone()));
        }
        TlsConfigInner::Environment(var) => {
            builder.tls_with(hdbconnect::ServerCerts::Environment(var.clone()));
        }
        TlsConfigInner::Direct(pem) => {
            builder.tls_with(hdbconnect::ServerCerts::Direct(pem.clone()));
        }
        TlsConfigInner::RootCertificates => {
            builder.tls_with(hdbconnect::ServerCerts::RootCertificates);
        }
        TlsConfigInner::Insecure => {
            builder.tls_without_server_verification();
        }
    }
}

/// Apply TLS configuration to an async `ConnectParamsBuilder`.
#[cfg(feature = "async")]
pub fn apply_tls_to_async_builder(
    tls: &TlsConfigInner,
    builder: &mut hdbconnect_async::ConnectParamsBuilder,
) {
    match tls {
        TlsConfigInner::Directory(path) => {
            builder.tls_with(hdbconnect_async::ServerCerts::Directory(path.clone()));
        }
        TlsConfigInner::Environment(var) => {
            builder.tls_with(hdbconnect_async::ServerCerts::Environment(var.clone()));
        }
        TlsConfigInner::Direct(pem) => {
            builder.tls_with(hdbconnect_async::ServerCerts::Direct(pem.clone()));
        }
        TlsConfigInner::RootCertificates => {
            builder.tls_with(hdbconnect_async::ServerCerts::RootCertificates);
        }
        TlsConfigInner::Insecure => {
            builder.tls_without_server_verification();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_tls_directory_sync() {
        let mut builder = hdbconnect::ConnectParams::builder();
        apply_tls_to_sync_builder(
            &TlsConfigInner::Directory("/path".to_string()),
            &mut builder,
        );
    }

    #[test]
    fn test_apply_tls_environment_sync() {
        let mut builder = hdbconnect::ConnectParams::builder();
        apply_tls_to_sync_builder(
            &TlsConfigInner::Environment("HANA_CA_CERT".to_string()),
            &mut builder,
        );
    }

    #[test]
    fn test_apply_tls_direct_sync() {
        let mut builder = hdbconnect::ConnectParams::builder();
        apply_tls_to_sync_builder(
            &TlsConfigInner::Direct("-----BEGIN CERTIFICATE-----".to_string()),
            &mut builder,
        );
    }

    #[test]
    fn test_apply_tls_root_certificates_sync() {
        let mut builder = hdbconnect::ConnectParams::builder();
        apply_tls_to_sync_builder(&TlsConfigInner::RootCertificates, &mut builder);
    }

    #[test]
    fn test_apply_tls_insecure_sync() {
        let mut builder = hdbconnect::ConnectParams::builder();
        apply_tls_to_sync_builder(&TlsConfigInner::Insecure, &mut builder);
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_apply_tls_directory_async() {
        let mut builder = hdbconnect_async::ConnectParams::builder();
        apply_tls_to_async_builder(
            &TlsConfigInner::Directory("/path".to_string()),
            &mut builder,
        );
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_apply_tls_environment_async() {
        let mut builder = hdbconnect_async::ConnectParams::builder();
        apply_tls_to_async_builder(
            &TlsConfigInner::Environment("HANA_CA_CERT".to_string()),
            &mut builder,
        );
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_apply_tls_direct_async() {
        let mut builder = hdbconnect_async::ConnectParams::builder();
        apply_tls_to_async_builder(
            &TlsConfigInner::Direct("-----BEGIN CERTIFICATE-----".to_string()),
            &mut builder,
        );
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_apply_tls_root_certificates_async() {
        let mut builder = hdbconnect_async::ConnectParams::builder();
        apply_tls_to_async_builder(&TlsConfigInner::RootCertificates, &mut builder);
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_apply_tls_insecure_async() {
        let mut builder = hdbconnect_async::ConnectParams::builder();
        apply_tls_to_async_builder(&TlsConfigInner::Insecure, &mut builder);
    }
}
