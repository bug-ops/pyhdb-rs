"""Migration guide from v0.2.x to v0.3.0.

Demonstrates the changes and new features in v0.3.0,
with side-by-side comparisons of old and new APIs.
"""

import asyncio

import pyhdb_rs
from pyhdb_rs import ConnectionBuilder, CursorHoldability, TlsConfig
from pyhdb_rs.aio import AsyncConnectionBuilder, ConnectionPoolBuilder, connect, create_pool


def old_vs_new_connection():
    """Compare old and new connection methods."""
    print("1. Basic Connection")
    print("-" * 50)

    # OLD STYLE (still works)
    print("Old style (v0.2.x):")
    conn_old = pyhdb_rs.connect("hdbsql://user:pass@host:30015")
    print("  conn = pyhdb_rs.connect('hdbsql://user:pass@host:30015')")
    conn_old.close()

    # NEW STYLE (recommended)
    print("\nNew style (v0.3.0 - recommended):")
    conn_new = (
        ConnectionBuilder()
        .host("host")
        .port(30015)
        .credentials("user", "pass")
        .build()
    )
    print("  conn = (ConnectionBuilder()")
    print("      .host('host')")
    print("      .port(30015)")
    print("      .credentials('user', 'pass')")
    print("      .build())")
    conn_new.close()

    print("\nBoth methods work, but builder provides better configuration options")


async def async_statement_cache_removal():
    """Demonstrate removal of statement_cache_size parameter."""
    print("\n2. Async Connection - Removed statement_cache_size")
    print("-" * 50)

    # OLD STYLE (no longer works)
    print("Old style (v0.2.x) - REMOVED:")
    print("  conn = await connect(url, statement_cache_size=100)")
    print("  ^^ This parameter has been removed")

    # NEW STYLE
    print("\nNew style (v0.3.0):")
    conn = await connect("hdbsql://user:pass@host:30015")
    print("  conn = await connect('hdbsql://user:pass@host:30015')")
    print("  # statement_cache_size is always 100 (default)")
    await conn.close()


def tls_configuration_new():
    """New TlsConfig feature in v0.3.0."""
    print("\n3. NEW: TLS Configuration")
    print("-" * 50)

    print("v0.3.0 introduces TlsConfig with 5 factory methods:")

    # Method 1: System roots
    print("\n1. System root certificates:")
    tls1 = TlsConfig.with_system_roots()
    print("  tls = TlsConfig.with_system_roots()")

    # Method 2: From directory
    print("\n2. From directory:")
    print("  tls = TlsConfig.from_directory('/etc/hana/certs')")

    # Method 3: From environment
    print("\n3. From environment variable:")
    print("  tls = TlsConfig.from_environment('HANA_CA_CERT')")

    # Method 4: From certificate string
    print("\n4. From certificate string:")
    print("  tls = TlsConfig.from_certificate(cert_pem)")

    # Method 5: Insecure (dev only)
    print("\n5. Insecure (development only):")
    print("  tls = TlsConfig.insecure()")

    # Usage with builder
    print("\nUsage:")
    conn = (
        ConnectionBuilder()
        .host("hana.example.com")
        .credentials("SYSTEM", "password")
        .tls(TlsConfig.with_system_roots())
        .build()
    )
    print("  conn = (ConnectionBuilder()")
    print("      .host('hana.example.com')")
    print("      .credentials('SYSTEM', 'password')")
    print("      .tls(TlsConfig.with_system_roots())")
    print("      .build())")
    conn.close()


def cursor_holdability_new():
    """New CursorHoldability feature in v0.3.0."""
    print("\n4. NEW: Cursor Holdability")
    print("-" * 50)

    print("Control cursor behavior across transactions:")

    conn = (
        ConnectionBuilder()
        .host("hana.example.com")
        .credentials("SYSTEM", "password")
        .cursor_holdability(CursorHoldability.CommitAndRollback)
        .build()
    )

    print("\n  conn = (ConnectionBuilder()")
    print("      .host('hana.example.com')")
    print("      .credentials('SYSTEM', 'password')")
    print("      .cursor_holdability(CursorHoldability.CommitAndRollback)")
    print("      .build())")

    print("\nVariants:")
    print("  - CursorHoldability.None_: Closed on commit and rollback")
    print("  - CursorHoldability.Commit: Held across commits")
    print("  - CursorHoldability.Rollback: Held across rollbacks")
    print("  - CursorHoldability.CommitAndRollback: Held across both")

    conn.close()


