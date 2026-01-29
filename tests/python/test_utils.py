"""Unit tests for pyhdb_rs._utils module."""

from __future__ import annotations

from unittest.mock import MagicMock

import pytest


class TestValidateIdentifier:
    """Tests for validate_identifier function."""

    def test_valid_simple_identifier(self) -> None:
        """Test that simple valid identifiers pass validation."""
        from pyhdb_rs._utils import validate_identifier

        assert validate_identifier("table_name") == "table_name"
        assert validate_identifier("_private") == "_private"
        assert validate_identifier("Table123") == "Table123"
        assert validate_identifier("a") == "a"
        assert validate_identifier("_") == "_"

    def test_valid_schema_table(self) -> None:
        """Test that schema.table format is valid."""
        from pyhdb_rs._utils import validate_identifier

        assert validate_identifier("schema.table") == "schema.table"
        assert validate_identifier("MySchema.MyTable") == "MySchema.MyTable"
        assert validate_identifier("_schema._table") == "_schema._table"

    def test_invalid_hyphen(self) -> None:
        """Test that hyphens are rejected."""
        from pyhdb_rs._utils import validate_identifier

        with pytest.raises(ValueError, match="Invalid SQL identifier"):
            validate_identifier("table-name")

    def test_invalid_starts_with_number(self) -> None:
        """Test that identifiers starting with numbers are rejected."""
        from pyhdb_rs._utils import validate_identifier

        with pytest.raises(ValueError, match="Invalid SQL identifier"):
            validate_identifier("123table")

    def test_invalid_sql_injection(self) -> None:
        """Test that SQL injection attempts are rejected."""
        from pyhdb_rs._utils import validate_identifier

        with pytest.raises(ValueError, match="Invalid SQL identifier"):
            validate_identifier("table; DROP TABLE")

        with pytest.raises(ValueError, match="Invalid SQL identifier"):
            validate_identifier("table--comment")

        with pytest.raises(ValueError, match="Invalid SQL identifier"):
            validate_identifier("table' OR '1'='1")

    def test_invalid_double_dot(self) -> None:
        """Test that double dots are rejected."""
        from pyhdb_rs._utils import validate_identifier

        with pytest.raises(ValueError, match="Invalid SQL identifier"):
            validate_identifier("schema..table")

    def test_invalid_empty_string(self) -> None:
        """Test that empty strings are rejected."""
        from pyhdb_rs._utils import validate_identifier

        with pytest.raises(ValueError, match="Invalid SQL identifier"):
            validate_identifier("")

    def test_invalid_special_characters(self) -> None:
        """Test that special characters are rejected."""
        from pyhdb_rs._utils import validate_identifier

        with pytest.raises(ValueError, match="Invalid SQL identifier"):
            validate_identifier("table@name")

        with pytest.raises(ValueError, match="Invalid SQL identifier"):
            validate_identifier("table#name")

        with pytest.raises(ValueError, match="Invalid SQL identifier"):
            validate_identifier("table$name")

    def test_invalid_whitespace(self) -> None:
        """Test that whitespace is rejected."""
        from pyhdb_rs._utils import validate_identifier

        with pytest.raises(ValueError, match="Invalid SQL identifier"):
            validate_identifier("table name")

        with pytest.raises(ValueError, match="Invalid SQL identifier"):
            validate_identifier(" table")

        with pytest.raises(ValueError, match="Invalid SQL identifier"):
            validate_identifier("table ")

    def test_max_length_valid(self) -> None:
        """Test that identifiers at max length are valid."""
        from pyhdb_rs._utils import MAX_IDENTIFIER_LENGTH, validate_identifier

        valid_name = "a" * MAX_IDENTIFIER_LENGTH
        assert validate_identifier(valid_name) == valid_name

    def test_exceeds_max_length(self) -> None:
        """Test that identifiers exceeding max length are rejected."""
        from pyhdb_rs._utils import MAX_IDENTIFIER_LENGTH, validate_identifier

        long_name = "a" * (MAX_IDENTIFIER_LENGTH + 1)
        with pytest.raises(ValueError, match="exceeds maximum length"):
            validate_identifier(long_name)

    def test_max_length_constant(self) -> None:
        """Test that MAX_IDENTIFIER_LENGTH is 127 per SAP HANA spec."""
        from pyhdb_rs._utils import MAX_IDENTIFIER_LENGTH

        assert MAX_IDENTIFIER_LENGTH == 127


