//! Tests for indirect draw command generation and batching

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::culling::{
    batch_visible_instances, build_indirect_commands_cpu, BatchId, DrawBatch, DrawIndirectCommand,
};

#[test]
fn test_draw_indirect_command_creation() {
    let cmd = DrawIndirectCommand::new(36, 10, 0, 0);

    assert_eq!(cmd.vertex_count, 36);
    assert_eq!(cmd.instance_count, 10);
    assert_eq!(cmd.first_vertex, 0);
    assert_eq!(cmd.first_instance, 0);

    // Verify size matches wgpu::DrawIndirect (4 * u32 = 16 bytes)
    assert_eq!(std::mem::size_of::<DrawIndirectCommand>(), 16);
}

#[test]
fn test_draw_batch_building() {
    let batch_id = BatchId::new(0, 0);
    let mut batch = DrawBatch::new(batch_id, 36, 0);

    batch.add_instance(0);
    batch.add_instance(1);
    batch.add_instance(2);

    assert_eq!(batch.instance_count(), 3);
    assert_eq!(batch.instances, vec![0, 1, 2]);
}

#[test]
fn test_build_indirect_commands_from_batches() {
    let batches = vec![
        {
            let mut b = DrawBatch::new(BatchId::new(0, 0), 36, 0);
            b.add_instance(0);
            b.add_instance(1);
            b
        },
        {
            let mut b = DrawBatch::new(BatchId::new(1, 0), 24, 36);
            b.add_instance(2);
            b.add_instance(3);
            b.add_instance(4);
            b
        },
    ];

    let commands = build_indirect_commands_cpu(&batches);

    assert_eq!(commands.len(), 2);

    // First batch: 36 vertices, 2 instances
    assert_eq!(commands[0].vertex_count, 36);
    assert_eq!(commands[0].instance_count, 2);
    assert_eq!(commands[0].first_vertex, 0);

    // Second batch: 24 vertices, 3 instances
    assert_eq!(commands[1].vertex_count, 24);
    assert_eq!(commands[1].instance_count, 3);
    assert_eq!(commands[1].first_vertex, 36);
}

#[test]
fn test_batch_visible_instances() {
    // Simulate 6 instances: 3 cubes (mesh 0), 3 spheres (mesh 1)
    let visible_indices = vec![0, 1, 2, 3, 4, 5];

    let get_batch_id = |idx: u32| {
        if idx < 3 {
            BatchId::new(0, 0) // Cubes
        } else {
            BatchId::new(1, 0) // Spheres
        }
    };

    let get_mesh_info = |batch_id: BatchId| {
        match batch_id.mesh_id {
            0 => (36, 0),  // Cube: 36 vertices starting at 0
            1 => (24, 36), // Sphere: 24 vertices starting at 36
            _ => (0, 0),
        }
    };

    let batches = batch_visible_instances(&visible_indices, get_batch_id, get_mesh_info);

    assert_eq!(
        batches.len(),
        2,
        "Should have 2 batches (cubes and spheres)"
    );

    // Batches should be sorted by BatchId (BTreeMap ordering)
    assert_eq!(batches[0].batch_id, BatchId::new(0, 0));
    assert_eq!(batches[0].instance_count(), 3);
    assert_eq!(batches[0].instances, vec![0, 1, 2]);

    assert_eq!(batches[1].batch_id, BatchId::new(1, 0));
    assert_eq!(batches[1].instance_count(), 3);
    assert_eq!(batches[1].instances, vec![3, 4, 5]);
}

#[test]
fn test_batching_with_materials() {
    // 6 instances: mesh0/mat0, mesh0/mat1, mesh1/mat0
    let visible_indices = vec![0, 1, 2, 3, 4, 5];

    let get_batch_id = |idx: u32| match idx {
        0 | 1 => BatchId::new(0, 0), // mesh0, mat0
        2 | 3 => BatchId::new(0, 1), // mesh0, mat1
        4 | 5 => BatchId::new(1, 0), // mesh1, mat0
        _ => BatchId::new(0, 0),
    };

    let get_mesh_info = |_: BatchId| (36, 0);

    let batches = batch_visible_instances(&visible_indices, get_batch_id, get_mesh_info);

    assert_eq!(
        batches.len(),
        3,
        "Should have 3 batches (mesh+material combinations)"
    );

    // Verify batch ordering (sorted by BatchId)
    assert_eq!(batches[0].batch_id, BatchId::new(0, 0));
    assert_eq!(batches[0].instance_count(), 2);

    assert_eq!(batches[1].batch_id, BatchId::new(0, 1));
    assert_eq!(batches[1].instance_count(), 2);

    assert_eq!(batches[2].batch_id, BatchId::new(1, 0));
    assert_eq!(batches[2].instance_count(), 2);
}

#[test]
fn test_empty_batch_list() {
    let batches: Vec<DrawBatch> = vec![];
    let commands = build_indirect_commands_cpu(&batches);

    assert_eq!(
        commands.len(),
        0,
        "Empty batch list should produce no commands"
    );
}

#[test]
fn test_single_instance_per_batch() {
    let visible_indices = vec![0, 1, 2];

    // Each instance gets its own batch (different mesh)
    let get_batch_id = |idx: u32| BatchId::new(idx, 0);
    let get_mesh_info = |batch_id: BatchId| (36, batch_id.mesh_id * 36);

    let batches = batch_visible_instances(&visible_indices, get_batch_id, get_mesh_info);

    assert_eq!(batches.len(), 3);

    for (i, batch) in batches.iter().enumerate() {
        assert_eq!(
            batch.instance_count(),
            1,
            "Each batch should have 1 instance"
        );
        assert_eq!(batch.instances[0], i as u32);
    }
}
