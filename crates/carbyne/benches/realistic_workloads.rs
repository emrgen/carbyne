use carbyne::Doc;
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

/// Simulate realistic text editing: typing paragraphs
fn bench_text_typing_paragraph(c: &mut Criterion) {
    let paragraph = "The quick brown fox jumps over the lazy dog. This is a realistic paragraph that someone might type. It contains multiple sentences and words that demonstrate real-world usage patterns.";
    
    c.bench_function("text_typing_paragraph_individual", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let text = doc.text();
            doc.set("content", text.clone());

            // Old way: each word separately
            for word in paragraph.split_whitespace() {
                text.append(doc.string(word));
                text.append(doc.string(" "));
            }
            
            doc.commit();
            black_box(text.size())
        })
    });

    c.bench_function("text_typing_paragraph_batch", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let text = doc.text();
            doc.set("content", text.clone());

            // New way: entire paragraph at once
            doc.append_to_text(&text, paragraph);
            
            doc.commit();
            black_box(text.size())
        })
    });
}

/// Simulate building lists of different sizes
fn bench_list_bulk_insert_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("list_bulk_insert");
    group.sample_size(50); // Reduce samples for large sizes
    
    for size in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("individual", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let doc = Doc::new(black_box(Default::default()));
                    let list = doc.list();
                    doc.set("items", list.clone());

                    for i in 0..size {
                        list.append(doc.atom(format!("item_{:04}", i)));
                    }
                    
                    doc.commit();
                    black_box(list.size())
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("batch", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let doc = Doc::new(black_box(Default::default()));
                    let list = doc.list();
                    doc.set("items", list.clone());

                    let items: Vec<_> = (0..size)
                        .map(|i| doc.atom(format!("item_{:04}", i)).into())
                        .collect();
                    list.append_all(items);
                    
                    doc.commit();
                    black_box(list.size())
                })
            },
        );
    }
    group.finish();
}

/// Simulate document with many fields and changes
fn bench_has_changes_since_large_doc(c: &mut Criterion) {
    c.bench_function("has_changes_since_large_doc_no_changes", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            
            // Build document with many fields
            for i in 0..50 {
                doc.set(
                    &format!("field_{:02}", i),
                    doc.atom(format!("value_{}", i)),
                );
            }
            doc.commit();

            let state = doc.version();
            
            // Check for changes (should find none)
            black_box(doc.has_changes_since(black_box(state)))
        })
    });

    c.bench_function("diff_large_doc_no_changes", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            
            // Build document with many fields
            for i in 0..50 {
                doc.set(
                    &format!("field_{:02}", i),
                    doc.atom(format!("value_{}", i)),
                );
            }
            doc.commit();

            let state = doc.version();
            
            // Compute full diff (expensive)
            black_box(doc.diff(black_box(state)))
        })
    });

    c.bench_function("has_changes_since_large_doc_10_changes", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            
            // Build document with many fields
            for i in 0..50 {
                doc.set(
                    &format!("field_{:02}", i),
                    doc.atom(format!("value_{}", i)),
                );
            }
            doc.commit();

            let state = doc.version();
            
            // Make 10 changes (20% of fields)
            for i in 0..10 {
                doc.set(
                    &format!("field_{:02}", i),
                    doc.atom(format!("updated_{}", i)),
                );
            }
            doc.commit();
            
            // Check for changes
            black_box(doc.has_changes_since(black_box(state)))
        })
    });

    c.bench_function("diff_large_doc_10_changes", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            
            // Build document with many fields
            for i in 0..50 {
                doc.set(
                    &format!("field_{:02}", i),
                    doc.atom(format!("value_{}", i)),
                );
            }
            doc.commit();

            let state = doc.version();
            
            // Make 10 changes
            for i in 0..10 {
                doc.set(
                    &format!("field_{:02}", i),
                    doc.atom(format!("updated_{}", i)),
                );
            }
            doc.commit();
            
            // Compute full diff
            black_box(doc.diff(black_box(state)))
        })
    });
}

/// Simulate sync polling: checking many times if anything changed
fn bench_sync_polling_pattern(c: &mut Criterion) {
    c.bench_function("sync_polling_has_changes_since", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            
            // Build initial document
            for i in 0..20 {
                doc.set(
                    &format!("item_{:02}", i),
                    doc.atom(format!("data_{}", i)),
                );
            }
            doc.commit();

            let mut last_state = doc.version();
            let mut changed_count = 0;

            // Simulate polling pattern: check 50 times, make change every 10 checks
            for check_num in 0..50 {
                if check_num > 0 && check_num % 10 == 0 {
                    // Make a change
                    doc.set("last_change", doc.atom(format!("change_{}", check_num)));
                    doc.commit();
                    last_state = doc.version();
                } else {
                    // Check if changed
                    if doc.has_changes_since(black_box(last_state.clone())) {
                        changed_count += 1;
                    }
                }
            }
            
            black_box(changed_count)
        })
    });

    c.bench_function("sync_polling_diff", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            
            // Build initial document
            for i in 0..20 {
                doc.set(
                    &format!("item_{:02}", i),
                    doc.atom(format!("data_{}", i)),
                );
            }
            doc.commit();

            let mut last_state = doc.version();
            let mut empty_diffs = 0;

            // Simulate polling pattern with diff
            for check_num in 0..50 {
                if check_num > 0 && check_num % 10 == 0 {
                    // Make a change
                    doc.set("last_change", doc.atom(format!("change_{}", check_num)));
                    doc.commit();
                    last_state = doc.version();
                } else {
                    // Compute full diff (expensive even if empty)
                    let _diff = doc.diff(black_box(last_state.clone()));
                    empty_diffs += 1;
                }
            }
            
            black_box(empty_diffs)
        })
    });
}

criterion_group!(
    benches,
    bench_text_typing_paragraph,
    bench_list_bulk_insert_sizes,
    bench_has_changes_since_large_doc,
    bench_sync_polling_pattern
);
criterion_main!(benches);
