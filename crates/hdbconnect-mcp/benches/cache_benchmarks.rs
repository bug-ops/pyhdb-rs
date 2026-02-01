//! Performance benchmarks for the cache abstraction layer
//!
//! Measures trait overhead, throughput, and concurrent access patterns.

use std::hint::black_box;
use std::sync::Arc;
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use hdbconnect_mcp::cache::{
    CacheBackend, CacheConfig, CacheKey, CacheProvider, InMemoryCache, NoopCache, TracedCache,
    create_cache,
};
use tokio::runtime::Runtime;

fn create_test_key(index: usize) -> CacheKey {
    CacheKey::table_schema(Some("bench_schema"), &format!("table_{index}"))
}

fn create_test_value(size: usize) -> Vec<u8> {
    vec![0u8; size]
}

fn bench_trait_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("trait_overhead");

    let direct_cache = InMemoryCache::new();
    let key = create_test_key(0);
    let value = create_test_value(100);

    rt.block_on(async {
        direct_cache.set(&key, &value, None).await.unwrap();
    });

    // Direct InMemoryCache call
    group.bench_function("direct_inmemory_get", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = black_box(direct_cache.get(&key).await);
        });
    });

    // Arc<dyn CacheProvider> dispatch
    let dyn_cache: Arc<dyn CacheProvider> = Arc::new(InMemoryCache::new());
    rt.block_on(async {
        dyn_cache.set(&key, &value, None).await.unwrap();
    });

    group.bench_function("arc_dyn_provider_get", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = black_box(dyn_cache.get(&key).await);
        });
    });

    // NoopCache baseline
    let noop_cache = NoopCache::new();
    group.bench_function("noop_get", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = black_box(noop_cache.get(&key).await);
        });
    });

    // TracedCache wrapper overhead
    let traced_cache = TracedCache::new(InMemoryCache::new(), "bench");
    rt.block_on(async {
        traced_cache.set(&key, &value, None).await.unwrap();
    });

    group.bench_function("traced_cache_get", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = black_box(traced_cache.get(&key).await);
        });
    });

    group.finish();
}

fn bench_inmemory_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("inmemory_throughput");

    for size in [64, 256, 1024, 4096] {
        group.throughput(Throughput::Bytes(size as u64));

        let cache = InMemoryCache::new();
        let key = create_test_key(0);
        let value = create_test_value(size);

        group.bench_with_input(BenchmarkId::new("set", size), &size, |b, _| {
            b.to_async(&rt).iter(|| async {
                cache.set(&key, &value, None).await.unwrap();
            });
        });

        rt.block_on(async {
            cache.set(&key, &value, None).await.unwrap();
        });

        group.bench_with_input(BenchmarkId::new("get", size), &size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let _ = black_box(cache.get(&key).await);
            });
        });
    }

    // Get vs miss
    let cache = InMemoryCache::new();
    let hit_key = create_test_key(0);
    let miss_key = create_test_key(9999);
    let value = create_test_value(100);

    rt.block_on(async {
        cache.set(&hit_key, &value, None).await.unwrap();
    });

    group.bench_function("get_hit", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = black_box(cache.get(&hit_key).await);
        });
    });

    group.bench_function("get_miss", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = black_box(cache.get(&miss_key).await);
        });
    });

    // Delete - use a pre-populated cache and delete different keys each iteration
    let cache_for_delete = InMemoryCache::new();
    rt.block_on(async {
        for i in 0..10000 {
            let key = create_test_key(i);
            cache_for_delete.set(&key, &value, None).await.unwrap();
        }
    });
    let delete_counter = std::sync::atomic::AtomicUsize::new(0);

    group.bench_function("delete", |b| {
        b.to_async(&rt).iter(|| async {
            let i = delete_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % 10000;
            let key = create_test_key(i);
            let _ = cache_for_delete.delete(&key).await;
        });
    });

    // Exists check
    let cache = InMemoryCache::new();
    let key = create_test_key(0);
    let value = create_test_value(100);
    rt.block_on(async {
        cache.set(&key, &value, None).await.unwrap();
    });

    group.bench_function("exists_hit", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = black_box(cache.exists(&key).await);
        });
    });

    group.bench_function("exists_miss", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = black_box(cache.exists(&miss_key).await);
        });
    });

    group.finish();
}

