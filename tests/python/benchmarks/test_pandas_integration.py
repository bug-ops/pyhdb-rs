"""Pandas integration benchmarks.

Run with: pytest tests/python/benchmarks/test_pandas_integration.py --benchmark-only

Requires: pytest-benchmark, pandas, pyarrow
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from collections.abc import Callable

pd = pytest.importorskip("pandas")
pa = pytest.importorskip("pyarrow")


class TestPandasFromArrowBenchmarks:
    """Benchmarks for Pandas DataFrame creation from Arrow data."""

    @pytest.mark.benchmark(group="pandas_from_dict")
    def test_pandas_from_dict_1k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Baseline: Create Pandas DataFrame from Python dict (1K rows)."""
        data = mock_arrow_data(1_000)
        benchmark(pd.DataFrame, data)

    @pytest.mark.benchmark(group="pandas_from_dict")
    def test_pandas_from_dict_10k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Baseline: Create Pandas DataFrame from Python dict (10K rows)."""
        data = mock_arrow_data(10_000)
        benchmark(pd.DataFrame, data)

    @pytest.mark.benchmark(group="pandas_from_dict")
    def test_pandas_from_dict_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Baseline: Create Pandas DataFrame from Python dict (100K rows)."""
        data = mock_arrow_data(100_000)
        benchmark(pd.DataFrame, data)


class TestPandasArrowConversionBenchmarks:
    """Benchmarks for Arrow-Pandas conversion (key for pyhdb-rs integration)."""

    @pytest.mark.benchmark(group="pandas_arrow_conversion")
    def test_arrow_to_pandas_1k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Convert Arrow Table to Pandas DataFrame (1K rows)."""
        data = mock_arrow_data(1_000)
        table = pa.Table.from_pydict(data)
        benchmark(table.to_pandas)

    @pytest.mark.benchmark(group="pandas_arrow_conversion")
    def test_arrow_to_pandas_10k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Convert Arrow Table to Pandas DataFrame (10K rows)."""
        data = mock_arrow_data(10_000)
        table = pa.Table.from_pydict(data)
        benchmark(table.to_pandas)

    @pytest.mark.benchmark(group="pandas_arrow_conversion")
    def test_arrow_to_pandas_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Convert Arrow Table to Pandas DataFrame (100K rows)."""
        data = mock_arrow_data(100_000)
        table = pa.Table.from_pydict(data)
        benchmark(table.to_pandas)

    @pytest.mark.benchmark(group="pandas_arrow_conversion")
    def test_arrow_to_pandas_zero_copy_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Convert Arrow Table to Pandas with zero-copy (100K rows)."""
        data = mock_arrow_data(100_000)

        def to_pandas_zero_copy() -> pd.DataFrame:
            table = pa.Table.from_pydict(data)
            return table.to_pandas(self_destruct=True)

        benchmark(to_pandas_zero_copy)

    @pytest.mark.benchmark(group="pandas_arrow_conversion")
    def test_arrow_batches_to_pandas_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Convert Arrow RecordBatches to Pandas (100K rows, batched)."""
        data = mock_arrow_data(100_000)
        table = pa.Table.from_pydict(data)
        batches = table.to_batches(max_chunksize=10_000)

        def batches_to_pandas() -> pd.DataFrame:
            return pa.Table.from_batches(batches).to_pandas()

        benchmark(batches_to_pandas)


class TestPandasOperationsBenchmarks:
    """Benchmarks for common Pandas operations that would follow Arrow import."""

    @pytest.mark.benchmark(group="pandas_operations")
    def test_pandas_filter_1k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Filter operation on 1K row DataFrame."""
        df = pd.DataFrame(mock_arrow_data(1_000))
        benchmark(lambda: df[df["is_priority"]])

    @pytest.mark.benchmark(group="pandas_operations")
    def test_pandas_filter_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Filter operation on 100K row DataFrame."""
        df = pd.DataFrame(mock_arrow_data(100_000))
        benchmark(lambda: df[df["is_priority"]])

    @pytest.mark.benchmark(group="pandas_operations")
    def test_pandas_groupby_1k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """GroupBy aggregation on 1K row DataFrame."""
        df = pd.DataFrame(mock_arrow_data(1_000))
        benchmark(
            lambda: df.groupby("region").agg(
                total_amount=("amount", "sum"),
                avg_quantity=("quantity", "mean"),
            )
        )

    @pytest.mark.benchmark(group="pandas_operations")
    def test_pandas_groupby_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """GroupBy aggregation on 100K row DataFrame."""
        df = pd.DataFrame(mock_arrow_data(100_000))
        benchmark(
            lambda: df.groupby("region").agg(
                total_amount=("amount", "sum"),
                avg_quantity=("quantity", "mean"),
            )
        )

    @pytest.mark.benchmark(group="pandas_operations")
    def test_pandas_sort_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Sort operation on 100K row DataFrame."""
        df = pd.DataFrame(mock_arrow_data(100_000))
        benchmark(lambda: df.sort_values("amount", ascending=False))


class TestPandasMemoryBenchmarks:
    """Benchmarks focused on memory efficiency patterns."""

    @pytest.mark.benchmark(group="pandas_memory")
    def test_pandas_copy_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Measure DataFrame copy overhead (100K rows)."""
        df = pd.DataFrame(mock_arrow_data(100_000))
        benchmark(df.copy)

    @pytest.mark.benchmark(group="pandas_memory")
    def test_pandas_dtypes_optimization_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Optimize dtypes for memory efficiency (100K rows)."""
        data = mock_arrow_data(100_000)
        df = pd.DataFrame(data)

        def optimize_dtypes() -> pd.DataFrame:
            result = df.copy()
            result["quantity"] = result["quantity"].astype("int16")
            result["is_priority"] = result["is_priority"].astype("bool")
            return result

        benchmark(optimize_dtypes)
