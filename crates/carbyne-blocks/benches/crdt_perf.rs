use criterion::{black_box, criterion_group, criterion_main, Criterion};
use carbyne_blocks::BlockStore;
use carbyne_blocks::TextOpId;
use carbyne::Doc;
use std::sync::Arc;
use tempfile::TempDir;

fn setup_store() -> (BlockStore, TempDir, Arc<Doc>) {
    let dir = TempDir::new().unwrap();
    let doc = Arc::new(Doc::new());
    let store = BlockStore::new(doc.clone(), dir.path()).unwrap();
    (store, dir, doc)
}

fn benchmark_mark_ops_applied(c: &mut Criterion) {
    c.bench_function("mark_ops_applied_100", |b| {
        b.iter_batched(
            setup_store,
            |(store, _dir, _doc)| {
                let ops: Vec<TextOpId> = (0..100)
                    .map(|i| TextOpId {
                        id: format!("seq-{}#client-A", i),
                    })
                    .collect();
                store.mark_ops_applied("inst-X", black_box(ops)).unwrap();
            },
            criterion::BatchSize::SmallInput,
        );
    });

    c.bench_function("mark_ops_applied_1000", |b| {
        b.iter_batched(
            setup_store,
            |(store, _dir, _doc)| {
                let ops: Vec<TextOpId> = (0..1000)
                    .map(|i| TextOpId {
                        id: format!("seq-{}#client-A", i),
                    })
                    .collect();
                store.mark_ops_applied("inst-X", black_box(ops)).unwrap();
            },
            criterion::BatchSize::SmallInput,
        );
    });

    c.bench_function("mark_ops_applied_10k", |b| {
        b.iter_batched(
            setup_store,
            |(store, _dir, _doc)| {
                let ops: Vec<TextOpId> = (0..10000)
                    .map(|i| TextOpId {
                        id: format!("seq-{}#client-A", i),
                    })
                    .collect();
                store.mark_ops_applied("inst-X", black_box(ops)).unwrap();
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn benchmark_is_op_applied(c: &mut Criterion) {
    c.bench_function("is_op_applied_after_100_inserts", |b| {
        let (store, _dir, _doc) = setup_store();
        let ops: Vec<TextOpId> = (0..100)
            .map(|i| TextOpId {
                id: format!("seq-{}#client-A", i),
            })
            .collect();
        store.mark_ops_applied("inst-Y", ops.clone()).unwrap();

        b.iter(|| {
            for op in &ops {
                let _ = store.is_op_applied("inst-Y", &op.id).unwrap();
            }
        });
    });

    c.bench_function("is_op_applied_after_1000_inserts", |b| {
        let (store, _dir, _doc) = setup_store();
        let ops: Vec<TextOpId> = (0..1000)
            .map(|i| TextOpId {
                id: format!("seq-{}#client-A", i),
            })
            .collect();
        store.mark_ops_applied("inst-Y", ops.clone()).unwrap();

        b.iter(|| {
            for op in &ops {
                let _ = store.is_op_applied("inst-Y", &op.id).unwrap();
            }
        });
    });
}

fn benchmark_record_placement(c: &mut Criterion) {
    c.bench_function("record_placement_100_blocks", |b| {
        b.iter_batched(
            setup_store,
            |(store, _dir, _doc)| {
                for block_id in 0..100 {
                    store
                        .record_placement(&format!("inst-{}", block_id), block_id)
                        .unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    c.bench_function("record_placement_1000_blocks", |b| {
        b.iter_batched(
            setup_store,
            |(store, _dir, _doc)| {
                for block_id in 0..1000 {
                    store
                        .record_placement(&format!("inst-{}", block_id), block_id)
                        .unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn benchmark_instances_in(c: &mut Criterion) {
    c.bench_function("instances_in_after_100_placements", |b| {
        let (store, _dir, _doc) = setup_store();
        for block_id in 0..100 {
            store
                .record_placement(&format!("inst-{}", block_id), block_id)
                .unwrap();
        }

        let block_ids: Vec<usize> = (0..100).collect();
        b.iter(|| {
            let _ = store.instances_in(black_box(&block_ids)).unwrap();
        });
    });

    c.bench_function("instances_in_after_1000_placements", |b| {
        let (store, _dir, _doc) = setup_store();
        for block_id in 0..1000 {
            store
                .record_placement(&format!("inst-{}", block_id), block_id)
                .unwrap();
        }

        let block_ids: Vec<usize> = (0..100).collect();
        b.iter(|| {
            let _ = store.instances_in(black_box(&block_ids)).unwrap();
        });
    });
}

fn benchmark_resend_dedup(c: &mut Criterion) {
    c.bench_function("resend_dedup_same_op_5x", |b| {
        b.iter_batched(
            setup_store,
            |(store, _dir, _doc)| {
                let ops = vec![TextOpId {
                    id: "seq-1#client-A".to_string(),
                }];

                // Insert same op 5 times (simulates resends)
                for _ in 0..5 {
                    let _ = store.mark_ops_applied("inst-Z", ops.clone());
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    c.bench_function("resend_dedup_verification_5x", |b| {
        let (store, _dir, _doc) = setup_store();
        let ops = vec![TextOpId {
            id: "seq-1#client-A".to_string(),
        }];

        store.mark_ops_applied("inst-Z", ops.clone()).unwrap();

        b.iter(|| {
            // Check 5 times if op was already applied
            for _ in 0..5 {
                let _ = store.is_op_applied("inst-Z", "seq-1#client-A");
            }
        });
    });
}

criterion_group!(
    benches,
    benchmark_mark_ops_applied,
    benchmark_is_op_applied,
    benchmark_record_placement,
    benchmark_instances_in,
    benchmark_resend_dedup,
);
criterion_main!(benches);
