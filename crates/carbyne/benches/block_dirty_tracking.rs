use carbyne::Doc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Find which blocks changed: use new dirty_blocks() API
fn bench_find_dirty_blocks_api(c: &mut Criterion) {
    c.bench_function("find_dirty_blocks_10k_tree", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let root = doc.list();
            
            for i in 0..10000 {
                let block = doc.map();
                block.set("id", doc.atom(format!("b{}", i)));
                block.set("content", doc.string(&format!("Block {}", i)));
                root.append(block);
            }
            
            doc.commit();
            let render_version = doc.version();
            
            // Peer edits
            for i in [150usize, 520, 545, 8000, 9500] {
                if let Some(block) = root.get(i) {
                    if let carbyne::Type::Map(m) = block {
                        m.set("content", doc.string("UPDATED"));
                    }
                }
            }
            doc.commit();
            
            // Get dirty blocks
            let dirty_blocks = doc.dirty_blocks(render_version);
            black_box(dirty_blocks.len())
        })
    });
}

/// Find dirty blocks in viewport
fn bench_find_dirty_in_viewport(c: &mut Criterion) {
    c.bench_function("find_dirty_in_viewport_100_blocks", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let root = doc.list();
            
            for i in 0..8000 {
                let block = doc.map();
                block.set("id", doc.atom(format!("b{}", i)));
                block.set("content", doc.string(&format!("Block {}", i)));
                root.append(block);
            }
            
            doc.commit();
            let render_version = doc.version();
            
            // Peer edits
            for i in [50usize, 75, 5000, 6000, 7500] {
                if let Some(block) = root.get(i) {
                    if let carbyne::Type::Map(m) = block {
                        m.set("content", doc.string("UPDATED"));
                    }
                }
            }
            doc.commit();
            
            // Get all dirty blocks
            let all_dirty = doc.dirty_blocks(render_version);
            
            // Filter to viewport
            let viewport_dirty: Vec<_> = all_dirty
                .into_iter()
                .filter(|idx| *idx < 100)
                .collect();
            
            black_box(viewport_dirty.len())
        })
    });
}

/// Two-phase dirty tracking
fn bench_two_phase_with_dirty_blocks(c: &mut Criterion) {
    c.bench_function("two_phase_has_changes_then_dirty", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let root = doc.list();
            
            for i in 0..6000 {
                let block = doc.map();
                block.set("id", doc.atom(format!("b{}", i)));
                block.set("content", doc.string(&format!("Block {}", i)));
                root.append(block);
            }
            
            doc.commit();
            let render_version = doc.version();
            
            // Phase 1: Quick check
            let has_changes = doc.has_changes_since(render_version.clone());
            
            let dirty_blocks = if !has_changes {
                vec![]
            } else {
                doc.dirty_blocks(render_version)
            };
            
            black_box(dirty_blocks.len())
        })
    });
}

/// Multi-viewport dirty checks
fn bench_multi_viewport_dirty_blocks(c: &mut Criterion) {
    c.bench_function("multi_viewport_5_dirty_checks", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let root = doc.list();
            
            for i in 0..5000 {
                let block = doc.map();
                block.set("id", doc.atom(format!("b{}", i)));
                block.set("content", doc.string(&format!("Block {}", i)));
                root.append(block);
            }
            
            doc.commit();
            
            // 5 render times
            let render_times = [
                doc.version(),
                doc.version(),
                doc.version(),
                doc.version(),
                doc.version(),
            ];
            
            // Peer edits
            for i in [500usize, 1200, 2500, 3800, 4500] {
                if let Some(block) = root.get(i) {
                    if let carbyne::Type::Map(m) = block {
                        m.set("content", doc.string("UPDATED"));
                    }
                }
            }
            doc.commit();
            
            // Check each
            let mut total_dirty = 0;
            for render_time in render_times {
                let dirty = doc.dirty_blocks(render_time);
                total_dirty += dirty.len();
            }
            
            black_box(total_dirty)
        })
    });
}

/// Incremental dirty tracking
fn bench_incremental_dirty_tracking(c: &mut Criterion) {
    c.bench_function("incremental_dirty_tracking_3_rounds", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let root = doc.list();
            
            for i in 0..4000 {
                let block = doc.map();
                block.set("id", doc.atom(format!("b{}", i)));
                block.set("content", doc.string(&format!("Block {}", i)));
                root.append(block);
            }
            
            doc.commit();
            let mut last_check = doc.version();
            
            let mut total_dirty = 0;
            
            for _round in 0..3 {
                // Peer edits
                for i in [300usize, 800, 2000] {
                    if let Some(block) = root.get(i) {
                        if let carbyne::Type::Map(m) = block {
                            m.set("content", doc.string("UPDATE"));
                        }
                    }
                }
                doc.commit();
                
                // Check dirty
                let dirty = doc.dirty_blocks(last_check);
                total_dirty += dirty.len();
                
                last_check = doc.version();
            }
            
            black_box(total_dirty)
        })
    });
}

criterion_group!(
    benches,
    bench_find_dirty_blocks_api,
    bench_find_dirty_in_viewport,
    bench_two_phase_with_dirty_blocks,
    bench_multi_viewport_dirty_blocks,
    bench_incremental_dirty_tracking
);
criterion_main!(benches);
