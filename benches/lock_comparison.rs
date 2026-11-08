use std::hint::black_box;
use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use dkv::Cache;
use tokio::runtime::Runtime;
use tokio::sync::{Mutex, RwLock};

const CLIENTS: usize = 8;
const OPERATIONS_PER_CLIENT: usize = 1_000;

async fn run_mutex(read_percent: usize) {
    let cache = Arc::new(Mutex::new(Cache::new()));

    cache.lock().await.set("shared", "initial");

    let mut tasks = Vec::new();

    for _ in 0..CLIENTS {
        let cache = Arc::clone(&cache);

        tasks.push(tokio::spawn(async move {
            for operation in 0..OPERATIONS_PER_CLIENT {
                if operation % 100 < read_percent {
                    let cache = cache.lock().await;
                    black_box(cache.get("shared"));
                } else {
                    cache.lock().await.set("shared", "updated");
                }
            }
        }));
    }

    for task in tasks {
        task.await.unwrap();
    }
}

async fn run_rwlock(read_percent: usize) {
    let cache = Arc::new(RwLock::new(Cache::new()));

    cache.write().await.set("shared", "initial");

    let mut tasks = Vec::new();

    for _ in 0..CLIENTS {
        let cache = Arc::clone(&cache);

        tasks.push(tokio::spawn(async move {
            for operation in 0..OPERATIONS_PER_CLIENT {
                if operation % 100 < read_percent {
                    let cache = cache.read().await;
                    black_box(cache.get("shared"));
                } else {
                    cache.write().await.set("shared", "updated");
                }
            }
        }));
    }

    for task in tasks {
        task.await.unwrap();
    }
}

fn benchmark_locks(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();

    let mut group = c.benchmark_group("lock_comparison");

    for (name, read_percent) in [("read_heavy", 90), ("balanced", 50), ("write_heavy", 10)] {
        group.bench_with_input(
            BenchmarkId::new("Mutex", name),
            &read_percent,
            |b, &percent| {
                b.iter(|| runtime.block_on(run_mutex(percent)));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("RwLock", name),
            &read_percent,
            |b, &percent| {
                b.iter(|| runtime.block_on(run_rwlock(percent)));
            },
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_locks);
criterion_main!(benches);
