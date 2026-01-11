"""Polars integration utilities for pyhdb_rs.

Provides high-level functions matching Polars' read_database_uri pattern.

Example::

    import pyhdb_rs.polars as hdb

    # Read data
    df = hdb.read_hana(
        "SELECT * FROM sales WHERE year = 2024",
        "hdbsql://analyst:secret@hana.corp:39017"
    )

    # Write data
    hdb.write_hana(df, "MY_TABLE", uri, if_table_exists="replace")
"""

from __future__ import annotations

import contextlib
from typing import TYPE_CHECKING, Any, Literal

from pyhdb_rs._utils import validate_identifier

if TYPE_CHECKING:
    import polars as pl

__all__ = ["read_hana", "scan_hana", "write_hana"]


def read_hana(
    query: str,
    connection_uri: str,
    *,
    batch_size: int = 65536,
) -> pl.DataFrame:
    """Read SAP HANA query results directly into a Polars DataFrame.

    This function wraps connection creation and Arrow-based data transfer
    for zero-copy data loading.

    Args:
        query: SQL query to execute
        connection_uri: HANA connection URI (hdbsql://user:pass@host:port)
        batch_size: Number of rows per Arrow batch (default: 65536)

    Returns:
        Polars DataFrame with query results

    Example::

        import pyhdb_rs.polars as hdb

        df = hdb.read_hana(
            "SELECT * FROM sales WHERE year = 2024",
            "hdbsql://analyst:secret@hana.corp:39017"
        )
        print(df.head())
    """
    import polars as pl

    from pyhdb_rs import connect

    with connect(connection_uri) as conn:
        reader = conn.execute_arrow(query, batch_size=batch_size)
        return pl.from_arrow(reader)


def scan_hana(
    query: str,
    connection_uri: str,
    *,
    batch_size: int = 65536,
) -> pl.LazyFrame:
    """Create a lazy scan of SAP HANA query results.

    Unlike read_hana(), this returns a LazyFrame that can be
    composed with other operations before collection.

    Note:
        Currently this reads eagerly under the hood, but provides
        the LazyFrame API for composition. Future versions may
        implement true lazy scanning with predicate pushdown.

    Args:
        query: SQL query to execute
        connection_uri: HANA connection URI
        batch_size: Rows per Arrow batch

    Returns:
        Lazy representation for deferred execution

    Example::

        lf = hdb.scan_hana("SELECT * FROM sales", uri)
        result = (
            lf.filter(pl.col("amount") > 1000)
              .group_by("region")
              .agg(pl.sum("amount"))
              .collect()
        )
    """
    return read_hana(query, connection_uri, batch_size=batch_size).lazy()


def write_hana(
    df: pl.DataFrame,
    table: str,
    connection_uri: str,
    *,
    if_table_exists: Literal["fail", "replace", "append"] = "fail",
    batch_size: int = 10000,
) -> int:
    """Write a Polars DataFrame to SAP HANA table.

    Args:
        df: DataFrame to write
        table: Target table name (can include schema: "SCHEMA.TABLE")
        connection_uri: HANA connection URI
        if_table_exists: Behavior if table exists:
            - "fail": Raise error (default)
            - "replace": Drop and recreate table
            - "append": Insert into existing table
        batch_size: Rows per INSERT batch

    Returns:
        Number of rows written

    Example::

        df = pl.DataFrame({"id": [1, 2, 3], "value": [10, 20, 30]})
        rows = hdb.write_hana(df, "MY_TABLE", uri, if_table_exists="replace")
        print(f"Wrote {rows} rows")
    """

    from pyhdb_rs import connect

    validated_table = validate_identifier(table)

    with connect(connection_uri) as conn:
        cursor = conn.cursor()

        if if_table_exists == "replace":
            with contextlib.suppress(Exception):
                cursor.execute(f"DROP TABLE {validated_table}")
            _create_table_from_df(cursor, validated_table, df)

        columns = ", ".join(f'"{col}"' for col in df.columns)
        placeholders = ", ".join(["?"] * len(df.columns))
        insert_sql = f"INSERT INTO {validated_table} ({columns}) VALUES ({placeholders})"

        rows = df.rows()
        total = 0

        for i in range(0, len(rows), batch_size):
            batch = rows[i : i + batch_size]
            cursor.executemany(insert_sql, batch)
            total += len(batch)

        conn.commit()
        return total


def _create_table_from_df(cursor: Any, table: str, df: pl.DataFrame) -> None:
    """Generate and execute CREATE TABLE from DataFrame schema.

    Note: table name should already be validated by caller.
    """
    import polars as pl

    type_map: dict[type, str] = {
        pl.Int8: "TINYINT",
        pl.Int16: "SMALLINT",
        pl.Int32: "INTEGER",
        pl.Int64: "BIGINT",
        pl.UInt8: "TINYINT",
        pl.UInt16: "SMALLINT",
        pl.UInt32: "INTEGER",
        pl.UInt64: "BIGINT",
        pl.Float32: "REAL",
        pl.Float64: "DOUBLE",
        pl.Boolean: "BOOLEAN",
        pl.Utf8: "NVARCHAR(5000)",
        pl.String: "NVARCHAR(5000)",
        pl.Date: "DATE",
        pl.Time: "TIME",
        pl.Datetime: "TIMESTAMP",
        pl.Binary: "VARBINARY(5000)",
    }

    columns = []
    for name, dtype in df.schema.items():
        if isinstance(dtype, pl.Decimal):
            precision = dtype.precision or 38
            scale = dtype.scale or 0
            hana_type = f"DECIMAL({precision}, {scale})"
        else:
            hana_type = type_map.get(type(dtype), "NVARCHAR(5000)")
        columns.append(f'"{name}" {hana_type}')

    ddl = f"CREATE TABLE {table} ({', '.join(columns)})"
    cursor.execute(ddl)