fn bench_ttl_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("ttl_overhead");

    let cache_no_ttl = InMemoryCache::new();
    let cache_with_ttl = InMemoryCache::new().with_default_ttl(Duration::from_secs(300));

    let key = create_test_key(0);
    let value = create_test_value(100);

    group.bench_function("set_no_ttl", |b| {
        b.to_async(&rt).iter(|| async {
            cache_no_ttl.set(&key, &value, None).await.unwrap();
        });
    });

    group.bench_function("set_with_default_ttl", |b| {
        b.to_async(&rt).iter(|| async {
            cache_with_ttl.set(&key, &value, None).await.unwrap();
        });
    });

    group.bench_function("set_explicit_ttl", |b| {
        let ttl = Some(Duration::from_secs(60));
        b.to_async(&rt).iter(|| async {
            cache_no_ttl.set(&key, &value, ttl).await.unwrap();
        });
    });

    // Get with TTL check overhead
    rt.block_on(async {
        cache_no_ttl.set(&key, &value, None).await.unwrap();
        cache_with_ttl.set(&key, &value, None).await.unwrap();
    });

    group.bench_function("get_no_ttl_check", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = black_box(cache_no_ttl.get(&key).await);
        });
    });

    group.bench_function("get_with_ttl_check", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = black_box(cache_with_ttl.get(&key).await);
        });
    });

    group.finish();
}

