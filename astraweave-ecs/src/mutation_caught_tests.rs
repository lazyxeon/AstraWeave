//! Tests for mutants that the November 10 `cargo mutants` run left missed.
//!
//! Each assertion names a value the unmutated function returns, so a
//! replacement with `0`, `1`, `None`, or an empty body fails here.

use std::any::TypeId;

use crate::{Schedule, UnknownStage, World};

#[test]
fn change_tick_advances_and_filters_with_a_strict_greater_than() {
    let mut world = World::new();
    assert_eq!(world.change_tick(), 0);
    assert_eq!(world.increment_change_tick(), 1);
    assert_eq!(world.change_tick(), 1);
    assert_eq!(world.increment_change_tick(), 2);
    assert_eq!(world.change_tick(), 2);

    let entity = world.spawn();
    world.insert(entity, 7u32);
    assert!(world.get_mut::<u32>(entity).is_some());
    assert_eq!(world.get_change_tick::<u32>(entity), Some(2));

    let mut since_one = Vec::new();
    world.each_changed::<u32>(1, |id, value| since_one.push((id, *value)));
    assert_eq!(since_one, vec![(entity, 7)]);

    let mut since_two = Vec::new();
    world.each_changed::<u32>(2, |id, value| since_two.push((id, *value)));
    assert!(since_two.is_empty());

    let dead = entity;
    assert!(world.despawn(dead));
    assert_eq!(world.get_change_tick::<u32>(dead), None);
}

#[test]
fn add_stage_inserts_each_name_once() {
    let mut schedule = Schedule::default();
    schedule.add_stage("sim");
    schedule.add_stage("sim");
    assert_eq!(schedule.stages.len(), 1);
    assert_eq!(schedule.stages[0].name, "sim");
    schedule.add_stage("present");
    assert_eq!(schedule.stages.len(), 2);
}

#[test]
fn unknown_stage_display_names_the_stage() {
    let text = UnknownStage { stage: "sim" }.to_string();
    assert!(text.contains("sim"));
    assert!(text.contains("not registered"));
}

#[test]
fn append_entities_with_component_writes_the_live_entity() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, 7u32);
    let mut out = Vec::new();
    world
        .archetypes()
        .append_entities_with_component(TypeId::of::<u32>(), &mut out);
    assert_eq!(out, vec![entity]);
}
