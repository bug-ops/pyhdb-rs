"""Pandas integration utilities for pyhdb_rs.

Uses PyArrow as intermediate format for efficient data transfer.

Example::

    import pyhdb_rs.pandas as hdb

    # Read data
    df = hdb.read_hana(
        "SELECT * FROM sales",
        "hdbsql://user:pass@host:39017"
    )

    # Write data
    hdb.to_hana(df, "MY_TABLE", uri, if_exists="replace")
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Literal

if TYPE_CHECKING:
    import pandas as pd

__all__ = ["read_hana", "to_hana"]


def read_hana(
    query: str,
    connection_uri: str,
    *,
    batch_size: int = 65536,
) -> pd.DataFrame:
    """Read SAP HANA query results into a pandas DataFrame.

    Uses PyArrow as intermediate format for efficient data transfer.

    Args:
        query: SQL query to execute
        connection_uri: HANA connection URI (hdbsql://user:pass@host:port)
        batch_size: Number of rows per Arrow batch

    Returns:
        pandas DataFrame with query results

    Example::

        import pyhdb_rs.pandas as hdb

        df = hdb.read_hana(
            "SELECT * FROM sales WHERE year = 2024",
            "hdbsql://user:pass@host:39017"
        )
    """
    from pyhdb_rs import connect

    with connect(connection_uri) as conn:
        reader = conn.execute_arrow(query, batch_size=batch_size)
        arrow_reader = reader.to_pyarrow()
        return arrow_reader.read_all().to_pandas()


def to_hana(
    df: pd.DataFrame,
    table: str,
    connection_uri: str,
    *,
    if_exists: Literal["fail", "replace", "append"] = "fail",
    batch_size: int = 10000,
) -> int:
    """Write a pandas DataFrame to SAP HANA table.

    Args:
        df: DataFrame to write
        table: Target table name (can include schema: "SCHEMA.TABLE")
        connection_uri: HANA connection URI
        if_exists: Behavior if table exists:
            - "fail": Raise error (default)
            - "replace": Drop and recreate table
            - "append": Insert into existing table
        batch_size: Rows per batch

    Returns:
        Number of rows written

    Example::

        import pandas as pd
        import pyhdb_rs.pandas as hdb

        df = pd.DataFrame({"id": [1, 2, 3], "value": [10, 20, 30]})
        hdb.to_hana(df, "MY_TABLE", uri, if_exists="replace")
    """
    import numpy as np

    from pyhdb_rs import connect

    with connect(connection_uri) as conn:
        cursor = conn.cursor()

        if if_exists == "replace":
            try:
                cursor.execute(f"DROP TABLE {table}")  # noqa: S608
            except Exception:
                pass
            _create_table_from_pandas(cursor, table, df)

        columns = ", ".join(f'"{col}"' for col in df.columns)
        placeholders = ", ".join(["?"] * len(df.columns))
        insert_sql = f"INSERT INTO {table} ({columns}) VALUES ({placeholders})"  # noqa: S608

        rows = df.replace({np.nan: None}).values.tolist()
        total = 0

        for i in range(0, len(rows), batch_size):
            batch = rows[i : i + batch_size]
            for row in batch:
                cursor.execute(insert_sql, tuple(row))
                total += 1

        conn.commit()
        return total


def _create_table_from_pandas(cursor: Any, table: str, df: pd.DataFrame) -> None:
    """Generate and execute CREATE TABLE from pandas DataFrame schema."""
    type_map: dict[str, str] = {
        "int8": "TINYINT",
        "int16": "SMALLINT",
        "int32": "INTEGER",
        "int64": "BIGINT",
        "uint8": "TINYINT",
        "uint16": "SMALLINT",
        "uint32": "INTEGER",
        "uint64": "BIGINT",
        "float32": "REAL",
        "float64": "DOUBLE",
        "bool": "BOOLEAN",
        "object": "NVARCHAR(5000)",
        "string": "NVARCHAR(5000)",
        "datetime64[ns]": "TIMESTAMP",
        "date": "DATE",
    }

    columns = []
    for col_name, dtype in df.dtypes.items():
        dtype_str = str(dtype)
        hana_type = type_map.get(dtype_str, "NVARCHAR(5000)")
        columns.append(f'"{col_name}" {hana_type}')

    ddl = f"CREATE TABLE {table} ({', '.join(columns)})"  # noqa: S608
    cursor.execute(ddl)
