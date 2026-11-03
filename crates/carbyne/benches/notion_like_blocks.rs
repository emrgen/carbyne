use carbyne::Doc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Realistic Notion-like document with nested blocks
/// Structure:
/// - Page root
///   - Heading blocks
///   - Paragraph blocks (with inline formatting)
///   - List blocks (multiple items)
///   - Database/table blocks
fn bench_notion_1000_blocks(c: &mut Criterion) {
    c.bench_function("notion_1000_blocks_with_metadata", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let root = doc.list();
            doc.set("blocks", root.clone());
            
            // 1000 blocks (mix of types)
            for block_num in 0..1000 {
                // Each block has metadata + content
                let block = doc.map();
                
                // Block metadata
                block.set("id", doc.atom(format!("block_{}", block_num)));
                block.set("type", doc.atom(get_block_type(block_num)));
                block.set("created_at", doc.atom("2026-11-03"));
                block.set("created_by", doc.atom("user_1"));
                
                // Block content varies by type
                match block_num % 4 {
                    0 => {
                        // Heading: just text
                        block.set(
                            "content",
                            doc.string(&format!("Heading {}", block_num / 4 + 1))
                        );
                    }
                    1 => {
                        // Paragraph: text + inline formatting
                        let paragraph_text = "This is a paragraph with some content. It contains multiple sentences. \
                                            Paragraphs can have bold, italic, and other formatting.";
                        block.set("content", doc.string(paragraph_text));
                        block.set("formatting", doc.atom("rich_text"));
                    }
                    2 => {
                        // List: multiple items
                        let items = doc.list();
                        for item_idx in 0..5 {
                            let item = doc.map();
                            item.set("text", doc.string(&format!("List item {}", item_idx)));
                            item.set("level", doc.atom("0"));
                            items.append(item);
                        }
                        block.set("items", items);
                    }
                    _ => {
                        // Table: rows and cells
                        let rows = doc.list();
                        for row_idx in 0..3 {
                            let row = doc.list();
                            for col_idx in 0..3 {
                                let cell = doc.map();
                                cell.set(
                                    "content",
                                    doc.string(&format!("R{}C{}", row_idx, col_idx))
                                );
                                row.append(cell);
                            }
                            rows.append(row);
                        }
                        block.set("rows", rows);
                    }
                }
                
                root.append(block);
            }
            
            doc.commit();
            black_box(root.size())
        })
    });
}

/// Realistic: 100 pages worth of blocks (mix of types)
fn bench_notion_100_pages(c: &mut Criterion) {
    c.bench_function("notion_100_pages_realistic", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let root = doc.list();
            doc.set("blocks", root.clone());
            
            let mut block_num = 0;
            
            // 100 "pages" (sections)
            for page_num in 0..100 {
                // Page heading
                let heading = doc.map();
                heading.set("id", doc.atom(format!("block_{}", block_num)));
                heading.set("type", doc.atom("heading_1"));
                heading.set("content", doc.string(&format!("Page {}", page_num + 1)));
                root.append(heading);
                block_num += 1;
                
                // 10 paragraphs per page
                for _ in 0..10 {
                    let para = doc.map();
                    para.set("id", doc.atom(format!("block_{}", block_num)));
                    para.set("type", doc.atom("paragraph"));
                    para.set(
                        "content",
                        doc.string("Lorem ipsum dolor sit amet, consectetur adipiscing elit.")
                    );
                    root.append(para);
                    block_num += 1;
                }
            }
            
            doc.commit();
            black_box(root.size())
        })
    });
}

fn get_block_type(idx: usize) -> String {
    match idx % 4 {
        0 => "heading_2".to_string(),
        1 => "paragraph".to_string(),
        2 => "bulleted_list".to_string(),
        _ => "table".to_string(),
    }
}

criterion_group!(
    benches,
    bench_notion_1000_blocks,
    bench_notion_100_pages
);
criterion_main!(benches);
