# ECS API Reference

> **Crate**: `astraweave-ecs`  
> **Manifest version**: `0.1.0`. The package table does not set `publish`.

## Quick Links

- [Source](https://github.com/lazyxeon/AstraWeave/tree/main/astraweave-ecs)
- [ECS Core Concepts](../architecture/ecs.md)

---

## World

`World::spawn` returns an `Entity` and takes no components. `insert` adds one component. `insert_resource` stores a singleton. `get_resource` returns `Option<&T>`.

```rust
use astraweave_ecs::World;

let mut world = World::new();
let entity = world.spawn();
world.insert(entity, 1u32);
assert!(world.is_alive(entity));
world.insert_resource(0.25f32);
let step = world.get_resource::<f32>().copied();
assert_eq!(step, Some(0.25));
let _removed = world.despawn(entity);
```

Other methods on the same impl: `get`, `get_mut`, `has`, `remove`, `count`, `each_mut`, `entity_count`. There is no `world.resource()`, `world.query()`, or `world.contains()`.

---

## Entity

`Entity` is `id: u32` plus `generation: u32`. `to_raw` packs `(id as u64) | ((generation as u64) << 32)`. `from_raw` is `unsafe` and takes that one `u64`.

---

## App and Schedule

`App` holds `world: World` and `schedule: Schedule`. `App::new` registers stages `pre_simulation`, `perception`, `simulation`, `sync`, `ai_planning`, `physics`, `post_simulation`, and `presentation`.

```rust
use astraweave_ecs::{App, World};

fn note(world: &mut World) {
    let _ = world.entity_count();
}

let mut app = App::new();
app.add_system("simulation", note).unwrap();
let _app = app.run_fixed(1);
```

`add_system` returns `Result<(), UnknownStage>` when the stage name was never registered. `SystemFn` is `fn(&mut World)`. `run_fixed(steps)` runs that many schedule passes.

---

## Component and Resource

```rust
pub trait Component: 'static + Send + Sync {}
impl<T: 'static + Send + Sync> Component for T {}

pub trait Resource: 'static + Send + Sync {}
impl<T: 'static + Send + Sync> Resource for T {}
```

There is no `#[derive(Component)]`.

---

## Query

`Query::<T>::new(&world)` iterates `(Entity, &T)` for archetypes that contain `T`. `Query2` and `Query2Mut` are the two-component forms.

```rust
use astraweave_ecs::{Query, World};

let mut world = World::new();
let entity = world.spawn();
world.insert(entity, 7u32);
let found: Vec<_> = Query::<u32>::new(&world).map(|(e, v)| (e, *v)).collect();
assert_eq!(found.len(), 1);
```

---

## CommandBuffer

`CommandBuffer::new`, `spawn`, `insert`, `remove`, `despawn`, then `flush(&mut world)`.

`spawn` returns a `SpawnBuilder`. The builder has `with` and no entity id. Dropping it queues a spawn. `flush` calls `World::spawn`, then `insert_boxed`. `insert_boxed` panics unless `World::register_component::<T>` ran for that component. `T` must be `Clone`.

```rust
use astraweave_ecs::{CommandBuffer, World};

let mut world = World::new();
world.register_component::<u32>();
let mut commands = CommandBuffer::new();
commands.spawn().with(1u32);
commands.flush(&mut world);
assert_eq!(world.entity_count(), 1);
```

---

## Events

`Event` is a marker trait with no blanket impl. `Events::new` keeps two frames. `send`, `read`, `drain`, and `update` are on `Events`.

```rust
use astraweave_ecs::{Event, Events};

struct Ping(u32);
impl Event for Ping {}

let mut events = Events::new();
events.send(Ping(1));
assert_eq!(events.len::<Ping>(), 1);
```

---

## Feature names

`astraweave-ecs/Cargo.toml` declares `profiling`, `loom`, `alloc-counter`, and `fast-alloc`. It does not set `default`.

---

## See Also

- [Architecture](../architecture/ecs.md)
- [Save and load](../game-dev/save-load.md)
