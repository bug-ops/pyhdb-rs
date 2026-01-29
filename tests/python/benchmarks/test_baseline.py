"""Baseline benchmarks for comparison targets.

Run with: pytest tests/python/benchmarks/test_baseline.py --benchmark-only

These benchmarks establish baselines for:
1. Pure Python data creation overhead
2. PyArrow operations overhead
3. Memory allocation patterns

Goal: pyhdb-rs should achieve >=2x performance vs hdbcli on bulk reads.
These baselines help measure where we stand.
"""

from __future__ import annotations

import datetime
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from collections.abc import Callable

pa = pytest.importorskip("pyarrow")


class TestDataCreationBaseline:
    """Baseline benchmarks for data creation overhead."""

    @pytest.mark.benchmark(group="data_creation")
    def test_list_creation_1k(self, benchmark: Any) -> None:
        """Create Python list of 1K integers."""
        benchmark(lambda: list(range(1_000)))

    @pytest.mark.benchmark(group="data_creation")
    def test_list_creation_100k(self, benchmark: Any) -> None:
        """Create Python list of 100K integers."""
        benchmark(lambda: list(range(100_000)))

    @pytest.mark.benchmark(group="data_creation")
    def test_list_creation_1m(self, benchmark: Any) -> None:
        """Create Python list of 1M integers."""
        benchmark(lambda: list(range(1_000_000)))

    @pytest.mark.benchmark(group="data_creation")
    def test_dict_creation_1k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Create Python dict with 1K rows (analytics schema)."""
        benchmark(mock_arrow_data, 1_000)

    @pytest.mark.benchmark(group="data_creation")
    def test_dict_creation_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Create Python dict with 100K rows (analytics schema)."""
        benchmark(mock_arrow_data, 100_000)


class TestPyArrowTableCreation:
    """Benchmarks for PyArrow Table creation (what pyhdb-rs produces)."""

    @pytest.mark.benchmark(group="pyarrow_table")
    def test_pyarrow_table_from_dict_1k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Create PyArrow Table from dict (1K rows)."""
        data = mock_arrow_data(1_000)
        benchmark(pa.Table.from_pydict, data)

    @pytest.mark.benchmark(group="pyarrow_table")
    def test_pyarrow_table_from_dict_10k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Create PyArrow Table from dict (10K rows)."""
        data = mock_arrow_data(10_000)
        benchmark(pa.Table.from_pydict, data)

    @pytest.mark.benchmark(group="pyarrow_table")
    def test_pyarrow_table_from_dict_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Create PyArrow Table from dict (100K rows)."""
        data = mock_arrow_data(100_000)
        benchmark(pa.Table.from_pydict, data)

    @pytest.mark.benchmark(group="pyarrow_table")
    def test_pyarrow_table_from_dict_1m(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Create PyArrow Table from dict (1M rows)."""
        data = mock_arrow_data(1_000_000)
        benchmark(pa.Table.from_pydict, data)


