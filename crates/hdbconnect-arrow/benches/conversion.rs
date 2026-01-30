//! Comprehensive benchmarks for HANA to Arrow conversion operations.
//!
//! Run with: `cargo bench --bench conversion --features test-utils`
//!
//! These benchmarks measure:
//! - Single-type conversion performance (Int32, Int64, Float64, Boolean, String)
//! - Decimal conversion performance (HANA DECIMAL -> Arrow Decimal128)
//! - Temporal conversion performance (Date32, Time64, Timestamp)
//! - Large binary/string (LOB) handling
//! - Mixed schema conversion performance
//! - Batch size impact on throughput
//! - Builder creation overhead
//! - Null handling performance
//! - Wide table performance (many columns)
//! - Bulk read simulation (1M+ rows)

use std::hint::black_box;
use std::sync::Arc;

use arrow_schema::{DataType, Field, Schema, TimeUnit};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use hdbconnect_arrow::traits::row::MockRowBuilder;
use hdbconnect_arrow::traits::streaming::BatchConfig;
use hdbconnect_arrow::{BuilderFactory, HanaBatchProcessor, MockRow};

/// Create test rows with Int32 values.
fn create_int32_rows(count: usize) -> Vec<MockRow> {
    (0..count)
        .map(|i| MockRowBuilder::new().int(i as i32).build())
        .collect()
}

/// Create test rows with Int64 values.
fn create_int64_rows(count: usize) -> Vec<MockRow> {
    (0..count)
        .map(|i| MockRowBuilder::new().bigint(i as i64).build())
        .collect()
}

/// Create test rows with Float64 values.
fn create_float64_rows(count: usize) -> Vec<MockRow> {
    (0..count)
        .map(|i| MockRowBuilder::new().double(i as f64 * 1.5).build())
        .collect()
}

/// Create test rows with Boolean values.
fn create_boolean_rows(count: usize) -> Vec<MockRow> {
    (0..count)
        .map(|i| MockRowBuilder::new().boolean(i % 2 == 0).build())
        .collect()
}

/// Create test rows with String values of specified length.
fn create_string_rows(count: usize, string_len: usize) -> Vec<MockRow> {
    let base_string: String = "x".repeat(string_len);
    (0..count)
        .map(|_| MockRowBuilder::new().string(base_string.clone()).build())
        .collect()
}

/// Create test rows with Decimal values (requires test-utils feature).
fn create_decimal_rows(count: usize) -> Vec<MockRow> {
    (0..count)
        .map(|i| {
            MockRowBuilder::new()
                .decimal_str(&format!("{}.{:02}", i, i % 100))
                .build()
        })
        .collect()
}

/// Create decimal rows with specific precision patterns (high precision).
fn create_decimal_rows_high_precision(count: usize) -> Vec<MockRow> {
    (0..count)
        .map(|i| {
            MockRowBuilder::new()
                .decimal_str(&format!(
                    "{}.{:08}",
                    i % 1_000_000_000,
                    (i * 12345678) % 100_000_000
                ))
                .build()
        })
        .collect()
}

/// Create decimal rows with large values (stress test BigDecimal -> i128).
fn create_decimal_rows_large_values(count: usize) -> Vec<MockRow> {
    (0..count)
        .map(|i| {
            MockRowBuilder::new()
                .decimal_str(&format!(
                    "{}{}.{:04}",
                    i % 1_000_000,
                    i % 1_000_000_000,
                    i % 10000
                ))
                .build()
        })
        .collect()
}

/// Create test rows with binary data of specified size.
fn create_binary_rows(count: usize, binary_len: usize) -> Vec<MockRow> {
    let data: Vec<u8> = (0..binary_len).map(|i| (i % 256) as u8).collect();
    (0..count)
        .map(|_| MockRowBuilder::new().binary(data.clone()).build())
        .collect()
}

/// Create test rows with mixed types (5 columns).
fn create_mixed_rows(count: usize) -> Vec<MockRow> {
    (0..count)
        .map(|i| {
            MockRowBuilder::new()
                .int(i as i32)
                .bigint(i as i64 * 100)
                .double(i as f64 * 1.5)
                .string(format!("row_{i}"))
                .boolean(i % 2 == 0)
                .build()
        })
        .collect()
}

