use carbyne::Doc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Build 1000-page doc, then simulate realistic editing patterns
fn bench_1000_page_with_edits(c: &mut Criterion) {
    c.bench_function("1000_page_build_plus_100_edits", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let text = doc.text();
            doc.set("content", text.clone());
            
            // Build 1000 pages
            for page_num in 0..1000 {
                let page_text = format!(
                    "Page {}\n\n{}\n\n",
                    page_num + 1,
                    "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                    Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
                    Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris. \
                    Duis aute irure dolor in reprehenderit in voluptate velit esse cillum. \
                    Dolore eu fugiat nulla pariatur excepteur sint occaecat cupidatat non proident."
                );
                doc.append_to_text(&text, &page_text);
            }
            doc.commit();
            
            // Now simulate 100 realistic edits (scattered throughout doc)
            let initial_state = doc.version();
            for edit_num in 0..100 {
                // Edit at different positions
                doc.set(
                    &format!("edit_{}", edit_num),
                    doc.atom(format!("change_{}", edit_num)),
                );
            }
            doc.commit();
            
            // Check: has changes since initial?
            let has_changes = doc.has_changes_since(black_box(initial_state));
            black_box((text.size(), has_changes))
        })
    });
}

/// Incremental building: add 100 pages at a time
fn bench_incremental_100_page_chunks(c: &mut Criterion) {
    c.bench_function("incremental_10x100_pages", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let text = doc.text();
            doc.set("content", text.clone());
            
            // Simulate loading document in 100-page chunks
            for chunk in 0..10 {
                for page_num in 0..100 {
                    let global_page = chunk * 100 + page_num;
                    let page_text = format!(
                        "Page {}\n\n{}\n\n",
                        global_page + 1,
                        "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                        Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
                        Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris. \
                        Duis aute irure dolor in reprehenderit in voluptate velit esse cillum. \
                        Dolore eu fugiat nulla pariatur excepteur sint occaecat cupidatat non proident."
                    );
                    doc.append_to_text(&text, &page_text);
                }
                doc.commit();
            }
            
            black_box(text.size())
        })
    });
}

criterion_group!(
    benches,
    bench_1000_page_with_edits,
    bench_incremental_100_page_chunks
);
criterion_main!(benches);
