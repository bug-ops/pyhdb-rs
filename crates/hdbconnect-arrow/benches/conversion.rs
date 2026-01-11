//! Comprehensive benchmarks for HANA to Arrow conversion operations.
//!
//! Run with: `cargo bench --bench conversion --features test-utils`
//!
//! These benchmarks measure:
//! - Single-type conversion performance (Int32, Int64, Float64, Boolean, String)
//! - Mixed schema conversion performance
//! - Batch size impact on throughput
//! - Builder creation overhead
//! - Null handling performance

use std::hint::black_box;
use std::sync::Arc;

use arrow_schema::{DataType, Field, Schema};
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

    group.bench_function("processor_creation", |b| {
        b.iter(|| {
            let config = BatchConfig::with_batch_size(8192);
            black_box(HanaBatchProcessor::new(Arc::clone(&schema), config))
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
    bench_mixed_schema,
    bench_batch_sizes,
    bench_builder_creation,
    bench_null_handling,
);

criterion_main!(benches);
