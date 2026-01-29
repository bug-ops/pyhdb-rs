"""Basic ConnectionBuilder usage.

This example demonstrates the fundamental usage of ConnectionBuilder
for creating synchronous connections to SAP HANA.
"""

from pyhdb_rs import ConnectionBuilder


def main():
    # Build a connection using the builder pattern
    conn = (
        ConnectionBuilder()
        .host("hana.example.com")
        .port(30015)
        .credentials("SYSTEM", "password")
        .database("SYSTEMDB")
        .build()
    )

    try:
        # Execute a simple query
        with conn.cursor() as cur:
            cur.execute("SELECT * FROM DUMMY")
            result = cur.fetchall()
            print(f"Query result: {result}")

        # Parameterized query
        with conn.cursor() as cur:
            cur.execute(
                "SELECT SCHEMA_NAME, TABLE_NAME FROM SYS.TABLES WHERE SCHEMA_NAME = ? LIMIT 5",
                ["SYS"],
            )
            tables = cur.fetchall()
            print(f"\nFound {len(tables)} tables in SYS schema:")
            for schema, table in tables:
                print(f"  {schema}.{table}")

    finally:
        conn.close()


if __name__ == "__main__":
    main()
