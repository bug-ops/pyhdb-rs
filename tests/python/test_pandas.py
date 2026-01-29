"""Tests for pandas integration via execute_arrow() API.

These tests verify that the execute_arrow() API works correctly with pandas
for converting HANA query results to pandas DataFrames.
"""

from __future__ import annotations

import pytest


class TestExecuteArrowPandas:
    """Tests for execute_arrow() with pandas conversion."""

    def test_execute_arrow_to_pandas_returns_dataframe(self, hana_uri: str) -> None:
        """Test that execute_arrow() result can be converted to pandas DataFrame."""
        pandas = pytest.importorskip("pandas")
        pyarrow = pytest.importorskip("pyarrow")
        from pyhdb_rs import ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            reader = conn.execute_arrow("SELECT 1 AS value FROM DUMMY")
            pa_reader = pyarrow.RecordBatchReader.from_stream(reader)
            df = pa_reader.read_all().to_pandas()
            assert isinstance(df, pandas.DataFrame)
            assert len(df) == 1
        finally:
            conn.close()

    def test_execute_arrow_with_arrow_config(self, hana_uri: str) -> None:
        """Test execute_arrow with ArrowConfig for custom batch size."""
        pandas = pytest.importorskip("pandas")
        pyarrow = pytest.importorskip("pyarrow")
        from pyhdb_rs import ArrowConfig, ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            config = ArrowConfig(batch_size=100)
            reader = conn.execute_arrow("SELECT 1 AS value FROM DUMMY", config=config)
            pa_reader = pyarrow.RecordBatchReader.from_stream(reader)
            df = pa_reader.read_all().to_pandas()
            assert isinstance(df, pandas.DataFrame)
            assert len(df) == 1
        finally:
            conn.close()

    def test_execute_arrow_multiple_columns(self, hana_uri: str) -> None:
        """Test execute_arrow with multiple columns."""
        pytest.importorskip("pandas")
        pyarrow = pytest.importorskip("pyarrow")
        from pyhdb_rs import ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            reader = conn.execute_arrow("SELECT 1 AS int_col, 'hello' AS str_col FROM DUMMY")
            pa_reader = pyarrow.RecordBatchReader.from_stream(reader)
            df = pa_reader.read_all().to_pandas()
            assert len(df.columns) == 2
        finally:
            conn.close()


class TestCursorFetchArrowPandas:
    """Tests for cursor.fetch_arrow() with pandas conversion."""

    def test_cursor_fetch_arrow_to_pandas(self, hana_uri: str) -> None:
        """Test that cursor.fetch_arrow() can be converted to pandas DataFrame."""
        pandas = pytest.importorskip("pandas")
        pyarrow = pytest.importorskip("pyarrow")
        from pyhdb_rs import ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            cursor = conn.cursor()
            cursor.execute("SELECT 1 AS value FROM DUMMY")
            reader = cursor.fetch_arrow()
            pa_reader = pyarrow.RecordBatchReader.from_stream(reader)
            df = pa_reader.read_all().to_pandas()
            assert isinstance(df, pandas.DataFrame)
            assert len(df) == 1
        finally:
            conn.close()

    def test_cursor_fetch_arrow_with_arrow_config(self, hana_uri: str) -> None:
        """Test cursor.fetch_arrow() with ArrowConfig."""
        pandas = pytest.importorskip("pandas")
        pyarrow = pytest.importorskip("pyarrow")
        from pyhdb_rs import ArrowConfig, ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            cursor = conn.cursor()
            cursor.execute("SELECT 1 AS value FROM DUMMY")
            config = ArrowConfig(batch_size=100)
            reader = cursor.fetch_arrow(config=config)
            pa_reader = pyarrow.RecordBatchReader.from_stream(reader)
            df = pa_reader.read_all().to_pandas()
            assert isinstance(df, pandas.DataFrame)
            assert len(df) == 1
        finally:
            conn.close()

    def test_cursor_fetch_arrow_with_parameters(self, hana_uri: str) -> None:
        """Test cursor with parameters followed by fetch_arrow."""
        pytest.importorskip("pandas")
        pyarrow = pytest.importorskip("pyarrow")
        from pyhdb_rs import ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            cursor = conn.cursor()
            cursor.execute("SELECT ? AS value FROM DUMMY", [42])
            reader = cursor.fetch_arrow()
            pa_reader = pyarrow.RecordBatchReader.from_stream(reader)
            df = pa_reader.read_all().to_pandas()
            assert len(df) == 1
            assert df["VALUE"][0] == 42
        finally:
            conn.close()


class TestCursorExecuteArrowPandas:
    """Tests for cursor.execute_arrow() with pandas conversion."""

    def test_cursor_execute_arrow_to_pandas(self, hana_uri: str) -> None:
        """Test that cursor.execute_arrow() can be converted to pandas DataFrame."""
        pandas = pytest.importorskip("pandas")
        pyarrow = pytest.importorskip("pyarrow")
        from pyhdb_rs import ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            cursor = conn.cursor()
            reader = cursor.execute_arrow("SELECT 1 AS value FROM DUMMY")
            pa_reader = pyarrow.RecordBatchReader.from_stream(reader)
            df = pa_reader.read_all().to_pandas()
            assert isinstance(df, pandas.DataFrame)
            assert len(df) == 1
        finally:
            conn.close()

    def test_cursor_execute_arrow_with_arrow_config(self, hana_uri: str) -> None:
        """Test cursor.execute_arrow() with ArrowConfig."""
        pandas = pytest.importorskip("pandas")
        pyarrow = pytest.importorskip("pyarrow")
        from pyhdb_rs import ArrowConfig, ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            cursor = conn.cursor()
            config = ArrowConfig(batch_size=100)
            reader = cursor.execute_arrow("SELECT 1 AS value FROM DUMMY", config=config)
            pa_reader = pyarrow.RecordBatchReader.from_stream(reader)
            df = pa_reader.read_all().to_pandas()
            assert isinstance(df, pandas.DataFrame)
            assert len(df) == 1
        finally:
            conn.close()
