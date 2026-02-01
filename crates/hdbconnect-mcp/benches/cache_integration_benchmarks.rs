//! Cache integration benchmarks for Phase 3.4
//!
//! Measures cache performance in the context of server tools:
//! - cached_or_fetch helper latency (hit vs miss)
//! - Cache lookup overhead on hot path
//! - Concurrent access patterns with RwLock contention

use std::future::Future;
use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use hdbconnect_mcp::cache::{CacheKey, CacheProvider, InMemoryCache, NoopCache};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

// Simulated types matching server tool responses

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TableInfo {
    name: String,
    table_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TableSchema {
    table_name: String,
    columns: Vec<ColumnInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ColumnInfo {
    name: String,
    data_type: String,
    nullable: bool,
}

fn create_mock_table_list(count: usize) -> Vec<TableInfo> {
    (0..count)
        .map(|i| TableInfo {
            name: format!("TABLE_{i}"),
            table_type: "TABLE".to_string(),
        })
        .collect()
}

fn create_mock_table_schema(columns: usize) -> TableSchema {
    TableSchema {
        table_name: "USERS".to_string(),
        columns: (0..columns)
            .map(|i| ColumnInfo {
                name: format!("COLUMN_{i}"),
                data_type: "VARCHAR(255)".to_string(),
                nullable: i % 2 == 0,
            })
            .collect(),
    }
}

// Benchmark-local implementation of cached_or_fetch pattern.
// Cannot import from helpers module because benchmark uses `Result<T, String>`
// while server uses `crate::Result<T>`. This duplication is acceptable for
// benchmark isolation - ensures we measure the exact caching pattern behavior.
async fn cached_or_fetch<T, F, Fut>(
    cache: &dyn CacheProvider,
    key: &CacheKey,
    ttl: Duration,
    fetch: F,
) -> Result<T, String>
where
    T: Serialize + for<'de> Deserialize<'de>,
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    // 1. Try cache first
    if let Ok(Some(data)) = cache.get(key).await {
        if let Ok(value) = serde_json::from_slice::<T>(&data) {
            return Ok(value);
        }
    }

    // 2. Fetch from source
    let value = fetch().await?;

    // 3. Store in cache
    if let Ok(data) = serde_json::to_vec(&value) {
        let _ = cache.set(key, &data, Some(ttl)).await;
    }

    Ok(value)
}

/// Simulates database query latency
async fn simulate_db_fetch<T: Clone>(value: T, latency_us: u64) -> Result<T, String> {
    if latency_us > 0 {
        tokio::time::sleep(Duration::from_micros(latency_us)).await;
    }
    Ok(value)
}

fn bench_cached_or_fetch_hit_vs_miss(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("cached_or_fetch");
    group.sample_size(50);

    let table_list = create_mock_table_list(50);
    let table_schema = create_mock_table_schema(20);

    // Cache hit benchmarks
    for db_latency_us in [0, 1000, 10000] {
        let cache = Arc::new(InMemoryCache::new());
        let key = CacheKey::table_list(Some("BENCH"));
        let data = serde_json::to_vec(&table_list).unwrap();

        rt.block_on(async {
            cache.set(&key, &data, None).await.unwrap();
        });

        group.bench_with_input(
            BenchmarkId::new(format!("list_tables_hit_db{db_latency_us}us"), "50_tables"),
            &db_latency_us,
            |b, &latency| {
                let cache_clone = Arc::clone(&cache);
                let tables = table_list.clone();
                b.to_async(&rt).iter(|| {
                    let cache_ref = cache_clone.as_ref();
                    let tables = tables.clone();
                    let key = key.clone();
                    async move {
                        let result: Vec<TableInfo> =
                            cached_or_fetch(cache_ref, &key, Duration::from_secs(3600), || {
                                simulate_db_fetch(tables, latency)
                            })
                            .await
                            .unwrap();
                        black_box(result)
                    }
                });
            },
        );
    }

    // Cache miss benchmarks (simulating database fetch)
    for db_latency_us in [0, 1000, 10000] {
        let counter = Arc::new(AtomicU64::new(0));

        group.bench_with_input(
            BenchmarkId::new(format!("list_tables_miss_db{db_latency_us}us"), "50_tables"),
            &db_latency_us,
            |b, &latency| {
                let tables = table_list.clone();
                let counter_clone = Arc::clone(&counter);
                b.to_async(&rt).iter(|| {
                    let tables = tables.clone();
                    let cache = Arc::new(InMemoryCache::new());
                    let i = counter_clone.fetch_add(1, Ordering::Relaxed);
                    let key = CacheKey::table_list(Some(&format!("BENCH_{i}")));
                    async move {
                        let result: Vec<TableInfo> = cached_or_fetch(
                            cache.as_ref(),
                            &key,
                            Duration::from_secs(3600),
                            || simulate_db_fetch(tables, latency),
                        )
                        .await
                        .unwrap();
                        black_box(result)
                    }
                });
            },
        );
    }

    // Table schema benchmarks
    let cache = Arc::new(InMemoryCache::new());
    let key = CacheKey::table_schema(Some("BENCH"), "USERS");
    let data = serde_json::to_vec(&table_schema).unwrap();

    rt.block_on(async {
        cache.set(&key, &data, None).await.unwrap();
    });

    group.bench_function("describe_table_hit", |b| {
        let cache_clone = Arc::clone(&cache);
        let schema = table_schema.clone();
        let key = key.clone();
        b.to_async(&rt).iter(|| {
            let cache_ref = cache_clone.as_ref();
            let schema = schema.clone();
            let key = key.clone();
            async move {
                let result: TableSchema =
                    cached_or_fetch(cache_ref, &key, Duration::from_secs(3600), || {
                        simulate_db_fetch(schema, 0)
                    })
                    .await
                    .unwrap();
                black_box(result)
            }
        });
    });

    group.finish();
}

fn bench_cache_lookup_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("cache_lookup_overhead");

    // Measure pure cache lookup overhead (no fetch)
    let cache = Arc::new(InMemoryCache::new());
    let key = CacheKey::table_schema(Some("BENCH"), "USERS");
    let schema = create_mock_table_schema(20);
    let data = serde_json::to_vec(&schema).unwrap();

    rt.block_on(async {
        cache.set(&key, &data, None).await.unwrap();
    });

    // Direct cache get + deserialize (hot path)
    group.bench_function("cache_get_deserialize", |b| {
        let cache_clone = Arc::clone(&cache);
        let key = key.clone();
        b.to_async(&rt).iter(|| {
            let cache = Arc::clone(&cache_clone);
            let key = key.clone();
            async move {
                let data = cache.get(&key).await.unwrap().unwrap();
                let result: TableSchema = serde_json::from_slice(&data).unwrap();
                black_box(result)
            }
        });
    });

    // Noop cache overhead (should be near-zero)
    let noop_cache = Arc::new(NoopCache::new());
    group.bench_function("noop_cache_get", |b| {
        let noop = Arc::clone(&noop_cache);
        let key = key.clone();
        b.to_async(&rt).iter(|| {
            let noop = Arc::clone(&noop);
            let key = key.clone();
            async move {
                let result = noop.get(&key).await;
                black_box(result)
            }
        });
    });

    // Key creation overhead
    group.bench_function("key_creation_table_list", |b| {
        b.iter(|| black_box(CacheKey::table_list(Some("SCHEMA_NAME"))));
    });

    group.bench_function("key_creation_table_schema", |b| {
        b.iter(|| black_box(CacheKey::table_schema(Some("SCHEMA_NAME"), "TABLE_NAME")));
    });

    // Serialization overhead for typical payloads
    let tables = create_mock_table_list(100);
    group.bench_function("serialize_100_tables", |b| {
        b.iter(|| black_box(serde_json::to_vec(&tables).unwrap()));
    });

    let schema = create_mock_table_schema(50);
    group.bench_function("serialize_50_columns", |b| {
        b.iter(|| black_box(serde_json::to_vec(&schema).unwrap()));
    });

    // Deserialization overhead
    let tables_bytes = serde_json::to_vec(&tables).unwrap();
    group.bench_function("deserialize_100_tables", |b| {
        b.iter(|| {
            let result: Vec<TableInfo> = serde_json::from_slice(&tables_bytes).unwrap();
            black_box(result)
        });
    });

    let schema_bytes = serde_json::to_vec(&schema).unwrap();
    group.bench_function("deserialize_50_columns", |b| {
        b.iter(|| {
            let result: TableSchema = serde_json::from_slice(&schema_bytes).unwrap();
            black_box(result)
        });
    });

    group.finish();
}

