// Minimal headless smoke test for the render graph scaffolding
#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::graph::{ClearNode, RendererMainNode};
use astraweave_render::graph::{GraphContext, RenderGraph, RenderNode};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct CountNode {
    name: &'static str,
    counter: Arc<AtomicUsize>,
}
impl RenderNode for CountNode {
    fn name(&self) -> &str {
        self.name
    }
    fn run(&mut self, _ctx: &mut GraphContext) -> anyhow::Result<()> {
        self.counter.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn graph_executes_all_nodes() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut g = RenderGraph::new();
    g.add_node(CountNode {
        name: "shadow",
        counter: count.clone(),
    });
    g.add_node(CountNode {
        name: "main",
        counter: count.clone(),
    });
    g.add_node(CountNode {
        name: "post",
        counter: count.clone(),
    });

    let mut dummy = 0u32;
    let mut ctx = GraphContext::new(&mut dummy);
    g.execute(&mut ctx).expect("graph execution");
    assert_eq!(count.load(Ordering::SeqCst), 3);
}

#[test]
fn adapter_nodes_compile() {
    let mut g = RenderGraph::new();
    g.add_node(ClearNode::new("clear", "surface", wgpu::Color::TRANSPARENT));
    g.add_node(RendererMainNode::new("main", "surface"));
    // do not execute (requires GPU). Just ensure GraphContext and node boxing type-checks.
    let mut dummy = 0usize;
    let mut ctx = GraphContext::new(&mut dummy);
    let _ = g.execute(&mut ctx).err();
}
