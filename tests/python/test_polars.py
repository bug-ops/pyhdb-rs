"""Polars helper tests for pyhdb_rs."""

from __future__ import annotations

import contextlib

import pytest


class TestReadHana:
    """Tests for pyhdb_rs.polars.read_hana."""

    def test_read_hana_returns_dataframe(self, hana_uri: str) -> None:
        """Test that read_hana returns a Polars DataFrame."""
        polars = pytest.importorskip("polars")
        import pyhdb_rs.polars as hdb

        df = hdb.read_hana("SELECT 1 AS value FROM DUMMY", hana_uri)
        assert isinstance(df, polars.DataFrame)
        assert len(df) == 1

    def test_read_hana_with_batch_size(self, hana_uri: str) -> None:
        """Test read_hana with custom batch size."""
        polars = pytest.importorskip("polars")
        import pyhdb_rs.polars as hdb

        df = hdb.read_hana("SELECT 1 AS value FROM DUMMY", hana_uri, batch_size=100)
        assert isinstance(df, polars.DataFrame)
        assert len(df) == 1

    def test_read_hana_multiple_columns(self, hana_uri: str) -> None:
        """Test read_hana with multiple columns."""
        pytest.importorskip("polars")
        import pyhdb_rs.polars as hdb

        df = hdb.read_hana(
            "SELECT 1 AS int_col, 'hello' AS str_col FROM DUMMY",
            hana_uri,
        )
        assert len(df.columns) == 2


class TestScanHana:
    """Tests for pyhdb_rs.polars.scan_hana."""

    def test_scan_hana_returns_lazyframe(self, hana_uri: str) -> None:
        """Test that scan_hana returns a LazyFrame."""
        polars = pytest.importorskip("polars")
        import pyhdb_rs.polars as hdb

        lf = hdb.scan_hana("SELECT 1 AS value FROM DUMMY", hana_uri)
        assert isinstance(lf, polars.LazyFrame)

    def test_scan_hana_collect(self, hana_uri: str) -> None:
        """Test that scan_hana can be collected."""
        polars = pytest.importorskip("polars")
        import pyhdb_rs.polars as hdb

        lf = hdb.scan_hana("SELECT 1 AS value FROM DUMMY", hana_uri)
        df = lf.collect()
        assert isinstance(df, polars.DataFrame)
        assert len(df) == 1

    def test_scan_hana_filter(self, hana_uri: str) -> None:
        """Test that scan_hana supports filter operations."""
        polars = pytest.importorskip("polars")
        import pyhdb_rs.polars as hdb

        lf = hdb.scan_hana("SELECT 1 AS value FROM DUMMY", hana_uri)
        col_name = lf.collect().columns[0]
        result = lf.filter(polars.col(col_name) > 0).collect()
        assert len(result) == 1


class TestWriteHana:
    """Tests for pyhdb_rs.polars.write_hana.

    These tests require a writable HANA schema.
    """

    @pytest.fixture
    def test_table_name(self) -> str:
        """Generate a unique test table name."""
        import uuid

        return f"TEST_POLARS_{uuid.uuid4().hex[:8].upper()}"

    def test_write_hana_replace(
        self, hana_uri: str, test_table_name: str, connection: object
    ) -> None:
        """Test write_hana with if_table_exists='replace'."""
        polars = pytest.importorskip("polars")
        import pyhdb_rs
        import pyhdb_rs.polars as hdb

        conn = pyhdb_rs.connect(hana_uri)

        try:
            df = polars.DataFrame(
                {
                    "id": [1, 2, 3],
                    "value": [10, 20, 30],
                }
            )

            rows = hdb.write_hana(df, test_table_name, hana_uri, if_table_exists="replace")
            assert rows == 3

            result = hdb.read_hana(f"SELECT * FROM {test_table_name}", hana_uri)
            assert len(result) == 3
        finally:
            cursor = conn.cursor()
            with contextlib.suppress(Exception):
                cursor.execute(f"DROP TABLE {test_table_name}")
            conn.close()

    def test_write_hana_append(self, hana_uri: str, test_table_name: str) -> None:
        """Test write_hana with if_table_exists='append'."""
        polars = pytest.importorskip("polars")
        import pyhdb_rs
        import pyhdb_rs.polars as hdb

        conn = pyhdb_rs.connect(hana_uri)

        try:
            df1 = polars.DataFrame(
                {
                    "id": [1, 2],
                    "value": [10, 20],
                }
            )
            hdb.write_hana(df1, test_table_name, hana_uri, if_table_exists="replace")

            df2 = polars.DataFrame(
                {
                    "id": [3, 4],
                    "value": [30, 40],
                }
            )
            rows = hdb.write_hana(df2, test_table_name, hana_uri, if_table_exists="append")
            assert rows == 2

            result = hdb.read_hana(f"SELECT * FROM {test_table_name}", hana_uri)
            assert len(result) == 4
        finally:
            cursor = conn.cursor()
            with contextlib.suppress(Exception):
                cursor.execute(f"DROP TABLE {test_table_name}")
            conn.close()
