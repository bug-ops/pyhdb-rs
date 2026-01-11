"""Arrow integration tests for pyhdb_rs."""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    import pyhdb_rs


class TestExecuteArrow:
    """Tests for Arrow data transfer."""

    def test_execute_arrow_returns_reader(self, connection: pyhdb_rs.Connection) -> None:
        """Test that execute_arrow returns a RecordBatchReader."""
        import pyhdb_rs

        reader = connection.execute_arrow("SELECT 1 AS value FROM DUMMY")
        assert isinstance(reader, pyhdb_rs.RecordBatchReader)

    def test_arrow_to_polars(self, connection: pyhdb_rs.Connection) -> None:
        """Test Arrow to Polars conversion."""
        polars = pytest.importorskip("polars")

        reader = connection.execute_arrow("SELECT 1 AS value, 'test' AS name FROM DUMMY")
        df = polars.from_arrow(reader)

        assert len(df) == 1
        assert "VALUE" in df.columns or "value" in df.columns
        assert "NAME" in df.columns or "name" in df.columns

    def test_arrow_schema(self, connection: pyhdb_rs.Connection) -> None:
        """Test Arrow schema access."""
        pytest.importorskip("pyarrow")

        reader = connection.execute_arrow("SELECT 1 AS int_col, 'test' AS str_col FROM DUMMY")
        schema = reader.schema()

        assert schema is not None
        assert len(schema) == 2

    def test_arrow_to_pyarrow(self, connection: pyhdb_rs.Connection) -> None:
        """Test conversion to PyArrow reader."""
        pytest.importorskip("pyarrow")

        reader = connection.execute_arrow("SELECT 1 AS value FROM DUMMY")
        pa_reader = reader.to_pyarrow()

        table = pa_reader.read_all()
        assert len(table) == 1

    def test_decimal_precision(self, connection: pyhdb_rs.Connection) -> None:
        """Test DECIMAL type preserves precision."""
        pyarrow = pytest.importorskip("pyarrow")

        reader = connection.execute_arrow(
            "SELECT CAST(123.456 AS DECIMAL(10, 3)) AS dec_value FROM DUMMY"
        )
        pa_reader = reader.to_pyarrow()
        table = pa_reader.read_all()

        assert len(table) == 1
        field = table.schema.field(0)
        assert pyarrow.types.is_decimal(field.type)

    def test_null_handling(self, connection: pyhdb_rs.Connection) -> None:
        """Test NULL values are handled correctly."""
        pytest.importorskip("pyarrow")

        reader = connection.execute_arrow("SELECT NULL AS null_col FROM DUMMY")
        pa_reader = reader.to_pyarrow()
        table = pa_reader.read_all()

        assert len(table) == 1
        assert table.column(0).null_count == 1


class TestExecutePolars:
    """Tests for direct Polars integration."""

    def test_execute_polars_returns_dataframe(self, connection: pyhdb_rs.Connection) -> None:
        """Test that execute_polars returns a Polars DataFrame."""
        polars = pytest.importorskip("polars")

        df = connection.execute_polars("SELECT 1 AS value FROM DUMMY")
        assert isinstance(df, polars.DataFrame)

    def test_execute_polars_multiple_columns(self, connection: pyhdb_rs.Connection) -> None:
        """Test execute_polars with multiple columns."""
        pytest.importorskip("polars")

        df = connection.execute_polars(
            "SELECT 1 AS int_col, 'hello' AS str_col, 3.14 AS float_col FROM DUMMY"
        )
        assert len(df) == 1
        assert len(df.columns) == 3


class TestCursorFetchArrow:
    """Tests for Cursor.fetch_arrow."""

    def test_fetch_arrow_returns_reader(self, cursor: pyhdb_rs.Cursor) -> None:
        """Test that fetch_arrow returns a RecordBatchReader."""
        import pyhdb_rs

        cursor.execute("SELECT 1 AS value FROM DUMMY")
        reader = cursor.fetch_arrow()
        assert isinstance(reader, pyhdb_rs.RecordBatchReader)

    def test_fetch_arrow_batch_size(self, cursor: pyhdb_rs.Cursor) -> None:
        """Test fetch_arrow with custom batch size."""
        pytest.importorskip("pyarrow")

        cursor.execute("SELECT 1 AS value FROM DUMMY")
        reader = cursor.fetch_arrow(batch_size=100)
        pa_reader = reader.to_pyarrow()

        table = pa_reader.read_all()
        assert len(table) == 1