def network_group_new():
    """New network_group parameter in v0.3.0."""
    print("\n5. NEW: Network Groups (HA/Scale-Out)")
    print("-" * 50)

    print("Configure network routing for HA and Scale-Out:")

    conn = (
        ConnectionBuilder()
        .host("hana-ha.example.com")
        .credentials("SYSTEM", "password")
        .network_group("production")
        .build()
    )

    print("\n  conn = (ConnectionBuilder()")
    print("      .host('hana-ha.example.com')")
    print("      .credentials('SYSTEM', 'password')")
    print("      .network_group('production')")
    print("      .build())")

    print("\nUseful for:")
    print("  - High availability clusters")
    print("  - Scale-out deployments")
    print("  - Multi-network configurations")

    conn.close()


async def pool_builder_new():
    """New ConnectionPoolBuilder in v0.3.0."""
    print("\n6. NEW: Connection Pool Builder")
    print("-" * 50)

    # OLD STYLE (still works)
    print("Old style (v0.2.x - still works):")
    pool_old = create_pool("hdbsql://user:pass@host:30015", max_size=10)
    print("  pool = create_pool('hdbsql://user:pass@host:30015', max_size=10)")

    # NEW STYLE (recommended)
    print("\nNew style (v0.3.0 - recommended):")
    pool_new = (
        ConnectionPoolBuilder()
        .url("hdbsql://user:pass@host:30015")
        .max_size(10)
        .tls(TlsConfig.with_system_roots())
        .network_group("production")
        .build()
    )
    print("  pool = (ConnectionPoolBuilder()")
    print("      .url('hdbsql://user:pass@host:30015')")
    print("      .max_size(10)")
    print("      .tls(TlsConfig.with_system_roots())")
    print("      .network_group('production')")
    print("      .build())")

    print("\nBenefits:")
    print("  - Consistent builder pattern")
    print("  - TLS configuration")
    print("  - Network group support")


async def async_builder_new():
    """New AsyncConnectionBuilder in v0.3.0."""
    print("\n7. NEW: Async Connection Builder")
    print("-" * 50)

    conn = await (
        AsyncConnectionBuilder()
        .host("hana.example.com")
        .credentials("SYSTEM", "password")
        .tls(TlsConfig.with_system_roots())
        .autocommit(True)
        .build()
    )

    print("  conn = await (AsyncConnectionBuilder()")
    print("      .host('hana.example.com')")
    print("      .credentials('SYSTEM', 'password')")
    print("      .tls(TlsConfig.with_system_roots())")
    print("      .autocommit(True)")
    print("      .build())")

    await conn.close()


def upgrade_checklist():
    """Print upgrade checklist."""
    print("\n" + "=" * 50)
    print("UPGRADE CHECKLIST")
    print("=" * 50)

    checklist = [
        "Remove statement_cache_size from async connect() calls",
        "Consider migrating to ConnectionBuilder for better configuration",
        "Use TlsConfig for explicit TLS configuration",
        "Add network_group if using HANA HA/Scale-Out",
        "Use ConnectionPoolBuilder for new pool configurations",
        "Test CursorHoldability for large result set processing",
    ]

    for i, item in enumerate(checklist, 1):
        print(f"{i}. [ ] {item}")


async def main():
    """Run all migration examples."""
    print("Migration Guide: v0.2.x → v0.3.0")
    print("=" * 50)

    old_vs_new_connection()
    await async_statement_cache_removal()
    tls_configuration_new()
    cursor_holdability_new()
    network_group_new()
    await pool_builder_new()
    await async_builder_new()
    upgrade_checklist()

    print("\n" + "=" * 50)
    print("Summary:")
    print("  - Old APIs still work (backwards compatible)")
    print("  - New builder APIs provide more flexibility")
    print("  - TLS, cursor holdability, and network groups are new features")
    print("  - statement_cache_size removed from async connect()")


if __name__ == "__main__":
    asyncio.run(main())