class TestBatchInsert:
    """Tests for batch_insert function."""

    def test_single_batch(self) -> None:
        """Test insert with rows fitting in single batch."""
        from pyhdb_rs._utils import batch_insert

        cursor = MagicMock()
        rows = [(1, "a"), (2, "b"), (3, "c")]

        total = batch_insert(cursor, "test_table", ["id", "name"], rows, batch_size=10)

        assert total == 3
        assert cursor.executemany.call_count == 1

    def test_multiple_batches(self) -> None:
        """Test insert with rows spanning multiple batches."""
        from pyhdb_rs._utils import batch_insert

        cursor = MagicMock()
        rows = [(i, f"row{i}") for i in range(25)]

        total = batch_insert(cursor, "test_table", ["id", "name"], rows, batch_size=10)

        assert total == 25
        assert cursor.executemany.call_count == 3  # 10 + 10 + 5

    def test_empty_rows(self) -> None:
        """Test insert with empty rows list."""
        from pyhdb_rs._utils import batch_insert

        cursor = MagicMock()
        rows: list[tuple[int, str]] = []

        total = batch_insert(cursor, "test_table", ["id", "name"], rows)

        assert total == 0
        assert cursor.executemany.call_count == 0

    def test_single_row(self) -> None:
        """Test insert with single row."""
        from pyhdb_rs._utils import batch_insert

        cursor = MagicMock()
        rows = [(42, "single")]

        total = batch_insert(cursor, "test_table", ["id", "name"], rows)

        assert total == 1
        assert cursor.executemany.call_count == 1

    def test_exact_batch_size(self) -> None:
        """Test insert with rows exactly matching batch size."""
        from pyhdb_rs._utils import batch_insert

        cursor = MagicMock()
        rows = [(i, f"row{i}") for i in range(10)]

        total = batch_insert(cursor, "test_table", ["id", "name"], rows, batch_size=10)

        assert total == 10
        assert cursor.executemany.call_count == 1

    def test_sql_format(self) -> None:
        """Test that generated SQL has correct format."""
        from pyhdb_rs._utils import batch_insert

        cursor = MagicMock()
        rows = [(1, "test")]

        batch_insert(cursor, "schema.table", ["col1", "col2"], rows)

        call_args = cursor.executemany.call_args
        sql = call_args[0][0]
        assert sql == 'INSERT INTO schema.table ("col1", "col2") VALUES (?, ?)'


class TestIterPandasRows:
    """Tests for iter_pandas_rows function."""

    def test_nan_to_none_conversion(self) -> None:
        """Test that NaN values are converted to None."""
        pytest.importorskip("pandas")
        import numpy as np
        import pandas as pd
        from pyhdb_rs._utils import iter_pandas_rows

        df = pd.DataFrame({"a": [1, np.nan, 3], "b": ["x", "y", np.nan]})

        rows = list(iter_pandas_rows(df))

        assert rows[0] == (1.0, "x")
        assert rows[1][0] is None
        assert rows[1][1] == "y"
        assert rows[2][0] == 3.0
        assert rows[2][1] is None

    def test_empty_dataframe(self) -> None:
        """Test iteration over empty DataFrame."""
        pytest.importorskip("pandas")
        import pandas as pd
        from pyhdb_rs._utils import iter_pandas_rows

        df = pd.DataFrame({"a": [], "b": []})

        rows = list(iter_pandas_rows(df))

        assert rows == []

    def test_no_nan_values(self) -> None:
        """Test DataFrame without NaN values."""
        pytest.importorskip("pandas")
        import pandas as pd
        from pyhdb_rs._utils import iter_pandas_rows

        df = pd.DataFrame({"a": [1, 2, 3], "b": ["x", "y", "z"]})

        rows = list(iter_pandas_rows(df))

        assert rows == [(1, "x"), (2, "y"), (3, "z")]

    def test_mixed_types(self) -> None:
        """Test DataFrame with mixed types."""
        pytest.importorskip("pandas")
        import pandas as pd
        from pyhdb_rs._utils import iter_pandas_rows

        df = pd.DataFrame(
            {"int_col": [1, 2], "str_col": ["a", "b"], "float_col": [1.5, 2.5]}
        )

        rows = list(iter_pandas_rows(df))

        assert rows == [(1, "a", 1.5), (2, "b", 2.5)]

    def test_returns_iterator(self) -> None:
        """Test that function returns an iterator, not a list."""
        pytest.importorskip("pandas")
        import pandas as pd
        from pyhdb_rs._utils import iter_pandas_rows

        df = pd.DataFrame({"a": [1, 2, 3]})

        result = iter_pandas_rows(df)

        assert hasattr(result, "__iter__")
        assert hasattr(result, "__next__")


class TestIdentifierPattern:
    """Tests for IDENTIFIER_PATTERN constant."""

    def test_pattern_exists(self) -> None:
        """Test that pattern is exported."""
        from pyhdb_rs._utils import IDENTIFIER_PATTERN

        assert IDENTIFIER_PATTERN is not None

    def test_pattern_is_compiled_regex(self) -> None:
        """Test that pattern is a compiled regex."""
        import re

        from pyhdb_rs._utils import IDENTIFIER_PATTERN

        assert isinstance(IDENTIFIER_PATTERN, re.Pattern)
