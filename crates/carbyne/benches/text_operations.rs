use carbyne::Doc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_text_character_by_character(c: &mut Criterion) {
    c.bench_function("text_insert_5_chars_individually", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let text = doc.text();
            doc.set("text", text.clone());

            // Old way: insert each character separately
            text.append(doc.string("H"));
            text.append(doc.string("e"));
            text.append(doc.string("l"));
            text.append(doc.string("l"));
            text.append(doc.string("o"));
            
            doc.commit();
            black_box(text.size())
        })
    });
}

fn bench_text_batch_append(c: &mut Criterion) {
    c.bench_function("text_insert_5_chars_batch", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let text = doc.text();
            doc.set("text", text.clone());

            // New way: insert all at once
            doc.append_to_text(&text, "Hello");
            
            doc.commit();
            black_box(text.size())
        })
    });
}

fn bench_list_append_individually(c: &mut Criterion) {
    c.bench_function("list_append_10_items_individually", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let list = doc.list();
            doc.set("list", list.clone());

            // Old way: append each item separately
            for i in 0..10 {
                list.append(doc.atom(format!("item_{}", i)));
            }
            
            doc.commit();
            black_box(list.size())
        })
    });
}

fn bench_list_append_batch(c: &mut Criterion) {
    c.bench_function("list_append_10_items_batch", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let list = doc.list();
            doc.set("list", list.clone());

            // New way: batch append
            list.append_all(vec![
                doc.atom("item_0").into(),
                doc.atom("item_1").into(),
                doc.atom("item_2").into(),
                doc.atom("item_3").into(),
                doc.atom("item_4").into(),
                doc.atom("item_5").into(),
                doc.atom("item_6").into(),
                doc.atom("item_7").into(),
                doc.atom("item_8").into(),
                doc.atom("item_9").into(),
            ]);
            
            doc.commit();
            black_box(list.size())
        })
    });
}

fn bench_has_changes_since_vs_diff(c: &mut Criterion) {
    c.bench_function("has_changes_since_no_changes", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let state = doc.version();
            black_box(doc.has_changes_since(black_box(state)))
        })
    });

    c.bench_function("diff_no_changes", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let state = doc.version();
            black_box(doc.diff(black_box(state)))
        })
    });

    c.bench_function("has_changes_since_with_one_change", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let state = doc.version();
            
            doc.set("key", doc.atom("value"));
            doc.commit();
            
            black_box(doc.has_changes_since(black_box(state)))
        })
    });

    c.bench_function("diff_with_one_change", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let state = doc.version();
            
            doc.set("key", doc.atom("value"));
            doc.commit();
            
            black_box(doc.diff(black_box(state)))
        })
    });
}

criterion_group!(
    benches,
    bench_text_character_by_character,
    bench_text_batch_append,
    bench_list_append_individually,
    bench_list_append_batch,
    bench_has_changes_since_vs_diff
);
criterion_main!(benches);
