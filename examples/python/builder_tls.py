"""TLS configuration examples with ConnectionBuilder.

Demonstrates all five TlsConfig factory methods for different
certificate sources and security requirements.
"""

import os
from pathlib import Path

from pyhdb_rs import ConnectionBuilder, TlsConfig


def example_from_directory():
    """Load certificates from a directory."""
    print("1. TlsConfig.from_directory()")
    print("-" * 50)

    cert_dir = "/etc/hana/certs"
    tls = TlsConfig.from_directory(cert_dir)

    conn = (
        ConnectionBuilder()
        .host("hana.example.com")
        .port(30015)
        .credentials("SYSTEM", "password")
        .tls(tls)
        .build()
    )

    try:
        with conn.cursor() as cur:
            cur.execute("SELECT * FROM DUMMY")
            print(f"Connected successfully with certificates from {cert_dir}")
    finally:
        conn.close()


def example_from_environment():
    """Load certificate from environment variable."""
    print("\n2. TlsConfig.from_environment()")
    print("-" * 50)

    # Set certificate in environment
    os.environ["HANA_CA_CERT"] = """-----BEGIN CERTIFICATE-----
MIIDdzCCAl+gAwIBAgIEAgAAuTANBgkqhkiG9w0BAQUFADBaMQswCQYDVQQGEwJJ
...
-----END CERTIFICATE-----"""

    tls = TlsConfig.from_environment("HANA_CA_CERT")

    conn = (
        ConnectionBuilder()
        .host("hana.example.com")
        .credentials("SYSTEM", "password")
        .tls(tls)
        .build()
    )

    try:
        with conn.cursor() as cur:
            cur.execute("SELECT * FROM DUMMY")
            print("Connected successfully with certificate from environment")
    finally:
        conn.close()


def example_from_certificate():
    """Load certificate from PEM string."""
    print("\n3. TlsConfig.from_certificate()")
    print("-" * 50)

    cert_path = Path("/path/to/ca-bundle.pem")

    if cert_path.exists():
        cert_pem = cert_path.read_text()
        tls = TlsConfig.from_certificate(cert_pem)

        conn = (
            ConnectionBuilder()
            .host("hana.example.com")
            .credentials("SYSTEM", "password")
            .tls(tls)
            .build()
        )

        try:
            with conn.cursor() as cur:
                cur.execute("SELECT * FROM DUMMY")
                print(f"Connected successfully with certificate from {cert_path}")
        finally:
            conn.close()
    else:
        print(f"Certificate file not found: {cert_path}")


def example_with_system_roots():
    """Use Mozilla root certificates (bundled)."""
    print("\n4. TlsConfig.with_system_roots()")
    print("-" * 50)

    tls = TlsConfig.with_system_roots()

    conn = (
        ConnectionBuilder()
        .host("hana.example.com")
        .port(30015)
        .credentials("SYSTEM", "password")
        .tls(tls)
        .build()
    )

    try:
        with conn.cursor() as cur:
            cur.execute("SELECT * FROM DUMMY")
            print("Connected successfully with system root certificates")
    finally:
        conn.close()


def example_insecure():
    """Skip certificate verification (development only)."""
    print("\n5. TlsConfig.insecure() - DEVELOPMENT ONLY!")
    print("-" * 50)
    print("WARNING: This disables certificate verification!")
    print("NEVER use this in production environments.")

    tls = TlsConfig.insecure()

    conn = (
        ConnectionBuilder()
        .host("hana-dev.internal")
        .credentials("SYSTEM", "password")
        .tls(tls)
        .build()
    )

    try:
        with conn.cursor() as cur:
            cur.execute("SELECT * FROM DUMMY")
            print("Connected successfully (no certificate verification)")
    finally:
        conn.close()


def example_url_scheme():
    """Use hdbsqls:// scheme for automatic TLS."""
    print("\n6. URL Scheme with TLS")
    print("-" * 50)

    # hdbsqls:// automatically enables TLS with system roots
    conn = ConnectionBuilder.from_url("hdbsqls://user:pass@host:30015").build()

    try:
        with conn.cursor() as cur:
            cur.execute("SELECT * FROM DUMMY")
            print("Connected successfully using hdbsqls:// scheme")
    finally:
        conn.close()

    # Override with custom TLS config
    conn = (
        ConnectionBuilder.from_url("hdbsqls://user:pass@host:30015")
        .tls(TlsConfig.from_directory("/custom/certs"))
        .build()
    )

    try:
        with conn.cursor() as cur:
            cur.execute("SELECT * FROM DUMMY")
            print("Connected with custom TLS override")
    finally:
        conn.close()


def main():
    """Run all TLS configuration examples."""
    print("TLS Configuration Examples")
    print("=" * 50)

    examples = [
        example_with_system_roots,
        example_from_environment,
        example_insecure,
        example_url_scheme,
    ]

    for example in examples:
        try:
            example()
        except Exception as e:
            print(f"Error: {e}")

    print("\n" + "=" * 50)
    print("Recommendation: Use TlsConfig.from_directory() for production")
    print("                or TlsConfig.with_system_roots() for public CAs")


if __name__ == "__main__":
    main()