/// Create test rows with all NULL values.
fn create_null_rows(count: usize, column_count: usize) -> Vec<MockRow> {
    (0..count).map(|_| MockRow::nulls(column_count)).collect()
}

/// Create wide table rows (many columns).
fn create_wide_rows(count: usize, column_count: usize) -> Vec<MockRow> {
    (0..count)
        .map(|i| {
            let mut builder = MockRowBuilder::new();
            for j in 0..column_count {
                builder = builder.int((i * column_count + j) as i32);
            }
            builder.build()
        })
        .collect()
}

/// Create mixed type wide rows (realistic analytics schema).
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

/// Create schema for Int32 column.
fn int32_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![Field::new(
        "value",
        DataType::Int32,
        false,
    )]))
}

/// Create schema for Int64 column.
fn int64_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![Field::new(
        "value",
        DataType::Int64,
        false,
    )]))
}

/// Create schema for Float64 column.
fn float64_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![Field::new(
        "value",
        DataType::Float64,
        false,
    )]))
}

/// Create schema for Boolean column.
fn boolean_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![Field::new(
        "value",
        DataType::Boolean,
        false,
    )]))
}

/// Create schema for Utf8 column.
fn string_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![Field::new(
        "value",
        DataType::Utf8,
        false,
    )]))
}

/// Create schema for Decimal128 column.
fn decimal_schema(precision: u8, scale: i8) -> Arc<Schema> {
    Arc::new(Schema::new(vec![Field::new(
        "value",
        DataType::Decimal128(precision, scale),
        false,
    )]))
}

/// Create schema for Binary column.
fn binary_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![Field::new(
        "value",
        DataType::Binary,
        false,
    )]))
}

/// Create schema for LargeBinary column (LOB).
fn large_binary_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![Field::new(
        "value",
        DataType::LargeBinary,
        false,
    )]))
}

/// Create schema for LargeUtf8 column (CLOB).
fn large_utf8_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![Field::new(
        "value",
        DataType::LargeUtf8,
        false,
    )]))
}

/// Create schema for Date32 column.
fn date32_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![Field::new(
        "value",
        DataType::Date32,
        true,
    )]))
}

/// Create schema for Time64 column.
fn time64_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![Field::new(
        "value",
        DataType::Time64(TimeUnit::Nanosecond),
        true,
    )]))
}

/// Create schema for Timestamp column.
fn timestamp_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![Field::new(
        "value",
        DataType::Timestamp(TimeUnit::Nanosecond, None),
        true,
    )]))
}

/// Create 5-column mixed schema.
fn mixed_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("count", DataType::Int64, false),
        Field::new("value", DataType::Float64, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("active", DataType::Boolean, false),
    ]))
}

/// Create wide table schema (many columns).
fn wide_schema(column_count: usize) -> Arc<Schema> {
    let fields: Vec<Field> = (0..column_count)
        .map(|i| Field::new(format!("col_{i}"), DataType::Int32, false))
        .collect();
    Arc::new(Schema::new(fields))
}

/// Create analytics schema (8 columns, mixed types).
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

/// Create schema with nullable columns for null handling benchmark.
fn nullable_schema(column_count: usize) -> Arc<Schema> {
    let fields: Vec<Field> = (0..column_count)
        .map(|i| Field::new(format!("col_{i}"), DataType::Int32, true))
        .collect();
    Arc::new(Schema::new(fields))
}

/// Process rows through the batch processor and collect results.
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

