use carbyne::Doc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// CORE INSIGHT: Virtual rendering with has_changes_since()
/// 
/// Huge block tree (10,000 blocks), viewport renders 100 blocks
/// When peer changes arrive, need to know: "Did viewport change?"
/// 
/// WITHOUT has_changes_since: Must full-diff to find out
/// WITH has_changes_since: O(1) yes/no answer

fn bench_huge_tree_10k_viewport_dirty_check(c: &mut Criterion) {
    c.bench_function("huge_tree_10k_viewport_dirty_check", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let root = doc.list();
            
            // Build 10,000 blocks (everything is a block in Notion)
            for i in 0..10000 {
                let block = doc.map();
                block.set("id", doc.atom(format!("b{}", i)));
                block.set("type", doc.atom("paragraph"));
                block.set("content", doc.string(&format!("Block {}", i)));
                root.append(block);
            }
            
            doc.commit();
            let viewport_render_time = doc.version();
            
            // Peer edits blocks FAR from viewport (blocks 8000-8010)
            // User sees viewport (0-100)
            for i in 8000usize..8010usize {
                if let Some(block) = root.get(i) {
                    if let carbyne::Type::Map(m) = block {
                        m.set("content", doc.string(&format!("UPDATED {}", i)));
                    }
                }
            }
            doc.commit();
            
            // QUESTION: Did viewport (0-100) need re-render?
            // ANSWER: has_changes_since() returns true (but changes are not in viewport)
            // Then we do selective diff to check viewport specifically
            let has_changes = doc.has_changes_since(viewport_render_time);
            
            black_box(has_changes)
        })
    });
}

/// FAST SKIP: If no changes at all, skip viewport update entirely
fn bench_skip_render_when_no_changes(c: &mut Criterion) {
    c.bench_function("skip_render_when_no_changes", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let root = doc.list();
            
            // Build 8000 blocks
            for i in 0..8000 {
                let block = doc.map();
                block.set("id", doc.atom(format!("b{}", i)));
                block.set("content", doc.string(&format!("Content {}", i)));
                root.append(block);
            }
            
            doc.commit();
            let viewport_render = doc.version();
            
            // No peer changes (network idle)
            // Don't commit anything
            
            // Check: Need to re-render viewport?
            // has_changes_since() returns false → skip entire viewport update
            let has_changes = doc.has_changes_since(viewport_render);
            
            // If false, skip render (saves DOM operations)
            let rerender_cost = if !has_changes {
                0 // Skip re-render
            } else {
                100 // Hypothetical: re-render viewport (not measured here)
            };
            
            black_box(rerender_cost)
        })
    });
}

/// MULTI-VIEWPORT: Each viewport has independent render time
/// E.g., split editor, multiple tabs, collapsible sections
fn bench_multi_viewport_independent_dirty_checks(c: &mut Criterion) {
    c.bench_function("multi_viewport_5_independent_checks", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let root = doc.list();
            
            // Build 6000 blocks
            for i in 0..6000 {
                let block = doc.map();
                block.set("id", doc.atom(format!("b{}", i)));
                block.set("content", doc.string(&format!("Content {}", i)));
                root.append(block);
            }
            
            doc.commit();
            
            // 5 independent viewports
            let vp_times = [
                doc.version(),
                doc.version(),
                doc.version(),
                doc.version(),
                doc.version(),
            ];
            
            // Peer edits scattered blocks
            let edits = [300usize, 1200, 2500, 4000, 5800];
            for i in edits {
                if let Some(block) = root.get(i) {
                    if let carbyne::Type::Map(m) = block {
                        m.set("content", doc.string("CHANGED"));
                    }
                }
            }
            doc.commit();
            
            // Check each viewport independently (O(1) each, not O(n))
            let mut dirty_viewports = 0;
            for vp_time in vp_times {
                if doc.has_changes_since(vp_time) {
                    dirty_viewports += 1;
                }
            }
            
            black_box(dirty_viewports)
        })
    });
}

/// SCROLL & SYNC: User scrolls while peers edit
/// Need to check if OLD viewport or NEW viewport changed
fn bench_scroll_and_sync_viewport(c: &mut Criterion) {
    c.bench_function("scroll_and_sync_reactive_viewport", |b| {
        b.iter(|| {
            let doc = Doc::new(black_box(Default::default()));
            let root = doc.list();
            
            // Build 5000 blocks
            for i in 0..5000 {
                let block = doc.map();
                block.set("id", doc.atom(format!("b{}", i)));
                block.set("content", doc.string(&format!("Content {}", i)));
                root.append(block);
            }
            
            doc.commit();
            let old_viewport_render = doc.version(); // Rendered blocks 0-100
            
            // User scrolls to blocks 200-300
            let new_viewport_render = doc.version();
            
            // While scrolling, peer edits (some in old, some in new)
            let edits = [50usize, 250, 400, 1500];
            for i in edits {
                if let Some(block) = root.get(i) {
                    if let carbyne::Type::Map(m) = block {
                        m.set("content", doc.string("PEER_EDIT"));
                    }
                }
            }
            doc.commit();
            
            // Check both viewports independently
            let old_changed = doc.has_changes_since(old_viewport_render);
            let new_changed = doc.has_changes_since(new_viewport_render);
            
            black_box((old_changed, new_changed))
        })
    });
}

criterion_group!(
    benches,
    bench_huge_tree_10k_viewport_dirty_check,
    bench_skip_render_when_no_changes,
    bench_multi_viewport_independent_dirty_checks,
    bench_scroll_and_sync_viewport
);
criterion_main!(benches);
