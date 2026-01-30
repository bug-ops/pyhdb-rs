//! Profiling utilities for heap analysis.
//!
//! This module provides helpers for dhat heap profiling. Enable with `--features profiling`.
//!
//! # Usage
//!
//! ```rust,ignore
//! use hdbconnect_arrow::profiling;
//!
//! fn main() {
//!     let _profiler = profiling::init();
//!     // ... benchmark code ...
//!     // Profiler writes dhat-heap.json on drop
//! }
//! ```

/// Initialize dhat heap profiler.
///
/// Returns a `Profiler` that writes `dhat-heap.json` to the current working
/// directory on drop. The global allocator must be set to `dhat::Alloc` for
/// this to work.
#[must_use]
pub fn init() -> dhat::Profiler {
    dhat::Profiler::new_heap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiler_initializes() {
        let _profiler = init();
        // Allocate something to verify profiler is active
        let _vec: Vec<u8> = vec![0; 1024];
    }
}

#[cfg(all(test, feature = "test-utils"))]
mod workload_tests {
    use std::hint::black_box;
    use std::sync::Arc;

    use arrow_schema::{DataType, Field, Schema};

    use super::*;
    use crate::traits::row::MockRowBuilder;
    use crate::traits::streaming::BatchConfig;
    use crate::{HanaBatchProcessor, MockRow};

    fn create_int64_rows(count: usize) -> Vec<MockRow> {
        (0..count)
            .map(|i| MockRowBuilder::new().bigint(i as i64).build())
            .collect()
    }

    fn create_decimal_rows(count: usize) -> Vec<MockRow> {
        (0..count)
            .map(|i| {
                MockRowBuilder::new()
                    .decimal_str(&format!("{}.{:02}", i, i % 100))
                    .build()
            })
            .collect()
    }

    fn create_analytics_rows(count: usize) -> Vec<MockRow> {
        (0..count)
            .map(|i| {
                MockRowBuilder::new()
                    .bigint(i as i64)
                    .string(format!("customer_{}", i % 1000))
                    .string(format!("product_{}", i % 500))
                    .decimal_str(&format!("{}.{:02}", (i * 10) % 10000, i % 100))
                    .int((i % 100) as i32)
                    .double(i as f64 * 0.15)
                    .boolean(i % 3 == 0)
                    .string(format!("region_{}", i % 10))
                    .build()
            })
            .collect()
    }

    fn int64_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![Field::new(
            "value",
            DataType::Int64,
            false,
        )]))
    }

    fn decimal_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![Field::new(
            "value",
            DataType::Decimal128(18, 2),
            false,
        )]))
    }

    fn analytics_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("customer", DataType::Utf8, false),
            Field::new("product", DataType::Utf8, false),
            Field::new("amount", DataType::Decimal128(18, 2), false),
            Field::new("quantity", DataType::Int32, false),
            Field::new("discount", DataType::Float64, false),
            Field::new("is_priority", DataType::Boolean, false),
            Field::new("region", DataType::Utf8, false),
        ]))
    }

    fn process_rows(processor: &mut HanaBatchProcessor, rows: &[MockRow]) -> usize {
        let mut batch_count = 0;
        for row in rows {
            if let Ok(Some(_batch)) = processor.process_row_generic(row) {
                batch_count += 1;
            }
        }
        if let Ok(Some(_batch)) = processor.flush() {
            batch_count += 1;
        }
        batch_count
    }

    #[test]
    #[ignore]
    fn profile_int64_100k() {
        let _profiler = init();

        let rows = create_int64_rows(100_000);
        let schema = int64_schema();

        for _ in 0..10 {
            let config = BatchConfig::with_batch_size(8192);
            let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
            black_box(process_rows(&mut processor, &rows));
        }
    }

    #[test]
    #[ignore]
    fn profile_decimal_conversion_100k() {
        let _profiler = init();

        let rows = create_decimal_rows(100_000);
        let schema = decimal_schema();

        for _ in 0..10 {
            let config = BatchConfig::with_batch_size(8192);
            let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
            black_box(process_rows(&mut processor, &rows));
        }
    }

    #[test]
    #[ignore]
    fn profile_analytics_workload_100k() {
        let _profiler = init();

        let rows = create_analytics_rows(100_000);
        let schema = analytics_schema();

        for _ in 0..10 {
            let config = BatchConfig::with_batch_size(8192);
            let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
            black_box(process_rows(&mut processor, &rows));
        }
    }

    #[test]
    #[ignore]
    fn profile_decimal_conversion_1m() {
        let _profiler = init();

        let rows = create_decimal_rows(1_000_000);
        let schema = decimal_schema();

        let config = BatchConfig::with_batch_size(65536);
        let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
        black_box(process_rows(&mut processor, &rows));
    }
}
