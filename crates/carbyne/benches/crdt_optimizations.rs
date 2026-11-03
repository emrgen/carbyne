use carbyne::Doc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// Benchmark 1: update_count() — tree with many appends
fn bench_change_sorter_append_heavy(c: &mut Criterion) {
    let mut group = c.benchmark_group("sorter_appends");
    group.sample_size(10);

    for count in [100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let doc = Doc::new("user-1");
                let list = doc.list();
                for i in 0..count {
                    list.append(doc.string(&i.to_string()));
                }
                list.len()
            });
        });
    }
    group.finish();
}

/// Benchmark 2: Diff computation with ClientMap cloning (now optimized)
fn bench_diff_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_ops");
    group.sample_size(10);

    group.bench_function("diff_simple_doc", |b| {
        b.iter(|| {
            let doc = Doc::new("user-1");
            let map = doc.map();

            // Create a doc
            for i in 0..50 {
                map.set(&format!("f_{}", i), doc.string(&i.to_string()));
            }

            doc.commit();
            doc.frontiers().len()
        });
    });
    group.finish();
}

/// Benchmark 3: Text size operations
fn bench_text_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_ops");
    group.sample_size(10);

    group.bench_function("text_append_50_chars", |b| {
        b.iter(|| {
            let doc = Doc::new("user-1");
            let text = doc.text();

            for i in 0..50 {
                text.append(doc.string(&i.to_string()));
            }

            text.size()
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_change_sorter_append_heavy,
    bench_diff_operations,
    bench_text_operations,
);
criterion_main!(benches);
