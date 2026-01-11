"""Shared utilities for pyhdb_rs modules.

Internal module for code shared between polars.py and pandas.py.
"""

from __future__ import annotations

import re
from typing import TYPE_CHECKING, Final

if TYPE_CHECKING:
    from collections.abc import Iterator, Sequence

    import pandas as pd

__all__ = [
    "IDENTIFIER_PATTERN",
    "MAX_IDENTIFIER_LENGTH",
    "validate_identifier",
    "batch_insert",
    "iter_pandas_rows",
]


IDENTIFIER_PATTERN: Final = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*)?$")

MAX_IDENTIFIER_LENGTH: Final = 127


def validate_identifier(name: str) -> str:
    """Validate SQL identifier to prevent injection.

    Args:
        name: Table or schema.table name to validate

    Returns:
        The validated name if valid

    Raises:
        ValueError: If name contains invalid characters or exceeds length limit
    """
    if not IDENTIFIER_PATTERN.match(name):
        raise ValueError(f"Invalid SQL identifier: {name!r}")
    if len(name) > MAX_IDENTIFIER_LENGTH:
        raise ValueError(f"Identifier exceeds maximum length ({MAX_IDENTIFIER_LENGTH}): {name!r}")
    return name


def batch_insert(
    cursor,
    table: str,
    columns: list[str],
    rows: Sequence,
    batch_size: int = 10000,
) -> int:
    """Execute batched INSERT statements.

    Args:
        cursor: Database cursor with execute/executemany methods
        table: Validated table name
        columns: List of column names
        rows: Sequence of row tuples
        batch_size: Maximum rows per INSERT batch

    Returns:
        Total number of rows inserted
    """
    columns_str = ", ".join(f'"{col}"' for col in columns)
    placeholders = ", ".join(["?"] * len(columns))
    insert_sql = f"INSERT INTO {table} ({columns_str}) VALUES ({placeholders})"

    total = 0
    for i in range(0, len(rows), batch_size):
        batch = rows[i : i + batch_size]
        cursor.executemany(insert_sql, [tuple(row) for row in batch])
        total += len(batch)
    return total


def iter_pandas_rows(df: pd.DataFrame) -> Iterator[tuple]:
    """Yield rows from pandas DataFrame with NaN replaced by None.

    More memory-efficient than df.replace({np.nan: None}).values.tolist()
    for large DataFrames.

    Args:
        df: pandas DataFrame to iterate

    Yields:
        Row tuples with None instead of NaN
    """
    import pandas as pd

    for row in df.itertuples(index=False, name=None):
        yield tuple(None if pd.isna(v) else v for v in row)
