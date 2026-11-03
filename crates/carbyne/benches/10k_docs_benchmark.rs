use carbyne::Doc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Benchmark: 10k independent docs, each with notion-like blocks
fn bench_10k_docs_with_blocks(c: &mut Criterion) {
    c.bench_function("10k_docs_with_notion_blocks", |b| {
        b.iter(|| {
            let mut total_size = 0;
            
            for doc_num in 0..10000 {
                let doc = Doc::new(black_box(Default::default()));
                let root = doc.list();
                doc.set("blocks", root.clone());
                
                // Each doc: 1 heading + 3 paragraphs = 4 blocks
                let heading = doc.map();
                heading.set("id", doc.atom(format!("doc_{}_block_0", doc_num)));
                heading.set("type", doc.atom("heading_1"));
                heading.set("content", doc.string(&format!("Document {}", doc_num)));
                root.append(heading);
                
                for block_idx in 0..3 {
                    let para = doc.map();
                    para.set("id", doc.atom(format!("doc_{}_block_{}", doc_num, block_idx + 1)));
                    para.set("type", doc.atom("paragraph"));
                    para.set(
                        "content",
                        doc.string(&format!("Paragraph {} of document {}", block_idx + 1, doc_num))
                    );
                    root.append(para);
                }
                
                doc.commit();
                total_size += root.size();
            }
            
            black_box(total_size)
        })
    });
}

/// Benchmark: 1 doc with 10k blocks
fn bench_single_doc_10k_blocks(c: &mut Criterion) {
    c.bench_function("single_doc_10k_blocks", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let root = doc.list();
            doc.set("blocks", root.clone());
            
            for block_num in 0..10000 {
                let block = doc.map();
                block.set("id", doc.atom(format!("block_{}", block_num)));
                block.set(
                    "type",
                    doc.atom(match block_num % 3 {
                        0 => "heading_2",
                        1 => "paragraph",
                        _ => "list",
                    })
                );
                block.set("content", doc.string(&format!("Block content {}", block_num)));
                root.append(block);
            }
            
            doc.commit();
            black_box(root.size())
        })
    });
}

criterion_group!(
    benches,
    bench_10k_docs_with_blocks,
    bench_single_doc_10k_blocks
);
criterion_main!(benches);
