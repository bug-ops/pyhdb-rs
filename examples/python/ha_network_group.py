"""High availability and network group configuration.

Demonstrates network group usage for HANA HA and Scale-Out deployments.
"""

import asyncio

import polars as pl

from pyhdb_rs import ConnectionBuilder, TlsConfig
from pyhdb_rs.aio import AsyncConnectionBuilder, ConnectionPoolBuilder


def sync_ha_connection():
    """Connect to HA cluster with network group."""
    print("Sync HA Connection with Network Group")
    print("-" * 50)

    conn = (
        ConnectionBuilder()
        .host("hana-ha-cluster.example.com")
        .port(30015)
        .credentials("SYSTEM", "password")
        .network_group("ha-primary")
        .tls(TlsConfig.with_system_roots())
        .build()
    )

    try:
        with conn.cursor() as cur:
            cur.execute(
                """SELECT HOST, PORT, ACTIVE_STATUS
                   FROM SYS.M_LANDSCAPE_HOST_CONFIGURATION"""
            )
            nodes = cur.fetchall()
            print(f"Connected to HA cluster with {len(nodes)} nodes:")
            for host, port, status in nodes:
                print(f"  {host}:{port} - {status}")

    finally:
        conn.close()


async def async_ha_connection():
    """Async connection to HA cluster."""
    print("\nAsync HA Connection")
    print("-" * 50)

    conn = await (
        AsyncConnectionBuilder()
        .host("hana-ha.example.com")
        .credentials("SYSTEM", "password")
        .network_group("internal")
        .tls(TlsConfig.with_system_roots())
        .build()
    )

    try:
        cursor = conn.cursor()
        await cursor.execute("SELECT DATABASE_NAME, VERSION FROM SYS.M_DATABASE")
        db_name, version = await cursor.fetchone()
        print(f"Connected to {db_name} version {version}")

    finally:
        await conn.close()


async def pool_with_network_group():
    """Connection pool for HA deployment."""
    print("\nConnection Pool with Network Group")
    print("-" * 50)

    pool = (
        ConnectionPoolBuilder()
        .url("hdbsql://user:pass@hana-ha.example.com:30015")
        .network_group("production")
        .max_size(20)
        .tls(TlsConfig.with_system_roots())
        .build()
    )

    async with pool.acquire() as conn:
        reader = await conn.execute_arrow(
            """SELECT HOST, SERVICE_NAME, PORT, ACTIVE_STATUS
               FROM SYS.M_SERVICES
               WHERE SERVICE_NAME IN ('indexserver', 'nameserver')
               ORDER BY HOST, SERVICE_NAME"""
        )
        df = pl.from_arrow(reader)
        print("HANA services:")
        print(df)

    status = pool.status
    print(f"\nPool: {status.size} connections, {status.available} available")


def scale_out_connection():
    """Connect to Scale-Out system with network group."""
    print("\nScale-Out Connection")
    print("-" * 50)

    conn = (
        ConnectionBuilder()
        .host("hana-scaleout.example.com")
        .credentials("SYSTEM", "password")
        .network_group("data-network")
        .tls(TlsConfig.with_system_roots())
        .build()
    )

    try:
        with conn.cursor() as cur:
            cur.execute(
                """SELECT HOST, COORDINATOR_TYPE, INDEXSERVER_ACTUAL_ROLE
                   FROM SYS.M_LANDSCAPE_HOST_CONFIGURATION
                   ORDER BY HOST"""
            )
            nodes = cur.fetchall()
            print(f"Scale-Out topology ({len(nodes)} nodes):")
            for host, coord_type, role in nodes:
                print(f"  {host}: {coord_type} - {role}")

    finally:
        conn.close()


def multi_network_routing():
    """Connect to different network groups for different purposes."""
    print("\nMulti-Network Routing")
    print("-" * 50)

    # Primary connection for writes
    conn_primary = (
        ConnectionBuilder()
        .host("hana-ha.example.com")
        .credentials("SYSTEM", "password")
        .network_group("internal")
        .build()
    )

    # Secondary connection for reads
    conn_secondary = (
        ConnectionBuilder()
        .host("hana-ha.example.com")
        .credentials("READONLY_USER", "password")
        .network_group("external")
        .build()
    )

    try:
        # Write to primary
        with conn_primary.cursor() as cur:
            cur.execute("INSERT INTO logs (message) VALUES (?)", ["Test message"])
            conn_primary.commit()
            print("Written to primary network")

        # Read from secondary
        with conn_secondary.cursor() as cur:
            cur.execute("SELECT COUNT(*) FROM logs")
            count = cur.fetchone()[0]
            print(f"Read from secondary network: {count} log entries")

    finally:
        conn_primary.close()
        conn_secondary.close()


async def failover_simulation():
    """Simulate failover scenario with network groups."""
    print("\nFailover Simulation")
    print("-" * 50)

    primary_group = "ha-primary"
    secondary_group = "ha-secondary"

    async def try_connect(network_group: str) -> bool:
        try:
            conn = await (
                AsyncConnectionBuilder()
                .host("hana-ha.example.com")
                .credentials("SYSTEM", "password")
                .network_group(network_group)
                .build()
            )
            await conn.close()
            return True
        except Exception as e:
            print(f"Failed to connect to {network_group}: {e}")
            return False

    # Try primary first
    if await try_connect(primary_group):
        print(f"Connected to {primary_group}")
    elif await try_connect(secondary_group):
        print(f"Failed over to {secondary_group}")
    else:
        print("Both primary and secondary unavailable")


async def main():
    """Run all HA and network group examples."""
    print("High Availability & Network Group Examples")
    print("=" * 50)

    sync_ha_connection()
    await async_ha_connection()
    await pool_with_network_group()
    scale_out_connection()
    multi_network_routing()
    await failover_simulation()

    print("\n" + "=" * 50)
    print("Network group best practices:")
    print("  - Use descriptive names: 'production', 'internal', 'backup'")
    print("  - Configure network groups in HANA system configuration")
    print("  - Test failover scenarios regularly")
    print("  - Monitor connection distribution across nodes")


if __name__ == "__main__":
    asyncio.run(main())
