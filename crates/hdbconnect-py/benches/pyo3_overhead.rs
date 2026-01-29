//! PyO3 integration overhead benchmarks.
//!
//! Run with: `cargo bench --bench pyo3_overhead --features test-utils -p hdbconnect-py`
//!
//! These benchmarks measure the overhead introduced by PyO3 and the Arrow
//! C Stream protocol when transferring data from Rust to Python.
//!
//! Benchmarks:
//! - RecordBatch creation overhead
//! - pyo3-arrow PyRecordBatchReader creation
//! - Schema conversion overhead
//! - Arrow C Stream protocol setup

use std::hint::black_box;
use std::sync::Arc;

use arrow_array::RecordBatch;
use arrow_array::builder::{Int64Builder, StringBuilder};
use arrow_schema::{ArrowError, DataType, Field, Schema, SchemaRef, TimeUnit};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use hdbconnect_arrow::traits::row::MockRowBuilder;
use hdbconnect_arrow::traits::streaming::BatchConfig;
use hdbconnect_arrow::{BuilderFactory, HanaBatchProcessor, MockRow};

// ═══════════════════════════════════════════════════════════════════════════════
// Test Data Helpers
// ═══════════════════════════════════════════════════════════════════════════════

fn analytics_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("customer", DataType::Utf8, false),
        Field::new("product", DataType::Utf8, false),
        Field::new("quantity", DataType::Int32, false),
        Field::new("discount", DataType::Float64, false),
        Field::new("is_priority", DataType::Boolean, false),
        Field::new("region", DataType::Utf8, false),
    ]))
}

fn create_analytics_rows(count: usize) -> Vec<MockRow> {
    (0..count)
        .map(|i| {
            MockRowBuilder::new()
                .bigint(i as i64)
                .string(format!("customer_{}", i % 1000))
                .string(format!("product_{}", i % 500))
                .int((i % 100) as i32)
                .double(i as f64 * 0.15)
                .boolean(i % 3 == 0)
                .string(format!("region_{}", i % 10))
                .build()
        })
        .collect()
}

fn simple_int64_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![Field::new(
        "value",
        DataType::Int64,
        false,
    )]))
}

fn create_int64_rows(count: usize) -> Vec<MockRow> {
    (0..count)
        .map(|i| MockRowBuilder::new().bigint(i as i64).build())
        .collect()
}

fn create_record_batch_int64(size: usize) -> RecordBatch {
    let mut builder = Int64Builder::with_capacity(size);
    for i in 0..size {
        builder.append_value(i as i64);
    }
    let array = builder.finish();
    let schema = Arc::new(Schema::new(vec![Field::new(
        "value",
        DataType::Int64,
        false,
    )]));
    RecordBatch::try_new(schema, vec![Arc::new(array)]).unwrap()
}

fn create_record_batch_mixed(size: usize) -> RecordBatch {
    let mut id_builder = Int64Builder::with_capacity(size);
    let mut name_builder = StringBuilder::with_capacity(size, size * 20);

    for i in 0..size {
        id_builder.append_value(i as i64);
        name_builder.append_value(format!("row_{i}"));
    }

    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
    ]));

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id_builder.finish()),
            Arc::new(name_builder.finish()),
        ],
    )
    .unwrap()
}

// ═══════════════════════════════════════════════════════════════════════════════
// RecordBatch Creation Benchmarks
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_recordbatch_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("recordbatch_creation");

    for size in [1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| black_box(create_record_batch_int64(size)));
        });
    }

    group.finish();
}

fn bench_recordbatch_creation_mixed(c: &mut Criterion) {
    let mut group = c.benchmark_group("recordbatch_creation_mixed");

    for size in [1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| black_box(create_record_batch_mixed(size)));
        });
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// pyo3-arrow Wrapper Creation Benchmarks
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_pyo3_reader_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("pyo3_reader_creation");

    for size in [1_000, 10_000, 100_000] {
        let batches: Vec<RecordBatch> = vec![create_record_batch_int64(size)];
        let schema = simple_int64_schema();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(batches.clone(), schema.clone()),
            |b, (batches, _schema)| {
                b.iter(|| {
                    let batch_iter = batches.clone().into_iter().map(Ok);
                    black_box(pyo3_arrow::PyRecordBatchReader::new(Box::new(
                        arrow_array::RecordBatchIterator::new(batch_iter, simple_int64_schema()),
                    )))
                });
            },
        );
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Schema Conversion Benchmarks
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_schema_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("schema_conversion");

    let simple_schema = simple_int64_schema();
    let analytics = analytics_schema();

    group.bench_function("simple_1_column", |b| {
        b.iter(|| black_box(pyo3_arrow::PySchema::new(simple_schema.clone())));
    });

    group.bench_function("analytics_7_columns", |b| {
        b.iter(|| black_box(pyo3_arrow::PySchema::new(analytics.clone())));
    });

    // Wide schema (50 columns)
    let wide_schema: SchemaRef = Arc::new(Schema::new(
        (0..50)
            .map(|i| Field::new(format!("col_{i}"), DataType::Int32, false))
            .collect::<Vec<_>>(),
    ));

    group.bench_function("wide_50_columns", |b| {
        b.iter(|| black_box(pyo3_arrow::PySchema::new(wide_schema.clone())));
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Full Pipeline Benchmarks (HANA mock -> Arrow -> pyo3-arrow)
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_full_pipeline_int64(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline_int64");

    for size in [1_000, 10_000, 100_000] {
        let rows = create_int64_rows(size);
        let schema = simple_int64_schema();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &rows, |b, rows| {
            b.iter(|| {
                let config = BatchConfig::with_batch_size(8192);
                let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);

                let mut batches: Vec<Result<RecordBatch, ArrowError>> = Vec::new();
                for row in rows {
                    if let Ok(Some(batch)) = processor.process_row_generic(row) {
                        batches.push(Ok(batch));
                    }
                }
                if let Ok(Some(batch)) = processor.flush() {
                    batches.push(Ok(batch));
                }

                let reader = pyo3_arrow::PyRecordBatchReader::new(Box::new(
                    arrow_array::RecordBatchIterator::new(batches.into_iter(), schema.clone()),
                ));
                black_box(reader)
            });
        });
    }

    group.finish();
}