class TestPyArrowBatchCreation:
    """Benchmarks for PyArrow RecordBatch creation (streaming pattern)."""

    @pytest.mark.benchmark(group="pyarrow_batch")
    def test_pyarrow_batch_int64_1k(self, benchmark: Any) -> None:
        """Create single-column Int64 batch (1K rows)."""
        data = list(range(1_000))

        def create_batch() -> pa.RecordBatch:
            return pa.RecordBatch.from_pydict({"value": data})

        benchmark(create_batch)

    @pytest.mark.benchmark(group="pyarrow_batch")
    def test_pyarrow_batch_int64_10k(self, benchmark: Any) -> None:
        """Create single-column Int64 batch (10K rows)."""
        data = list(range(10_000))

        def create_batch() -> pa.RecordBatch:
            return pa.RecordBatch.from_pydict({"value": data})

        benchmark(create_batch)

    @pytest.mark.benchmark(group="pyarrow_batch")
    def test_pyarrow_batch_mixed_10k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Create mixed-type batch (10K rows, analytics schema)."""
        data = mock_arrow_data(10_000)

        def create_batch() -> pa.RecordBatch:
            return pa.RecordBatch.from_pydict(data)

        benchmark(create_batch)


class TestPyArrowStreamingPattern:
    """Benchmarks simulating streaming RecordBatch patterns."""

    @pytest.mark.benchmark(group="pyarrow_streaming")
    def test_streaming_batches_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Stream 100K rows in 10K batches."""
        batch_size = 10_000
        num_batches = 10

        def stream_batches() -> list[pa.RecordBatch]:
            batches = []
            for _ in range(num_batches):
                data = mock_arrow_data(batch_size)
                batches.append(pa.RecordBatch.from_pydict(data))
            return batches

        benchmark(stream_batches)

    @pytest.mark.benchmark(group="pyarrow_streaming")
    def test_batches_to_table_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Combine batches into Table (100K rows from 10 batches)."""
        batch_size = 10_000
        batches = [pa.RecordBatch.from_pydict(mock_arrow_data(batch_size)) for _ in range(10)]
        benchmark(pa.Table.from_batches, batches)


class TestBulkReadSimulation:
    """Simulate bulk read patterns (target: >=2x vs hdbcli)."""

    @pytest.mark.benchmark(group="bulk_read_simulation")
    def test_bulk_read_int64_1m(self, benchmark: Any) -> None:
        """Simulate 1M row bulk read (single Int64 column)."""
        data = {"value": list(range(1_000_000))}

        def bulk_read() -> pa.Table:
            return pa.Table.from_pydict(data)

        benchmark(bulk_read)

    @pytest.mark.benchmark(group="bulk_read_simulation")
    def test_bulk_read_analytics_100k(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Simulate 100K row bulk read (analytics schema, 8 columns)."""
        data = mock_arrow_data(100_000)

        def bulk_read() -> pa.Table:
            return pa.Table.from_pydict(data)

        benchmark(bulk_read)

    @pytest.mark.benchmark(group="bulk_read_simulation")
    def test_bulk_read_analytics_1m(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Simulate 1M row bulk read (analytics schema, 8 columns)."""
        data = mock_arrow_data(1_000_000)

        def bulk_read() -> pa.Table:
            return pa.Table.from_pydict(data)

        benchmark(bulk_read)

    @pytest.mark.benchmark(group="bulk_read_simulation")
    def test_bulk_read_streaming_1m(
        self, benchmark: Any, mock_arrow_data: Callable[[int], dict[str, list]]
    ) -> None:
        """Simulate streaming 1M rows in 64K batches (realistic pattern)."""
        batch_size = 65536
        num_batches = 16  # ~1M rows

        def streaming_read() -> pa.Table:
            batches = []
            for _ in range(num_batches):
                data = mock_arrow_data(batch_size)
                batches.append(pa.RecordBatch.from_pydict(data))
            return pa.Table.from_batches(batches)

        benchmark(streaming_read)


class TestTypeConversionBaseline:
    """Baseline benchmarks for type conversion overhead."""

    @pytest.mark.benchmark(group="type_conversion")
    def test_decimal_creation_10k(self, benchmark: Any) -> None:
        """Create Decimal128 array (10K values)."""
        from decimal import Decimal

        data = [Decimal(f"{i}.{i % 100:02d}") for i in range(10_000)]

        def create_array() -> pa.Array:
            return pa.array(data, type=pa.decimal128(18, 2))

        benchmark(create_array)

    @pytest.mark.benchmark(group="type_conversion")
    def test_timestamp_creation_10k(self, benchmark: Any) -> None:
        """Create Timestamp array (10K values)."""
        base = datetime.datetime(2024, 1, 1, tzinfo=datetime.UTC)
        data = [base + datetime.timedelta(seconds=i) for i in range(10_000)]

        def create_array() -> pa.Array:
            return pa.array(data, type=pa.timestamp("ns"))

        benchmark(create_array)

    @pytest.mark.benchmark(group="type_conversion")
    def test_string_creation_10k(self, benchmark: Any) -> None:
        """Create String array (10K values, 50-char strings)."""
        data = [f"string_value_{i:010d}_padding_to_50_chars" for i in range(10_000)]

        def create_array() -> pa.Array:
            return pa.array(data, type=pa.utf8())

        benchmark(create_array)

    @pytest.mark.benchmark(group="type_conversion")
    def test_binary_creation_1k_1kb(self, benchmark: Any) -> None:
        """Create Binary array (1K values, 1KB each)."""
        data = [bytes(range(256)) * 4 for _ in range(1_000)]

        def create_array() -> pa.Array:
            return pa.array(data, type=pa.binary())

        benchmark(create_array)
