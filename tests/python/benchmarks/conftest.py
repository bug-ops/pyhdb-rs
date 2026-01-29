"""Pytest fixtures for benchmarks."""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Generator


def pytest_configure(config: pytest.Config) -> None:
    """Register custom markers."""
    config.addinivalue_line(
        "markers",
        "benchmark_group(name): mark test as belonging to a benchmark group",
    )


@pytest.fixture(scope="session")
def benchmark_data_sizes() -> list[int]:
    """Standard data sizes for benchmarks."""
    return [1_000, 10_000, 100_000]


@pytest.fixture(scope="session")
def large_data_sizes() -> list[int]:
    """Large data sizes for stress testing."""
    return [100_000, 500_000, 1_000_000]


@pytest.fixture
def mock_arrow_data() -> Generator[dict[str, list], None, None]:
    """Generate mock Arrow-compatible data for benchmarks."""
    import random

    random.seed(42)

    def generate(size: int) -> dict[str, list]:
        return {
            "id": list(range(size)),
            "customer": [f"customer_{i % 1000}" for i in range(size)],
            "product": [f"product_{i % 500}" for i in range(size)],
            "amount": [round(random.uniform(1.0, 10000.0), 2) for _ in range(size)],
            "quantity": [random.randint(1, 100) for _ in range(size)],
            "discount": [round(random.uniform(0.0, 0.3), 4) for _ in range(size)],
            "is_priority": [i % 3 == 0 for i in range(size)],
            "region": [f"region_{i % 10}" for i in range(size)],
        }

    yield generate
