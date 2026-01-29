"""Tests for Polars integration via execute_arrow() API.

These tests verify that the execute_arrow() API works correctly with Polars
for converting HANA query results to Polars DataFrames.
"""

from __future__ import annotations

import pytest


class TestExecuteArrowPolars:
    """Tests for execute_arrow() with Polars conversion."""

    def test_execute_arrow_to_polars_returns_dataframe(self, hana_uri: str) -> None:
        """Test that execute_arrow() result can be converted to Polars DataFrame."""
        polars = pytest.importorskip("polars")
        from pyhdb_rs import ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            reader = conn.execute_arrow("SELECT 1 AS value FROM DUMMY")
            df = polars.from_arrow(reader)
            assert isinstance(df, polars.DataFrame)
            assert len(df) == 1
        finally:
            conn.close()

    def test_execute_arrow_with_arrow_config(self, hana_uri: str) -> None:
        """Test execute_arrow with ArrowConfig for custom batch size."""
        polars = pytest.importorskip("polars")
        from pyhdb_rs import ArrowConfig, ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            config = ArrowConfig(batch_size=100)
            reader = conn.execute_arrow("SELECT 1 AS value FROM DUMMY", config=config)
            df = polars.from_arrow(reader)
            assert isinstance(df, polars.DataFrame)
            assert len(df) == 1
        finally:
            conn.close()

    def test_execute_arrow_multiple_columns(self, hana_uri: str) -> None:
        """Test execute_arrow with multiple columns."""
        polars = pytest.importorskip("polars")
        from pyhdb_rs import ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            reader = conn.execute_arrow(
                "SELECT 1 AS int_col, 'hello' AS str_col FROM DUMMY"
            )
            df = polars.from_arrow(reader)
            assert len(df.columns) == 2
        finally:
            conn.close()


class TestCursorFetchArrowPolars:
    """Tests for cursor.fetch_arrow() with Polars conversion."""

    def test_cursor_fetch_arrow_to_polars(self, hana_uri: str) -> None:
        """Test that cursor.fetch_arrow() can be converted to Polars DataFrame."""
        polars = pytest.importorskip("polars")
        from pyhdb_rs import ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            cursor = conn.cursor()
            cursor.execute("SELECT 1 AS value FROM DUMMY")
            reader = cursor.fetch_arrow()
            df = polars.from_arrow(reader)
            assert isinstance(df, polars.DataFrame)
            assert len(df) == 1
        finally:
            conn.close()

    def test_cursor_fetch_arrow_with_arrow_config(self, hana_uri: str) -> None:
        """Test cursor.fetch_arrow() with ArrowConfig."""
        polars = pytest.importorskip("polars")
        from pyhdb_rs import ArrowConfig, ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            cursor = conn.cursor()
            cursor.execute("SELECT 1 AS value FROM DUMMY")
            config = ArrowConfig(batch_size=100)
            reader = cursor.fetch_arrow(config=config)
            df = polars.from_arrow(reader)
            assert isinstance(df, polars.DataFrame)
            assert len(df) == 1
        finally:
            conn.close()

    def test_cursor_fetch_arrow_with_parameters(self, hana_uri: str) -> None:
        """Test cursor with parameters followed by fetch_arrow."""
        polars = pytest.importorskip("polars")
        from pyhdb_rs import ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            cursor = conn.cursor()
            cursor.execute("SELECT ? AS value FROM DUMMY", [42])
            reader = cursor.fetch_arrow()
            df = polars.from_arrow(reader)
            assert len(df) == 1
            assert df["VALUE"][0] == 42
        finally:
            conn.close()


class TestCursorExecuteArrowPolars:
    """Tests for cursor.execute_arrow() with Polars conversion."""

    def test_cursor_execute_arrow_to_polars(self, hana_uri: str) -> None:
        """Test that cursor.execute_arrow() can be converted to Polars DataFrame."""
        polars = pytest.importorskip("polars")
        from pyhdb_rs import ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            cursor = conn.cursor()
            reader = cursor.execute_arrow("SELECT 1 AS value FROM DUMMY")
            df = polars.from_arrow(reader)
            assert isinstance(df, polars.DataFrame)
            assert len(df) == 1
        finally:
            conn.close()

    def test_cursor_execute_arrow_with_arrow_config(self, hana_uri: str) -> None:
        """Test cursor.execute_arrow() with ArrowConfig."""
        polars = pytest.importorskip("polars")
        from pyhdb_rs import ArrowConfig, ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            cursor = conn.cursor()
            config = ArrowConfig(batch_size=100)
            reader = cursor.execute_arrow("SELECT 1 AS value FROM DUMMY", config=config)
            df = polars.from_arrow(reader)
            assert isinstance(df, polars.DataFrame)
            assert len(df) == 1
        finally:
            conn.close()


class TestPolarsLazyFrame:
    """Tests for Polars LazyFrame integration."""

    def test_execute_arrow_to_lazyframe(self, hana_uri: str) -> None:
        """Test converting execute_arrow result to LazyFrame."""
        polars = pytest.importorskip("polars")
        from pyhdb_rs import ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            reader = conn.execute_arrow("SELECT 1 AS value FROM DUMMY")
            df = polars.from_arrow(reader)
            lf = df.lazy()
            assert isinstance(lf, polars.LazyFrame)
        finally:
            conn.close()

    def test_lazyframe_collect(self, hana_uri: str) -> None:
        """Test that LazyFrame can be collected."""
        polars = pytest.importorskip("polars")
        from pyhdb_rs import ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            reader = conn.execute_arrow("SELECT 1 AS value FROM DUMMY")
            lf = polars.from_arrow(reader).lazy()
            df = lf.collect()
            assert isinstance(df, polars.DataFrame)
            assert len(df) == 1
        finally:
            conn.close()

    def test_lazyframe_filter(self, hana_uri: str) -> None:
        """Test that LazyFrame supports filter operations."""
        polars = pytest.importorskip("polars")
        from pyhdb_rs import ConnectionBuilder

        conn = ConnectionBuilder.from_url(hana_uri).build()
        try:
            reader = conn.execute_arrow("SELECT 1 AS value FROM DUMMY")
            lf = polars.from_arrow(reader).lazy()
            col_name = lf.collect().columns[0]
            result = lf.filter(polars.col(col_name) > 0).collect()
            assert len(result) == 1
        finally:
            conn.close()
