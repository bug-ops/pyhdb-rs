"""Arrow integration tests for pyhdb_rs."""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    import pyhdb_rs


class TestExecuteArrow:
    """Tests for Arrow data transfer."""

    def test_execute_arrow_returns_reader(
        self, connection: pyhdb_rs.Connection
    ) -> None:
        """Test that execute_arrow returns a RecordBatchReader."""
        import pyhdb_rs

        reader = connection.execute_arrow("SELECT 1 AS value FROM DUMMY")
        assert isinstance(reader, pyhdb_rs.RecordBatchReader)

    def test_arrow_to_polars(self, connection: pyhdb_rs.Connection) -> None:
        """Test Arrow to Polars conversion."""
        polars = pytest.importorskip("polars")

        reader = connection.execute_arrow(
            "SELECT 1 AS value, 'test' AS name FROM DUMMY"
        )
        df = polars.from_arrow(reader)

        assert len(df) == 1
        assert "VALUE" in df.columns or "value" in df.columns
        assert "NAME" in df.columns or "name" in df.columns

    def test_arrow_schema(self, connection: pyhdb_rs.Connection) -> None:
        """Test Arrow schema access."""
        pytest.importorskip("pyarrow")

        reader = connection.execute_arrow(
            "SELECT 1 AS int_col, 'test' AS str_col FROM DUMMY"
        )
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


class TestArrowCStreamProtocol:
    """Tests for Arrow PyCapsule Protocol (__arrow_c_stream__)."""

    def test_has_arrow_c_stream_method(self, connection: pyhdb_rs.Connection) -> None:
        """Test that RecordBatchReader has __arrow_c_stream__ method."""
        reader = connection.execute_arrow("SELECT 1 AS value FROM DUMMY")
        assert hasattr(reader, "__arrow_c_stream__")
        assert callable(reader.__arrow_c_stream__)

    def test_polars_from_arrow_uses_protocol(
        self, connection: pyhdb_rs.Connection
    ) -> None:
        """Test that Polars can consume reader via protocol."""
        polars = pytest.importorskip("polars")

        reader = connection.execute_arrow(
            "SELECT 1 AS int_col, 'test' AS str_col FROM DUMMY"
        )
        df = polars.from_arrow(reader)

        assert len(df) == 1
        assert len(df.columns) == 2

    def test_consumed_after_protocol_call(
        self, connection: pyhdb_rs.Connection
    ) -> None:
        """Test that reader is consumed after __arrow_c_stream__ call."""
        import pyhdb_rs

        reader = connection.execute_arrow("SELECT 1 AS value FROM DUMMY")

        # First call should succeed
        capsule = reader.__arrow_c_stream__()
        assert capsule is not None

        # Second call should fail
        with pytest.raises(pyhdb_rs.ProgrammingError, match="already consumed"):
            reader.__arrow_c_stream__()

    def test_repr_shows_consumed_state(self, connection: pyhdb_rs.Connection) -> None:
        """Test that repr shows consumed state after protocol call."""
        reader = connection.execute_arrow("SELECT 1 FROM DUMMY")

        assert "active" in repr(reader)
        _ = reader.__arrow_c_stream__()
        assert "consumed" in repr(reader)

    def test_pyarrow_from_stream(self, connection: pyhdb_rs.Connection) -> None:
        """Test that PyArrow can consume reader via protocol."""
        pyarrow = pytest.importorskip("pyarrow")

        reader = connection.execute_arrow("SELECT 1 AS value FROM DUMMY")
        pa_reader = pyarrow.RecordBatchReader.from_stream(reader)

        table = pa_reader.read_all()
        assert len(table) == 1

    def test_protocol_with_multiple_batches(
        self, connection: pyhdb_rs.Connection
    ) -> None:
        """Test protocol works with large result sets (multiple batches)."""
        import pyhdb_rs

        polars = pytest.importorskip("polars")

        # Generate multiple batches - use a system table with multiple rows
        config = pyhdb_rs.ArrowConfig(batch_size=100)
        reader = connection.execute_arrow(
            "SELECT TOP 500 * FROM M_TABLES", config=config
        )
        df = polars.from_arrow(reader)

        assert len(df) > 0

    def test_schema_preserved_through_protocol(
        self, connection: pyhdb_rs.Connection
    ) -> None:
        """Test that schema is correctly transferred via protocol."""
        pyarrow = pytest.importorskip("pyarrow")

        reader = connection.execute_arrow(
            "SELECT 1 AS int_col, 'test' AS str_col, 3.14 AS float_col FROM DUMMY"
        )
        pa_reader = pyarrow.RecordBatchReader.from_stream(reader)
        schema = pa_reader.schema

        assert len(schema) == 3
        # Verify field names (HANA returns uppercase)
        names = [f.name.upper() for f in schema]
        assert "INT_COL" in names
        assert "STR_COL" in names
        assert "FLOAT_COL" in names

    def test_null_values_through_protocol(
        self, connection: pyhdb_rs.Connection
    ) -> None:
        """Test that NULL values are correctly handled via protocol."""
        polars = pytest.importorskip("polars")

        reader = connection.execute_arrow("SELECT NULL AS null_col FROM DUMMY")
        df = polars.from_arrow(reader)

        assert len(df) == 1
        # HANA may return uppercase column names
        assert df["NULL_COL"][0] is None or df["null_col"][0] is None

    def test_requested_schema_parameter(self, connection: pyhdb_rs.Connection) -> None:
        """Test that requested_schema parameter is accepted."""
        reader = connection.execute_arrow("SELECT 1 AS value FROM DUMMY")

        # None is the most common case
        capsule = reader.__arrow_c_stream__(None)
        assert capsule is not None
