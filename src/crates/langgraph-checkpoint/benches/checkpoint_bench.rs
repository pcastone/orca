use criterion::{black_box, criterion_group, criterion_main, Criterion};
use langgraph_checkpoint::{Checkpoint, CheckpointConfig, CheckpointMetadata, CheckpointSaver, InMemoryCheckpointSaver};
use std::collections::HashMap;

fn checkpoint_save_benchmark(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("checkpoint save", |b| {
        b.to_async(&runtime).iter(|| async {
            let saver = InMemoryCheckpointSaver::new();
            let checkpoint = Checkpoint::empty();
            let metadata = CheckpointMetadata::new();
            let config = CheckpointConfig::new().with_thread_id("bench-thread".to_string());

            saver
                .put(&config, black_box(checkpoint), black_box(metadata), HashMap::new())
                .await
                .unwrap();
        });
    });
}

fn checkpoint_load_benchmark(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("checkpoint load", |b| {
        b.to_async(&runtime).iter(|| async {
            let saver = InMemoryCheckpointSaver::new();
            let checkpoint = Checkpoint::empty();
            let metadata = CheckpointMetadata::new();
            let config = CheckpointConfig::new().with_thread_id("bench-thread".to_string());

            let saved_config = saver
                .put(&config, checkpoint, metadata, HashMap::new())
                .await
                .unwrap();

            saver.get_tuple(black_box(&saved_config)).await.unwrap();
        });
    });
}

criterion_group!(benches, checkpoint_save_benchmark, checkpoint_load_benchmark);
criterion_main!(benches);
