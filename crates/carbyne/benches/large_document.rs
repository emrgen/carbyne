use carbyne::Doc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Instant;

fn bench_build_1000_page_document(c: &mut Criterion) {
    c.bench_function("build_1000_page_document", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let text = doc.text();
            doc.set("content", text.clone());
            
            // 1000 pages, each ~2.5KB = 2.5MB total
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
            black_box(text.size())
        })
    });
}

fn bench_build_100_page_document(c: &mut Criterion) {
    c.bench_function("build_100_page_document", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let text = doc.text();
            doc.set("content", text.clone());
            
            // 100 pages for faster iteration
            for page_num in 0..100 {
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
            black_box(text.size())
        })
    });
}

criterion_group!(
    benches,
    bench_build_100_page_document,
    bench_build_1000_page_document
);
criterion_main!(benches);