// ═══════════════════════════════════════════════════════════════════════════════
// Int32 Conversion Benchmarks
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_int32_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("int32_conversion");

    for size in [1_000, 10_000, 100_000] {
        let rows = create_int32_rows(size);
        let schema = int32_schema();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &rows, |b, rows| {
            b.iter(|| {
                let config = BatchConfig::with_batch_size(8192);
                let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
                black_box(process_rows(&mut processor, rows))
            });
        });
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Int64 Conversion Benchmarks
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_int64_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("int64_conversion");

    for size in [1_000, 10_000, 100_000] {
        let rows = create_int64_rows(size);
        let schema = int64_schema();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &rows, |b, rows| {
            b.iter(|| {
                let config = BatchConfig::with_batch_size(8192);
                let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
                black_box(process_rows(&mut processor, rows))
            });
        });
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Float64 Conversion Benchmarks
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_float64_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("float64_conversion");

    let size = 10_000;
    let rows = create_float64_rows(size);
    let schema = float64_schema();

    group.throughput(Throughput::Elements(size as u64));
    group.bench_function("10k_rows", |b| {
        b.iter(|| {
            let config = BatchConfig::with_batch_size(8192);
            let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
            black_box(process_rows(&mut processor, &rows))
        });
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Boolean Conversion Benchmarks
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_boolean_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("boolean_conversion");

    let size = 10_000;
    let rows = create_boolean_rows(size);
    let schema = boolean_schema();

    group.throughput(Throughput::Elements(size as u64));
    group.bench_function("10k_rows", |b| {
        b.iter(|| {
            let config = BatchConfig::with_batch_size(8192);
            let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
            black_box(process_rows(&mut processor, &rows))
        });
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// String Conversion Benchmarks
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_string_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_conversion");

    let row_count = 10_000;

    for string_len in [10, 100, 1000] {
        let rows = create_string_rows(row_count, string_len);
        let schema = string_schema();
        let total_bytes = row_count * string_len;

        group.throughput(Throughput::Bytes(total_bytes as u64));
        group.bench_with_input(BenchmarkId::new("len", string_len), &rows, |b, rows| {
            b.iter(|| {
                let config = BatchConfig::with_batch_size(8192);
                let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
                black_box(process_rows(&mut processor, rows))
            });
        });
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Decimal Conversion Benchmarks (CRITICAL for financial data)
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_decimal_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("decimal_conversion");

    for size in [1_000, 10_000, 100_000] {
        let rows = create_decimal_rows(size);
        let schema = decimal_schema(18, 2);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &rows, |b, rows| {
            b.iter(|| {
                let config = BatchConfig::with_batch_size(8192);
                let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
                black_box(process_rows(&mut processor, rows))
            });
        });
    }

    group.finish();
}

/// Benchmark decimal conversion with varying precision/scale combinations.
fn bench_decimal_precision_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("decimal_precision_scale");

    let size = 10_000;

    // Test different precision/scale combinations (BigDecimal -> i128 scaling)
    for (precision, scale) in [(10, 2), (18, 4), (28, 8), (38, 10)] {
        let rows = create_decimal_rows_high_precision(size);
        let schema = decimal_schema(precision, scale);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("precision_scale", format!("{precision}_{scale}")),
            &rows,
            |b, rows| {
                b.iter(|| {
                    let config = BatchConfig::with_batch_size(8192);
                    let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
                    black_box(process_rows(&mut processor, rows))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark decimal conversion with large values (stress BigDecimal -> i128).
fn bench_decimal_large_values(c: &mut Criterion) {
    let mut group = c.benchmark_group("decimal_large_values");

    for size in [1_000, 10_000] {
        let rows = create_decimal_rows_large_values(size);
        let schema = decimal_schema(38, 4);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &rows, |b, rows| {
            b.iter(|| {
                let config = BatchConfig::with_batch_size(8192);
                let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
                black_box(process_rows(&mut processor, rows))
            });
        });
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Temporal Conversion Benchmarks (null path - measures builder overhead)
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_temporal_null_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("temporal_null_handling");

    let size = 10_000;
    let null_rows = create_null_rows(size, 1);

    // Date32 (DAYDATE) null handling
    {
        let schema = date32_schema();
        group.throughput(Throughput::Elements(size as u64));
        group.bench_function("date32_nulls", |b| {
            b.iter(|| {
                let config = BatchConfig::with_batch_size(8192);
                let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
                black_box(process_rows(&mut processor, &null_rows))
            });
        });
    }

    // Time64 (SECONDTIME) null handling
    {
        let schema = time64_schema();
        group.throughput(Throughput::Elements(size as u64));
        group.bench_function("time64_nulls", |b| {
            b.iter(|| {
                let config = BatchConfig::with_batch_size(8192);
                let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
                black_box(process_rows(&mut processor, &null_rows))
            });
        });
    }

    // Timestamp (LONGDATE) null handling
    {
        let schema = timestamp_schema();
        group.throughput(Throughput::Elements(size as u64));
        group.bench_function("timestamp_nulls", |b| {
            b.iter(|| {
                let config = BatchConfig::with_batch_size(8192);
                let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
                black_box(process_rows(&mut processor, &null_rows))
            });
        });
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Binary/LOB Conversion Benchmarks
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_binary_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("binary_conversion");

    let row_count = 1_000;

    for binary_len in [100, 1_000, 10_000] {
        let rows = create_binary_rows(row_count, binary_len);
        let schema = binary_schema();
        let total_bytes = row_count * binary_len;

        group.throughput(Throughput::Bytes(total_bytes as u64));
        group.bench_with_input(
            BenchmarkId::new("size_bytes", binary_len),
            &rows,
            |b, rows| {
                b.iter(|| {
                    let config = BatchConfig::with_batch_size(8192);
                    let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
                    black_box(process_rows(&mut processor, rows))
                });
            },
        );
    }

    group.finish();
}

fn bench_large_binary_lob(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_binary_lob");

    let row_count = 100;

    // BLOB sizes: 10KB, 100KB, 1MB, 10MB
    for binary_len in [10_000, 100_000, 1_000_000, 10_000_000] {
        let rows = create_binary_rows(row_count, binary_len);
        let schema = large_binary_schema();
        let total_bytes = row_count * binary_len;

        group.throughput(Throughput::Bytes(total_bytes as u64));
        group.bench_with_input(
            BenchmarkId::new("size_bytes", binary_len),
            &rows,
            |b, rows| {
                b.iter(|| {
                    let config = BatchConfig::with_batch_size(1024);
                    let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
                    black_box(process_rows(&mut processor, rows))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark CLOB (LargeUtf8) with varying text sizes.
fn bench_large_utf8_clob(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_utf8_clob");

    let row_count = 100;

    // CLOB sizes: 1KB, 10KB, 100KB, 1MB
    for text_len in [1_000, 10_000, 100_000, 1_000_000] {
        let rows = create_string_rows(row_count, text_len);
        let schema = large_utf8_schema();
        let total_bytes = row_count * text_len;

        group.throughput(Throughput::Bytes(total_bytes as u64));
        group.bench_with_input(
            BenchmarkId::new("size_bytes", text_len),
            &rows,
            |b, rows| {
                b.iter(|| {
                    let config = BatchConfig::with_batch_size(1024);
                    let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
                    black_box(process_rows(&mut processor, rows))
                });
            },
        );
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Mixed Schema Benchmarks
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_mixed_schema(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_schema");

    let size = 100_000;
    let rows = create_mixed_rows(size);
    let schema = mixed_schema();

    group.throughput(Throughput::Elements(size as u64));
    group.bench_function("5_columns_100k_rows", |b| {
        b.iter(|| {
            let config = BatchConfig::with_batch_size(8192);
            let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
            black_box(process_rows(&mut processor, &rows))
        });
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Wide Table Benchmarks (many columns)
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_wide_table(c: &mut Criterion) {
    let mut group = c.benchmark_group("wide_table");

    let row_count = 10_000;

    for column_count in [10, 25, 50, 100] {
        let rows = create_wide_rows(row_count, column_count);
        let schema = wide_schema(column_count);

        group.throughput(Throughput::Elements((row_count * column_count) as u64));
        group.bench_with_input(
            BenchmarkId::new("columns", column_count),
            &rows,
            |b, rows| {
                b.iter(|| {
                    let config = BatchConfig::with_batch_size(8192);
                    let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
                    black_box(process_rows(&mut processor, rows))
                });
            },
        );
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Analytics Workload Benchmark (realistic mixed types)
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_analytics_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("analytics_workload");

    for size in [10_000, 100_000, 500_000] {
        let rows = create_analytics_rows(size);
        let schema = analytics_schema();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &rows, |b, rows| {
            b.iter(|| {
                let config = BatchConfig::with_batch_size(8192);
                let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
                black_box(process_rows(&mut processor, rows))
            });
        });
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Bulk Read Simulation (1M+ rows target: >=2x vs hdbcli)
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_bulk_read_1m(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulk_read_1m");
    group.sample_size(10);

    let size = 1_000_000;
    let rows = create_int64_rows(size);
    let schema = int64_schema();

    group.throughput(Throughput::Elements(size as u64));
    group.bench_function("int64_1m_rows", |b| {
        b.iter(|| {
            let config = BatchConfig::with_batch_size(65536);
            let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
            black_box(process_rows(&mut processor, &rows))
        });
    });

    group.finish();
}

/// Benchmark bulk read with analytics schema (more realistic workload).
fn bench_bulk_read_analytics(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulk_read_analytics");
    group.sample_size(10);

    let size = 1_000_000;
    let rows = create_analytics_rows(size);
    let schema = analytics_schema();

    group.throughput(Throughput::Elements(size as u64));
    group.bench_function("analytics_1m_rows", |b| {
        b.iter(|| {
            let config = BatchConfig::with_batch_size(65536);
            let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
            black_box(process_rows(&mut processor, &rows))
        });
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Batch Size Comparison Benchmarks
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_batch_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_sizes");

    let size = 100_000;
    let rows = create_int32_rows(size);
    let schema = int32_schema();

    group.throughput(Throughput::Elements(size as u64));

    for batch_size in [1024, 8192, 65536, 131072] {
        group.bench_with_input(BenchmarkId::from_parameter(batch_size), &rows, |b, rows| {
            b.iter(|| {
                let config = BatchConfig::with_batch_size(batch_size);
                let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
                black_box(process_rows(&mut processor, rows))
            });
        });
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Builder Creation Benchmarks
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_builder_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("builder_creation");

    let schema = mixed_schema();
    let factory = BuilderFactory::new(8192);

    group.bench_function("factory_create_builders", |b| {
        b.iter(|| black_box(factory.create_builders_for_schema(&schema)));
    });

    group.bench_function("factory_single_builder_int32", |b| {
        b.iter(|| black_box(factory.create_builder(&DataType::Int32)));
    });

    group.bench_function("factory_single_builder_utf8", |b| {
        b.iter(|| black_box(factory.create_builder(&DataType::Utf8)));
    });

    group.bench_function("factory_single_builder_decimal128", |b| {
        b.iter(|| black_box(factory.create_builder(&DataType::Decimal128(18, 2))));
    });

    group.bench_function("factory_single_builder_timestamp", |b| {
        b.iter(|| {
            black_box(factory.create_builder(&DataType::Timestamp(TimeUnit::Nanosecond, None)))
        });
    });

    group.bench_function("factory_single_builder_date32", |b| {
        b.iter(|| black_box(factory.create_builder(&DataType::Date32)));
    });

    group.bench_function("factory_single_builder_time64", |b| {
        b.iter(|| black_box(factory.create_builder(&DataType::Time64(TimeUnit::Nanosecond))));
    });

    group.bench_function("factory_single_builder_large_binary", |b| {
        b.iter(|| black_box(factory.create_builder(&DataType::LargeBinary)));
    });

    group.bench_function("factory_single_builder_large_utf8", |b| {
        b.iter(|| black_box(factory.create_builder(&DataType::LargeUtf8)));
    });

    group.bench_function("processor_creation", |b| {
        b.iter(|| {
            let config = BatchConfig::with_batch_size(8192);
            black_box(HanaBatchProcessor::new(Arc::clone(&schema), config))
        });
    });

    group.bench_function("processor_creation_analytics", |b| {
        let analytics = analytics_schema();
        b.iter(|| {
            let config = BatchConfig::with_batch_size(8192);
            black_box(HanaBatchProcessor::new(Arc::clone(&analytics), config))
        });
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Null Handling Benchmarks
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_null_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("null_handling");

    let size = 10_000;
    let column_count = 5;
    let rows = create_null_rows(size, column_count);
    let schema = nullable_schema(column_count);

    group.throughput(Throughput::Elements(size as u64));
    group.bench_function("all_nulls_5_columns", |b| {
        b.iter(|| {
            let config = BatchConfig::with_batch_size(8192);
            let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
            black_box(process_rows(&mut processor, &rows))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_int32_conversion,
    bench_int64_conversion,
    bench_float64_conversion,
    bench_boolean_conversion,
    bench_string_conversion,
    bench_decimal_conversion,
    bench_decimal_precision_scale,
    bench_decimal_large_values,
    bench_temporal_null_handling,
    bench_binary_conversion,
    bench_large_binary_lob,
    bench_large_utf8_clob,
    bench_mixed_schema,
    bench_wide_table,
    bench_analytics_workload,
    bench_bulk_read_1m,
    bench_bulk_read_analytics,
    bench_batch_sizes,
    bench_builder_creation,
    bench_null_handling,
);

criterion_main!(benches);
