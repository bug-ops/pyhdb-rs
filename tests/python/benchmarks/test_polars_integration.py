"""Polars integration benchmarks.

Run with: pytest tests/python/benchmarks/test_polars_integration.py --benchmark-only

Requires: pytest-benchmark, polars
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from collections.abc import Callable


pl = pytest.importorskip("polars")


class TestPolarsFromArrowBenchmarks:
    """Benchmarks for Polars DataFrame creation from Arrow data."""

    @pytest.mark.benchmark(group="polars_from_dict")
    def test_polars_from_dict_1k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Baseline: Create Polars DataFrame from Python dict (1K rows)."""
        data = mock_arrow_data(1_000)
        benchmark(pl.DataFrame, data)

    @pytest.mark.benchmark(group="polars_from_dict")
    def test_polars_from_dict_10k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Baseline: Create Polars DataFrame from Python dict (10K rows)."""
        data = mock_arrow_data(10_000)
        benchmark(pl.DataFrame, data)

    @pytest.mark.benchmark(group="polars_from_dict")
    def test_polars_from_dict_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Baseline: Create Polars DataFrame from Python dict (100K rows)."""
        data = mock_arrow_data(100_000)
        benchmark(pl.DataFrame, data)


class TestPolarsOperationsBenchmarks:
    """Benchmarks for common Polars operations that would follow Arrow import."""

    @pytest.mark.benchmark(group="polars_operations")
    def test_polars_filter_1k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Filter operation on 1K row DataFrame."""
        df = pl.DataFrame(mock_arrow_data(1_000))
        benchmark(lambda: df.filter(pl.col("is_priority")))

    @pytest.mark.benchmark(group="polars_operations")
    def test_polars_filter_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Filter operation on 100K row DataFrame."""
        df = pl.DataFrame(mock_arrow_data(100_000))
        benchmark(lambda: df.filter(pl.col("is_priority")))

    @pytest.mark.benchmark(group="polars_operations")
    def test_polars_groupby_1k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """GroupBy aggregation on 1K row DataFrame."""
        df = pl.DataFrame(mock_arrow_data(1_000))
        benchmark(
            lambda: df.group_by("region").agg(
                pl.col("amount").sum().alias("total_amount"),
                pl.col("quantity").mean().alias("avg_quantity"),
            )
        )

    @pytest.mark.benchmark(group="polars_operations")
    def test_polars_groupby_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """GroupBy aggregation on 100K row DataFrame."""
        df = pl.DataFrame(mock_arrow_data(100_000))
        benchmark(
            lambda: df.group_by("region").agg(
                pl.col("amount").sum().alias("total_amount"),
                pl.col("quantity").mean().alias("avg_quantity"),
            )
        )

    @pytest.mark.benchmark(group="polars_operations")
    def test_polars_sort_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Sort operation on 100K row DataFrame."""
        df = pl.DataFrame(mock_arrow_data(100_000))
        benchmark(lambda: df.sort("amount", descending=True))


class TestPolarsArrowInteropBenchmarks:
    """Benchmarks for Arrow interoperability (simulates pyhdb-rs integration)."""

    @pytest.mark.benchmark(group="polars_arrow_interop")
    def test_polars_to_arrow_table_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Convert Polars DataFrame to Arrow Table (100K rows)."""
        df = pl.DataFrame(mock_arrow_data(100_000))
        benchmark(df.to_arrow)

    @pytest.mark.benchmark(group="polars_arrow_interop")
    def test_polars_from_arrow_table_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Convert Arrow Table to Polars DataFrame (100K rows)."""
        df = pl.DataFrame(mock_arrow_data(100_000))
        table = df.to_arrow()
        benchmark(pl.from_arrow, table)

    @pytest.mark.benchmark(group="polars_arrow_interop")
    def test_polars_from_arrow_batches_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Convert Arrow RecordBatches to Polars (100K rows, batched)."""
        df = pl.DataFrame(mock_arrow_data(100_000))
        table = df.to_arrow()
        batches = table.to_batches(max_chunksize=10_000)

        def from_batches() -> pl.DataFrame:
            import pyarrow as pa

            return pl.from_arrow(pa.Table.from_batches(batches))

        benchmark(from_batches)


class TestPolarsMemoryBenchmarks:
    """Benchmarks focused on memory efficiency patterns."""

    @pytest.mark.benchmark(group="polars_memory")
    def test_polars_lazy_scan_simulation_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Simulate lazy scan with filter pushdown (100K rows)."""
        df = pl.DataFrame(mock_arrow_data(100_000))

        def lazy_query() -> pl.DataFrame:
            return (
                df.lazy()
                .filter(pl.col("is_priority"))
                .select(["id", "customer", "amount"])
                .collect()
            )

        benchmark(lazy_query)

    @pytest.mark.benchmark(group="polars_memory")
    def test_polars_streaming_groupby_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Streaming groupby operation (100K rows)."""
        df = pl.DataFrame(mock_arrow_data(100_000))

        def streaming_groupby() -> pl.DataFrame:
            return df.lazy().group_by("region").agg(pl.col("amount").sum()).collect(streaming=True)

        benchmark(streaming_groupby)