#[allow(clippy::too_many_lines)]
fn bench_concurrent_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_access");
    group.sample_size(50);

    for num_tasks in [1, 2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::new("concurrent_get", num_tasks),
            &num_tasks,
            |b, &num_tasks| {
                let rt = Runtime::new().unwrap();
                let cache = Arc::new(InMemoryCache::new());
                let value = create_test_value(100);

                rt.block_on(async {
                    for i in 0..100 {
                        let key = create_test_key(i);
                        cache.set(&key, &value, None).await.unwrap();
                    }
                });

                b.to_async(&rt).iter(|| {
                    let cache = cache.clone();
                    async move {
                        let handles: Vec<_> = (0..num_tasks)
                            .map(|task_id| {
                                let cache = cache.clone();
                                tokio::spawn(async move {
                                    for i in 0..100 {
                                        let key = create_test_key((task_id * 100 + i) % 100);
                                        let _ = cache.get(&key).await;
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

        group.bench_with_input(
            BenchmarkId::new("concurrent_set", num_tasks),
            &num_tasks,
            |b, &num_tasks| {
                let rt = Runtime::new().unwrap();
                let cache = Arc::new(InMemoryCache::new());
                let value = create_test_value(100);

                b.to_async(&rt).iter(|| {
                    let cache = cache.clone();
                    let value = value.clone();
                    async move {
                        let handles: Vec<_> = (0..num_tasks)
                            .map(|task_id| {
                                let cache = cache.clone();
                                let value = value.clone();
                                tokio::spawn(async move {
                                    for i in 0..100 {
                                        let key = create_test_key(task_id * 100 + i);
                                        cache.set(&key, &value, None).await.unwrap();
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

        group.bench_with_input(
            BenchmarkId::new("concurrent_mixed", num_tasks),
            &num_tasks,
            |b, &num_tasks| {
                let rt = Runtime::new().unwrap();
                let cache = Arc::new(InMemoryCache::new());
                let value = create_test_value(100);

                rt.block_on(async {
                    for i in 0..50 {
                        let key = create_test_key(i);
                        cache.set(&key, &value, None).await.unwrap();
                    }
                });

                b.to_async(&rt).iter(|| {
                    let cache = cache.clone();
                    let value = value.clone();
                    async move {
                        let handles: Vec<_> = (0..num_tasks)
                            .map(|task_id| {
                                let cache = cache.clone();
                                let value = value.clone();
                                tokio::spawn(async move {
                                    for i in 0..100 {
                                        let key = create_test_key((task_id * 100 + i) % 100);
                                        if i % 3 == 0 {
                                            cache.set(&key, &value, None).await.unwrap();
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

fn bench_memory_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_patterns");

    let cache_100b = InMemoryCache::new();
    let key_100b = create_test_key(0);
    let value_100b = create_test_value(100);

    group.bench_function("entry_set_100b", |b| {
        b.to_async(&rt).iter(|| async {
            cache_100b.set(&key_100b, &value_100b, None).await.unwrap();
        });
    });

    let cache_1kb = InMemoryCache::new();
    let key_1kb = create_test_key(1);
    let value_1kb = create_test_value(1024);

    group.bench_function("entry_set_1kb", |b| {
        b.to_async(&rt).iter(|| async {
            cache_1kb.set(&key_1kb, &value_1kb, None).await.unwrap();
        });
    });

    let cache_with_limit = InMemoryCache::new().with_max_entries(100);
    let value = create_test_value(100);

    rt.block_on(async {
        for i in 0..100 {
            let key = create_test_key(i);
            cache_with_limit.set(&key, &value, None).await.unwrap();
        }
    });

    let eviction_counter = std::sync::atomic::AtomicUsize::new(100);

    group.bench_function("set_with_eviction", |b| {
        b.to_async(&rt).iter(|| {
            let cache = cache_with_limit.clone();
            let i = eviction_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let key = create_test_key(i);
            let v = value.clone();
            async move {
                cache.set(&key, &v, None).await.unwrap();
            }
        });
    });

    group.finish();
}

fn bench_key_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_operations");

    group.bench_function("key_creation_table_schema", |b| {
        b.iter(|| black_box(CacheKey::table_schema(Some("schema"), "table")));
    });

    group.bench_function("key_creation_query_result", |b| {
        b.iter(|| {
            black_box(CacheKey::query_result(
                "SELECT * FROM users WHERE id = 1",
                Some(100),
            ))
        });
    });

    let key = CacheKey::table_schema(Some("schema"), "table");
    group.bench_function("key_to_string", |b| {
        b.iter(|| black_box(key.to_key_string()));
    });

    group.bench_function("key_namespace_prefix", |b| {
        b.iter(|| black_box(key.namespace_prefix()));
    });

    group.finish();
}

fn bench_create_cache_factory(c: &mut Criterion) {
    let mut group = c.benchmark_group("factory");

    group.bench_function("create_noop", |b| {
        let config = CacheConfig {
            enabled: true,
            backend: CacheBackend::Noop,
            ..Default::default()
        };
        b.iter(|| black_box(create_cache(&config)));
    });

    group.bench_function("create_memory", |b| {
        let config = CacheConfig {
            enabled: true,
            backend: CacheBackend::Memory,
            max_entries: Some(1000),
            ..Default::default()
        };
        b.iter(|| black_box(create_cache(&config)));
    });

    group.bench_function("create_disabled", |b| {
        let config = CacheConfig {
            enabled: false,
            ..Default::default()
        };
        b.iter(|| black_box(create_cache(&config)));
    });

    group.finish();
}

fn bench_delete_by_prefix(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("delete_by_prefix");
    group.sample_size(30);

    for num_entries in [10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::new("entries", num_entries),
            &num_entries,
            |b, &num_entries| {
                let value = create_test_value(100);
                b.to_async(&rt).iter(|| {
                    let value = value.clone();
                    async move {
                        let cache = InMemoryCache::new();
                        for i in 0..num_entries {
                            let key = create_test_key(i);
                            cache.set(&key, &value, None).await.unwrap();
                        }
                        cache.delete_by_prefix("tbl_schema").await.unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_stats(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("stats");

    for num_entries in [10, 100, 1000] {
        let cache = InMemoryCache::new();
        let value = create_test_value(100);

        rt.block_on(async {
            for i in 0..num_entries {
                let key = create_test_key(i);
                cache.set(&key, &value, None).await.unwrap();
            }
        });

        group.bench_with_input(
            BenchmarkId::new("compute_stats", num_entries),
            &num_entries,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let _ = black_box(cache.stats().await);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_trait_overhead,
    bench_inmemory_throughput,
    bench_ttl_overhead,
    bench_concurrent_access,
    bench_memory_overhead,
    bench_key_operations,
    bench_create_cache_factory,
    bench_delete_by_prefix,
    bench_stats,
);
criterion_main!(benches);
