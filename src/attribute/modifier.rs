use crate::attribute::{
    AttributeDependencies, AttributeEvaluator, AttributeQueries, AttributeValue,
    DependencyAttributeDirtyEvent,
};
use bevy::ecs::entity::EntityHashSet;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::system::SystemState;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

#[derive(Component, Default, Clone, Copy, Debug, PartialEq, PartialOrd)]
#[component(immutable)]
#[component(on_insert = modifier_value_on_insert)]
#[component(on_remove = modifier_value_on_remove)]
pub struct ModifierValue {
    pub ratio: f32,
    pub delta: f32,
}

fn modifier_value_on_insert(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let target_entity = world.get::<Modifier>(entity).unwrap().0;
    if world.entity(target_entity).contains::<AttributeValue>() {
        world
            .commands()
            .entity(target_entity)
            .insert(AttributeValue::default());
    }
}

fn modifier_value_on_remove(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let target_entity = world.get::<Modifier>(entity).unwrap().0;
    if world
        .get_entity(target_entity)
        .is_ok_and(|e| e.contains::<AttributeValue>())
    {
        world
            .commands()
            .entity(target_entity)
            .try_insert(AttributeValue::default());
    }
}

#[derive(Component, Deref, Default, Clone, Debug, PartialEq, Eq)]
#[relationship_target(relationship = Modifier, linked_spawn)]
pub struct Modifiers(EntityHashSet);

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[relationship(relationship_target = Modifiers)]
#[component(immutable)]
pub struct Modifier(pub Entity);

impl Modifier {
    pub fn new(target: Entity, ratio: f32, delta: f32) -> impl Bundle {
        (Modifier(target), ModifierValue { ratio, delta })
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, PartialOrd)]
#[component(immutable)]
#[component(on_insert = dynamic_modifier_on_insert)]
#[component(on_replace = dynamic_modifier_on_replace)]
pub struct DynamicModifier {
    pub source: Entity,
    pub threshold: f32,
    pub ratio: f32,
    pub delta: f32,
    pub modifier_type: DynamicModifierType,
}

impl DynamicModifier {
    pub fn new(
        target: Entity,
        source: Entity,
        threshold: f32,
        ratio: f32,
        delta: f32,
        modifier_type: DynamicModifierType,
    ) -> impl Bundle {
        (
            Modifier(target),
            DynamicModifier {
                source,
                threshold,
                ratio,
                delta,
                modifier_type,
            },
        )
    }

    pub fn new_copy(
        target: Entity,
        source: Entity,
        threshold: f32,
        ratio: f32,
        delta: f32,
    ) -> impl Bundle {
        Self::new(
            target,
            source,
            threshold,
            ratio,
            delta,
            DynamicModifierType::Copy,
        )
    }

    pub fn new_scale(
        target: Entity,
        source: Entity,
        threshold: f32,
        ratio: f32,
        delta: f32,
    ) -> impl Bundle {
        Self::new(
            target,
            source,
            threshold,
            ratio,
            delta,
            DynamicModifierType::Scale,
        )
    }

    pub fn new_scale_without_threshold(
        target: Entity,
        source: Entity,
        threshold: f32,
        ratio: f32,
        delta: f32,
    ) -> impl Bundle {
        Self::new(
            target,
            source,
            threshold,
            ratio,
            delta,
            DynamicModifierType::ScaleWithoutThreshold,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DynamicModifierType {
    Copy,
    Scale,
    ScaleWithoutThreshold,
}

fn dynamic_modifier_on_insert(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let dynamic_modifier = *world.get::<DynamicModifier>(entity).unwrap();
    let dependencies = world
        .get::<AttributeDependencies>(entity)
        .cloned()
        .unwrap_or_default();
    world
        .commands()
        .entity(entity)
        .insert(dependencies.increase(dynamic_modifier.source));
    let (ratio, delta) = unsafe {
        let world_mut = world.as_unsafe_world_cell().world_mut();
        world_mut.resource_scope(|world, mut state: Mut<DynamicModifierOnInsertCache>| {
            let mut attribute_evaluator = AttributeEvaluator::default();
            let mut attribute_queries = state.attribute_queries_state.get_mut(world);
            calculate_dynamic_modifier_value(
                &dynamic_modifier,
                &mut attribute_queries,
                &mut attribute_evaluator,
            )
        })
    };
    world
        .commands()
        .entity(entity)
        .insert(ModifierValue { ratio, delta });
}

fn dynamic_modifier_on_replace(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let source_entity = world.get::<DynamicModifier>(entity).unwrap().source;
    if let Some(dependencies) = world.get::<AttributeDependencies>(entity).cloned() {
        world
            .commands()
            .entity(entity)
            .try_insert(dependencies.release(source_entity));
    }
}

fn calculate_dynamic_modifier_value(
    dynamic_modifier: &DynamicModifier,
    attribute_queries: &mut AttributeQueries,
    attribute_evaluator: &mut AttributeEvaluator,
) -> (f32, f32) {
    let Some(source_value) =
        attribute_evaluator.fetch_value(attribute_queries, dynamic_modifier.source)
    else {
        return (0.0, 0.0);
    };
    let above_threshold = source_value >= dynamic_modifier.threshold;
    match dynamic_modifier.modifier_type {
        DynamicModifierType::Copy => {
            if above_threshold {
                (dynamic_modifier.ratio, dynamic_modifier.delta)
            } else {
                (0.0, 0.0)
            }
        }
        DynamicModifierType::Scale => {
            if above_threshold {
                (
                    dynamic_modifier.ratio * source_value,
                    dynamic_modifier.delta * source_value,
                )
            } else {
                (0.0, 0.0)
            }
        }
        DynamicModifierType::ScaleWithoutThreshold => {
            if above_threshold {
                let excess = source_value - dynamic_modifier.threshold;
                (
                    dynamic_modifier.ratio * excess,
                    dynamic_modifier.delta * excess,
                )
            } else {
                (0.0, 0.0)
            }
        }
    }
}

pub fn dynamic_modifier_on_dependency_attribute_dirty_observer(
    event: On<DependencyAttributeDirtyEvent, DynamicModifier>,
    dynamic_modifiers: Query<&DynamicModifier>,
    mut attribute_queries: AttributeQueries,
    mut commands: Commands,
) {
    let mut attribute_evaluator = AttributeEvaluator::default();
    let (ratio, delta) = calculate_dynamic_modifier_value(
        dynamic_modifiers.get(event.event_target()).unwrap(),
        &mut attribute_queries,
        &mut attribute_evaluator,
    );
    commands
        .entity(event.event_target())
        .insert(ModifierValue { ratio, delta });
}

#[derive(Resource, FromWorld)]
pub(super) struct DynamicModifierOnInsertCache {
    attribute_queries_state: SystemState<AttributeQueries<'static, 'static>>,
}
