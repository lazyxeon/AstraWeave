#[cfg(test)]
mod tests {
    use astraweave_ecs::blob_vec::BlobVec;
    use astraweave_ecs::sparse_set::SparseSet;
    use astraweave_ecs::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Velocity {
        x: f32,
        y: f32,
    }

    struct Droppable(Arc<AtomicUsize>);
    impl Drop for Droppable {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    // ====================
    // BlobVec Tests
    // ====================

    #[test]
    fn test_blob_vec_basic() {
        let mut blob = BlobVec::new::<Position>();
        assert_eq!(blob.len(), 0);

        unsafe {
            blob.push(Position { x: 1.0, y: 2.0 });
            blob.push(Position { x: 3.0, y: 4.0 });
        }

        assert_eq!(blob.len(), 2);

        unsafe {
            let p1 = blob.get::<Position>(0).unwrap();
            let p2 = blob.get::<Position>(1).unwrap();
            assert_eq!(p1.x, 1.0);
            assert_eq!(p2.x, 3.0);
        }
    }

    #[test]
    fn test_blob_vec_reserve() {
        let mut blob = BlobVec::new::<Position>();
        blob.reserve(10);
        assert!(blob.capacity() >= 10);
    }

    #[test]
    fn test_blob_vec_drop() {
        let drop_count = Arc::new(AtomicUsize::new(0));
        {
            let mut blob = BlobVec::new::<Droppable>();
            unsafe {
                blob.push(Droppable(drop_count.clone()));
                blob.push(Droppable(drop_count.clone()));
            }
            assert_eq!(drop_count.load(Ordering::SeqCst), 0);
        }
        // BlobVec drop should trigger Droppable drops
        assert_eq!(drop_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_blob_vec_swap_remove() {
        let mut blob = BlobVec::new::<i32>();
        unsafe {
            blob.push(10);
            blob.push(20);
            blob.push(30);

            let val: i32 = blob.swap_remove(0); // Remove 10, 30 moves to index 0
            assert_eq!(val, 10);

            assert_eq!(blob.len(), 2);
            assert_eq!(*blob.get::<i32>(0).unwrap(), 30);
            assert_eq!(*blob.get::<i32>(1).unwrap(), 20);
        }
    }

    // ====================
    // CommandBuffer Tests
    // ====================

    #[test]
    fn test_command_buffer_basic() {
        let mut world = World::new();
        world.register_component::<Position>();
        world.register_component::<Velocity>();
        let mut commands = CommandBuffer::new();

        let e1 = world.spawn();
        commands.insert(e1, Position { x: 1.0, y: 1.0 });
        commands
            .spawn()
            .with(Position { x: 2.0, y: 2.0 })
            .with(Velocity { x: 0.0, y: 0.0 });
        commands.despawn(e1);

        commands.flush(&mut world);

        assert!(!world.is_alive(e1));
        assert_eq!(world.entity_count(), 1); // The spawned one
    }

    #[test]
    fn test_command_buffer_remove() {
        let mut world = World::new();
        world.register_component::<Position>();
        let e1 = world.spawn();
        world.insert(e1, Position { x: 1.0, y: 1.0 });

        let mut commands = CommandBuffer::new();
        commands.remove::<Position>(e1);
        commands.flush(&mut world);

        assert!(world.get::<Position>(e1).is_none());
    }

    // ====================
    // Events Tests
    // ====================

    #[derive(Debug, Clone, PartialEq)]
    struct TestEvent(u32);
    impl Event for TestEvent {}

    #[test]
    fn test_events_basic() {
        let mut events = Events::new();
        events.send(TestEvent(1));
        events.send(TestEvent(2));

        let read_events: Vec<_> = events.read::<TestEvent>().cloned().collect();
        assert_eq!(read_events, vec![TestEvent(1), TestEvent(2)]);

        events.update(); // Advance frame
        events.send(TestEvent(3));

        // Reader should still see old events if not updated
        let read_events2: Vec<_> = events.read::<TestEvent>().cloned().collect();
        assert_eq!(read_events2, vec![TestEvent(1), TestEvent(2), TestEvent(3)]);
    }

    #[test]
    fn test_events_cleanup() {
        let mut events = Events::new();
        events.send(TestEvent(1));
        events.send(TestEvent(2));

        let read_events: Vec<_> = events.read::<TestEvent>().cloned().collect();
        assert_eq!(read_events, vec![TestEvent(1), TestEvent(2)]);

        events.clear_all();
        let read_events2: Vec<_> = events.read::<TestEvent>().cloned().collect();
        assert!(read_events2.is_empty());
    }

    // ====================
    // Rng Tests
    // ====================

    #[test]
    fn test_rng_determinism() {
        let mut rng1 = Rng::from_seed(42);
        let mut rng2 = Rng::from_seed(42);

        assert_eq!(rng1.gen_u32(), rng2.gen_u32());
        assert_eq!(rng1.gen_range(0..100), rng2.gen_range(0..100));

        let mut vec1 = vec![1, 2, 3, 4, 5];
        let mut vec2 = vec![1, 2, 3, 4, 5];
        rng1.shuffle(&mut vec1);
        rng2.shuffle(&mut vec2);
        assert_eq!(vec1, vec2);
    }

    // ====================
    // SparseSet Tests
    // ====================

    #[test]
    fn test_sparse_set_basic() {
        let mut set = SparseSet::new();
        let e1 = unsafe { Entity::from_raw(1) };
        let e2 = unsafe { Entity::from_raw(10) };

        set.insert(e1);
        set.insert(e2);

        assert!(set.contains(e1));
        assert!(set.contains(e2));
        assert!(!set.contains(unsafe { Entity::from_raw(5) }));

        set.remove(e1);
        assert!(!set.contains(e1));
        assert!(set.contains(e2));
    }

    // ====================
    // TypeRegistry Tests
    // ====================

    #[test]
    fn test_type_registry() {
        let mut registry = TypeRegistry::new();
        registry.register::<Position>();

        assert!(registry.is_registered(std::any::TypeId::of::<Position>()));
        assert!(!registry.is_registered(std::any::TypeId::of::<Velocity>()));
    }
}
