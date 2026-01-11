"""Pandas helper tests for pyhdb_rs."""

from __future__ import annotations

import contextlib

import pytest


class TestReadHana:
    """Tests for pyhdb_rs.pandas.read_hana."""

    def test_read_hana_returns_dataframe(self, hana_uri: str) -> None:
        """Test that read_hana returns a pandas DataFrame."""
        pandas = pytest.importorskip("pandas")
        pytest.importorskip("pyarrow")
        import pyhdb_rs.pandas as hdb

        df = hdb.read_hana("SELECT 1 AS value FROM DUMMY", hana_uri)
        assert isinstance(df, pandas.DataFrame)
        assert len(df) == 1

    def test_read_hana_with_batch_size(self, hana_uri: str) -> None:
        """Test read_hana with custom batch size."""
        pandas = pytest.importorskip("pandas")
        pytest.importorskip("pyarrow")
        import pyhdb_rs.pandas as hdb

        df = hdb.read_hana("SELECT 1 AS value FROM DUMMY", hana_uri, batch_size=100)
        assert isinstance(df, pandas.DataFrame)
        assert len(df) == 1

    def test_read_hana_multiple_columns(self, hana_uri: str) -> None:
        """Test read_hana with multiple columns."""
        pytest.importorskip("pandas")
        pytest.importorskip("pyarrow")
        import pyhdb_rs.pandas as hdb

        df = hdb.read_hana(
            "SELECT 1 AS int_col, 'hello' AS str_col FROM DUMMY",
            hana_uri,
        )
        assert len(df.columns) == 2


class TestToHana:
    """Tests for pyhdb_rs.pandas.to_hana.

    These tests require a writable HANA schema.
    """

    @pytest.fixture
    def test_table_name(self) -> str:
        """Generate a unique test table name."""
        import uuid

        return f"TEST_PANDAS_{uuid.uuid4().hex[:8].upper()}"

    def test_to_hana_replace(self, hana_uri: str, test_table_name: str) -> None:
        """Test to_hana with if_exists='replace'."""
        pandas = pytest.importorskip("pandas")
        pytest.importorskip("pyarrow")
        import pyhdb_rs
        import pyhdb_rs.pandas as hdb

        conn = pyhdb_rs.connect(hana_uri)

        try:
            df = pandas.DataFrame(
                {
                    "id": [1, 2, 3],
                    "value": [10, 20, 30],
                }
            )

            rows = hdb.to_hana(df, test_table_name, hana_uri, if_exists="replace")
            assert rows == 3

            result = hdb.read_hana(f"SELECT * FROM {test_table_name}", hana_uri)
            assert len(result) == 3
        finally:
            cursor = conn.cursor()
            with contextlib.suppress(Exception):
                cursor.execute(f"DROP TABLE {test_table_name}")
            conn.close()

    def test_to_hana_append(self, hana_uri: str, test_table_name: str) -> None:
        """Test to_hana with if_exists='append'."""
        pandas = pytest.importorskip("pandas")
        pytest.importorskip("pyarrow")
        import pyhdb_rs
        import pyhdb_rs.pandas as hdb

        conn = pyhdb_rs.connect(hana_uri)

        try:
            df1 = pandas.DataFrame(
                {
                    "id": [1, 2],
                    "value": [10, 20],
                }
            )
            hdb.to_hana(df1, test_table_name, hana_uri, if_exists="replace")

            df2 = pandas.DataFrame(
                {
                    "id": [3, 4],
                    "value": [30, 40],
                }
            )
            rows = hdb.to_hana(df2, test_table_name, hana_uri, if_exists="append")
            assert rows == 2

            result = hdb.read_hana(f"SELECT * FROM {test_table_name}", hana_uri)
            assert len(result) == 4
        finally:
            cursor = conn.cursor()
            with contextlib.suppress(Exception):
                cursor.execute(f"DROP TABLE {test_table_name}")
            conn.close()