fn bench_concurrent_cache_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_cache_access");
    group.sample_size(30);

    let schema = create_mock_table_schema(20);
    let schema_bytes = serde_json::to_vec(&schema).unwrap();

    // Test RwLock contention with varying reader/writer ratios
    for num_tasks in [2, 4, 8] {
        // Read-heavy workload (90% reads, 10% writes)
        group.bench_with_input(
            BenchmarkId::new("read_heavy", num_tasks),
            &num_tasks,
            |b, &num_tasks| {
                let rt = Runtime::new().unwrap();
                let cache = Arc::new(InMemoryCache::new());
                let schema_bytes_clone = schema_bytes.clone();

                // Pre-populate cache
                rt.block_on(async {
                    for i in 0..100 {
                        let key = CacheKey::table_schema(Some("BENCH"), &format!("TABLE_{i}"));
                        cache.set(&key, &schema_bytes_clone, None).await.unwrap();
                    }
                });

                b.to_async(&rt).iter(|| {
                    let cache = Arc::clone(&cache);
                    let schema_bytes = schema_bytes_clone.clone();
                    async move {
                        let handles: Vec<_> = (0..num_tasks)
                            .map(|task_id| {
                                let cache = Arc::clone(&cache);
                                let schema_bytes = schema_bytes.clone();
                                tokio::spawn(async move {
                                    for i in 0..100 {
                                        let idx = (task_id * 100 + i) % 100;
                                        let key = CacheKey::table_schema(
                                            Some("BENCH"),
                                            &format!("TABLE_{idx}"),
                                        );
                                        if i % 10 == 0 {
                                            // 10% writes
                                            cache.set(&key, &schema_bytes, None).await.unwrap();
                                        } else {
                                            // 90% reads
                                            let _ = cache.get(&key).await;
                                        }
                                    }
                                })
                            })
                            .collect();

                        for handle in handles {
                            handle.await.unwrap();
                        }
                    }
                });
            },
        );

        // Write-heavy workload (50% reads, 50% writes)
        group.bench_with_input(
            BenchmarkId::new("write_heavy", num_tasks),
            &num_tasks,
            |b, &num_tasks| {
                let rt = Runtime::new().unwrap();
                let cache = Arc::new(InMemoryCache::new());
                let schema_bytes_clone = schema_bytes.clone();

                b.to_async(&rt).iter(|| {
                    let cache = Arc::clone(&cache);
                    let schema_bytes = schema_bytes_clone.clone();
                    async move {
                        let handles: Vec<_> = (0..num_tasks)
                            .map(|task_id| {
                                let cache = Arc::clone(&cache);
                                let schema_bytes = schema_bytes.clone();
                                tokio::spawn(async move {
                                    for i in 0..100 {
                                        let idx = (task_id * 100 + i) % 100;
                                        let key = CacheKey::table_schema(
                                            Some("BENCH"),
                                            &format!("TABLE_{idx}"),
                                        );
                                        if i % 2 == 0 {
                                            cache.set(&key, &schema_bytes, None).await.unwrap();
                                        } else {
                                            let _ = cache.get(&key).await;
                                        }
                                    }
                                })
                            })
                            .collect();

                        for handle in handles {
                            handle.await.unwrap();
                        }
                    }
                });
            },
        );
    }

    // Same key contention (worst case)
    for num_tasks in [2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::new("same_key_contention", num_tasks),
            &num_tasks,
            |b, &num_tasks| {
                let rt = Runtime::new().unwrap();
                let cache = Arc::new(InMemoryCache::new());
                let schema_bytes_clone = schema_bytes.clone();
                let key = CacheKey::table_schema(Some("BENCH"), "SINGLE_TABLE");

                rt.block_on(async {
                    cache.set(&key, &schema_bytes_clone, None).await.unwrap();
                });

                b.to_async(&rt).iter(|| {
                    let cache = Arc::clone(&cache);
                    let schema_bytes = schema_bytes_clone.clone();
                    let key = key.clone();
                    async move {
                        let handles: Vec<_> = (0..num_tasks)
                            .map(|_| {
                                let cache = Arc::clone(&cache);
                                let schema_bytes = schema_bytes.clone();
                                let key = key.clone();
                                tokio::spawn(async move {
                                    for i in 0..100 {
                                        if i % 10 == 0 {
                                            cache.set(&key, &schema_bytes, None).await.unwrap();
                                        } else {
                                            let _ = cache.get(&key).await;
                                        }
                                    }
                                })
                            })
                            .collect();

                        for handle in handles {
                            handle.await.unwrap();
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_cache_disabled_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("cache_disabled");

    let table_list = create_mock_table_list(50);
    let table_schema = create_mock_table_schema(20);

    // Measure overhead when using NoopCache (cache disabled)
    let noop: Arc<dyn CacheProvider> = Arc::new(NoopCache::new());

    group.bench_function("list_tables_noop", |b| {
        let tables = table_list.clone();
        let noop_clone = Arc::clone(&noop);
        b.to_async(&rt).iter(|| {
            let tables = tables.clone();
            let noop = Arc::clone(&noop_clone);
            let key = CacheKey::table_list(Some("BENCH"));
            async move {
                let result: Vec<TableInfo> =
                    cached_or_fetch(noop.as_ref(), &key, Duration::from_secs(3600), || async {
                        Ok(tables)
                    })
                    .await
                    .unwrap();
                black_box(result)
            }
        });
    });

    group.bench_function("describe_table_noop", |b| {
        let schema = table_schema.clone();
        let noop_clone = Arc::clone(&noop);
        b.to_async(&rt).iter(|| {
            let schema = schema.clone();
            let noop = Arc::clone(&noop_clone);
            let key = CacheKey::table_schema(Some("BENCH"), "USERS");
            async move {
                let result: TableSchema =
                    cached_or_fetch(noop.as_ref(), &key, Duration::from_secs(3600), || async {
                        Ok(schema)
                    })
                    .await
                    .unwrap();
                black_box(result)
            }
        });
    });

    // Compare with enabled cache (always miss since we create new cache each time)
    let memory: Arc<dyn CacheProvider> = Arc::new(InMemoryCache::new());

    group.bench_function("list_tables_memory_miss", |b| {
        let tables = table_list.clone();
        let counter = Arc::new(AtomicU64::new(0));
        let memory_clone = Arc::clone(&memory);
        b.to_async(&rt).iter(|| {
            let tables = tables.clone();
            let memory = Arc::clone(&memory_clone);
            let i = counter.fetch_add(1, Ordering::Relaxed);
            let key = CacheKey::table_list(Some(&format!("BENCH_{i}")));
            async move {
                let result: Vec<TableInfo> =
                    cached_or_fetch(memory.as_ref(), &key, Duration::from_secs(3600), || async {
                        Ok(tables)
                    })
                    .await
                    .unwrap();
                black_box(result)
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cached_or_fetch_hit_vs_miss,
    bench_cache_lookup_overhead,
    bench_concurrent_cache_access,
    bench_cache_disabled_overhead,
);
criterion_main!(benches);