fn bench_full_pipeline_analytics(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline_analytics");

    for size in [1_000, 10_000, 100_000] {
        let rows = create_analytics_rows(size);
        let schema = analytics_schema();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &rows, |b, rows| {
            b.iter(|| {
                let config = BatchConfig::with_batch_size(8192);
                let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);

                let mut batches: Vec<Result<RecordBatch, ArrowError>> = Vec::new();
                for row in rows {
                    if let Ok(Some(batch)) = processor.process_row_generic(row) {
                        batches.push(Ok(batch));
                    }
                }
                if let Ok(Some(batch)) = processor.flush() {
                    batches.push(Ok(batch));
                }

                let reader = pyo3_arrow::PyRecordBatchReader::new(Box::new(
                    arrow_array::RecordBatchIterator::new(batches.into_iter(), schema.clone()),
                ));
                black_box(reader)
            });
        });
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Builder Factory Benchmarks (comparing to direct arrow builders)
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_builder_factory_vs_direct(c: &mut Criterion) {
    let mut group = c.benchmark_group("builder_factory_vs_direct");

    let capacity = 8192;

    group.bench_function("direct_int64_builder", |b| {
        b.iter(|| black_box(Int64Builder::with_capacity(capacity)));
    });

    group.bench_function("factory_int64_builder", |b| {
        let factory = BuilderFactory::new(capacity);
        b.iter(|| black_box(factory.create_builder(&DataType::Int64)));
    });

    group.bench_function("direct_string_builder", |b| {
        b.iter(|| black_box(StringBuilder::with_capacity(capacity, capacity * 50)));
    });

    group.bench_function("factory_string_builder", |b| {
        let factory = BuilderFactory::new(capacity);
        b.iter(|| black_box(factory.create_builder(&DataType::Utf8)));
    });

    group.bench_function("factory_timestamp_builder", |b| {
        let factory = BuilderFactory::new(capacity);
        b.iter(|| {
            black_box(factory.create_builder(&DataType::Timestamp(TimeUnit::Nanosecond, None)))
        });
    });

    group.bench_function("factory_decimal128_builder", |b| {
        let factory = BuilderFactory::new(capacity);
        b.iter(|| black_box(factory.create_builder(&DataType::Decimal128(38, 10))));
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Batch Size Impact on Pipeline
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_batch_size_impact(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_size_impact");

    let size = 100_000;
    let rows = create_int64_rows(size);
    let schema = simple_int64_schema();

    group.throughput(Throughput::Elements(size as u64));

    for batch_size in [1024, 4096, 8192, 16384, 32768, 65536] {
        group.bench_with_input(BenchmarkId::from_parameter(batch_size), &rows, |b, rows| {
            b.iter(|| {
                let config = BatchConfig::with_batch_size(batch_size);
                let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);

                let mut batch_count = 0usize;
                for row in rows {
                    if let Ok(Some(_batch)) = processor.process_row_generic(row) {
                        batch_count += 1;
                    }
                }
                if let Ok(Some(_batch)) = processor.flush() {
                    batch_count += 1;
                }

                black_box(batch_count)
            });
        });
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Memory Allocation Patterns
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_batch_reuse_vs_new(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_reuse_vs_new");

    let schema = simple_int64_schema();
    let iterations = 100;
    let batch_size = 1000;

    group.bench_function("create_new_processor_each_time", |b| {
        let rows = create_int64_rows(batch_size);
        b.iter(|| {
            for _ in 0..iterations {
                let config = BatchConfig::with_batch_size(batch_size);
                let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
                for row in &rows {
                    let _ = processor.process_row_generic(row);
                }
                black_box(processor.flush());
            }
        });
    });

    group.bench_function("reuse_processor_with_reset", |b| {
        let rows = create_int64_rows(batch_size);
        let config = BatchConfig::with_batch_size(batch_size);
        let mut processor = HanaBatchProcessor::new(Arc::clone(&schema), config);
        b.iter(|| {
            for _ in 0..iterations {
                for row in &rows {
                    let _ = processor.process_row_generic(row);
                }
                black_box(processor.flush());
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_recordbatch_creation,
    bench_recordbatch_creation_mixed,
    bench_pyo3_reader_creation,
    bench_schema_conversion,
    bench_full_pipeline_int64,
    bench_full_pipeline_analytics,
    bench_builder_factory_vs_direct,
    bench_batch_size_impact,
    bench_batch_reuse_vs_new,
);

criterion_main!(benches);
