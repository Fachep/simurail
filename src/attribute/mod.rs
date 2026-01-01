mod modifier;
mod plugin;
mod tag;
mod zone;

pub use modifier::*;
pub use plugin::*;
pub use tag::*;
pub use zone::*;

use bevy::ecs::entity::{EntityHashMap, EntityHashSet};
use bevy::ecs::error::CommandWithEntity;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::relationship::RelationshipSourceCollection;
use bevy::ecs::system::{QueryParamBuilder, SystemParam};
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use petgraph::algo::toposort;
use petgraph::prelude::DiGraph;
use petgraph::visit::GraphBase;
use std::mem::replace;
use std::collections::VecDeque;

#[derive(Component, Deref, Default, Clone, Copy, Debug, PartialEq, PartialOrd)]
#[component(on_insert = attribute_value_on_insert)]
pub struct AttributeValue(Option<f32>);

impl AttributeValue {
    pub fn new(value: Option<f32>) -> Self {
        Self(value)
    }
}

fn attribute_value_on_insert(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    if let Attribute::Fixed = world.get::<Attribute>(entity).unwrap() {
        assert!(world.get::<AttributeValue>(entity).is_some());
    }
    let dependents = world
        .get::<AttributeDependents>(entity)
        .iter()
        .flat_map(|d| d.0.iter())
        .copied()
        .collect::<Vec<_>>();
    for dependent in dependents {
        world.trigger(DependencyAttributeDirtyEvent(dependent));
        match world.get::<Attribute>(dependent) {
            Some(Attribute::Fixed) => {}
            Some(_) => {
                world
                    .commands()
                    .entity(dependent)
                    .insert(AttributeValue::new(None));
            }
            None => {}
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq)]
#[component(on_insert = attribute_on_insert)]
pub enum Attribute {
    Fixed,
    Plain(f32),
    BasedOn(Entity),
    Merged(EntityHashSet),
}

fn attribute_on_insert(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let mut refresh_value = true;
    let mut dependencies = None;
    match world.get::<Attribute>(entity).unwrap() {
        Attribute::Fixed => {
            refresh_value = false;
            assert!(world.get::<AttributeValue>(entity).is_some());
        }
        Attribute::BasedOn(base_entity) => {
            dependencies = Some(
                world
                    .get::<AttributeDependencies>(entity)
                    .cloned()
                    .unwrap_or_default()
                    .increase(*base_entity),
            );
        }
        Attribute::Merged(dependency_entities) => {
            let mut attribute_dependencies = world
                .get::<AttributeDependencies>(entity)
                .cloned()
                .unwrap_or_default();
            for dependency_entity in dependency_entities {
                attribute_dependencies = attribute_dependencies.increase(*dependency_entity);
            }
            dependencies = Some(attribute_dependencies);
        }
        Attribute::Plain(_) => {}
    }
    if let Some(dependencies) = dependencies {
        world.commands().entity(entity).insert(dependencies);
    }
    if refresh_value {
        world
            .commands()
            .entity(entity)
            .insert(AttributeValue::new(None));
    }
}

fn attribute_on_replace(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let mut dependencies = None;
    match world.get::<Attribute>(entity).unwrap() {
        Attribute::BasedOn(base_entity) => {
            dependencies = world
                .get::<AttributeDependencies>(entity)
                .cloned()
                .map(|d| d.release(*base_entity));
        }
        Attribute::Merged(dependency_entities) => {
            if let Some(mut attribute_dependencies) =
                world.get::<AttributeDependencies>(entity).cloned()
            {
                for dependency_entity in dependency_entities {
                    attribute_dependencies = attribute_dependencies.release(*dependency_entity);
                }
                dependencies = Some(attribute_dependencies);
            }
        }
        Attribute::Fixed => {}
        Attribute::Plain(_) => {}
    }
    if let Some(dependencies) = dependencies {
        world.commands().entity(entity).try_insert(dependencies);
    }
}

#[derive(Component, Deref, Default, Clone, Debug, PartialEq, Eq)]
#[component(on_replace = attribute_value_dependents_on_replace)]
pub struct AttributeDependents(pub EntityHashSet);

fn attribute_value_dependents_on_replace(
    mut world: DeferredWorld,
    HookContext { entity, .. }: HookContext,
) {
    let dependent_entities = replace(
        &mut world.get_mut::<AttributeDependents>(entity).unwrap().0,
        EntityHashSet::default(),
    );
    for dependent_entity in dependent_entities {
        if let Ok(mut dependent_entity_mut) = world.get_entity_mut(dependent_entity) {
            if let Some(mut dependency) = dependent_entity_mut.get_mut::<AttributeDependencies>() {
                dependency.0.remove(&entity);
                if dependency.0.is_empty() {
                    let command = |mut dependent_entity: EntityWorldMut| {
                        if dependent_entity
                            .get::<AttributeDependencies>()
                            .is_some_and(|d| d.0.is_empty())
                        {
                            dependent_entity.remove::<AttributeDependencies>();
                        }
                    };
                    world
                        .commands()
                        .queue_silenced(command.with_entity(dependent_entity));
                }
            }
        }
    }
}

#[derive(Component, Deref, Default, Clone, Debug, PartialEq, Eq)]
#[component(on_insert = attribute_dependencies_on_insert, on_replace = attribute_dependencies_on_replace)]
pub struct AttributeDependencies(pub EntityHashMap<usize>);

impl AttributeDependencies {
    pub fn increase(mut self, entity: Entity) -> Self {
        *self.0.entry(entity).or_insert(0) += 1;
        self
    }
    pub fn release(mut self, entity: Entity) -> Self {
        if let Some(count) = self.0.get_mut(&entity) {
            *count = count.saturating_sub(1);
        }
        self
    }
}

fn attribute_dependencies_on_insert(
    mut world: DeferredWorld,
    HookContext { entity, .. }: HookContext,
) {
    world
        .get_mut::<AttributeDependencies>(entity)
        .unwrap()
        .0
        .retain(|_, c| *c != 0);
    if world
        .get::<AttributeDependencies>(entity)
        .unwrap()
        .0
        .is_empty()
    {
        world
            .commands()
            .entity(entity)
            .remove::<AttributeDependencies>();
        return;
    }
    let dependency_entities = world
        .get::<AttributeDependencies>(entity)
        .unwrap()
        .0
        .keys()
        .copied()
        .collect::<Vec<_>>();
    for dependency_entity in dependency_entities {
        if let Ok(mut dependency_entity_commands) = world.commands().get_entity(dependency_entity) {
            dependency_entity_commands
                .entry::<AttributeDependents>()
                .and_modify(move |mut d| {
                    d.0.insert(entity);
                })
                .or_insert_with(move || {
                    let mut set = EntityHashSet::with_capacity(2);
                    set.insert(entity);
                    AttributeDependents(set)
                });
        } else {
            world
                .commands()
                .entity(entity)
                .remove::<AttributeDependencies>();
            break;
        }
    }
}

fn attribute_dependencies_on_replace(
    mut world: DeferredWorld,
    HookContext { entity, .. }: HookContext,
) {
    let dependency_entities = replace(
        &mut world.get_mut::<AttributeDependencies>(entity).unwrap().0,
        EntityHashMap::default(),
    );
    for dependency_entity in dependency_entities
        .iter()
        .filter(|(_, c)| **c != 0)
        .map(|(e, _)| *e)
    {
        if let Ok(mut dependency_entity_mut) = world.get_entity_mut(dependency_entity) {
            if let Some(mut dependents) = dependency_entity_mut.get_mut::<AttributeDependents>() {
                dependents.0.remove(entity);
                if dependents.0.is_empty() {
                    let command = |mut dependency_entity: EntityWorldMut| {
                        if dependency_entity
                            .get::<AttributeDependents>()
                            .is_some_and(|d| d.0.is_empty())
                        {
                            dependency_entity.remove::<AttributeDependents>();
                        }
                    };
                    world
                        .commands()
                        .queue_silenced(command.with_entity(dependency_entity));
                }
            }
        }
    }
}

#[derive(EntityEvent)]
pub struct DependencyAttributeDirtyEvent(pub Entity);

#[derive(SystemParam)]
#[system_param(builder)]
pub struct AttributeQueries<'w, 's> {
    pub attributes: Query<'w, 's, (&'static Attribute, Option<&'static Modifiers>)>,
    pub attribute_values: Query<'w, 's, &'static mut AttributeValue, With<Attribute>>,
    pub modifier_values: Query<'w, 's, &'static ModifierValue>,
}

impl<'w, 's> AttributeQueries<'w, 's> {
    pub fn builder() -> impl SystemParamBuilder<Self> {
        AttributeQueriesBuilder {
            attributes: QueryParamBuilder::new(|_| {}),
            attribute_values: QueryParamBuilder::new(|_| {}),
            modifier_values: QueryParamBuilder::new(|_| {}),
        }
    }
}

#[derive(Default)]
pub struct AttributeEvaluator {
    cache: EntityHashMap<f32>,
}

impl AttributeEvaluator {
    pub fn fetch_value(&mut self, queries: &mut AttributeQueries, entity: Entity) -> Option<f32> {
        if let Some(value) = self.cache.get(&entity) {
            return Some(*value);
        }
        match queries.attribute_values.get(entity) {
            Ok(AttributeValue(Some(value))) => {
                self.cache.insert(entity, *value);
                return Some(*value);
            }
            Err(_) => {
                return None;
            }
            _ => {}
        }
        match queries.attributes.get(entity) {
            Ok((Attribute::Fixed, _)) => {
                let value = queries.attribute_values.get(entity).unwrap().0.unwrap();
                self.cache.insert(entity, value);
                return Some(value);
            }
            Ok((Attribute::Plain(_), None)) => {
                if queries.attribute_values.get(entity).unwrap().0.is_none() {
                    *queries.attribute_values.get_mut(entity).unwrap() = AttributeValue(Some(0.0))
                }
                self.cache.insert(entity, 0.0);
                return Some(0.0);
            }
            Ok((Attribute::Plain(base), Some(modifiers))) => {
                if let AttributeValue(Some(value)) = queries.attribute_values.get(entity).unwrap() {
                    self.cache.insert(entity, *value);
                    return Some(*value);
                }
                let (ratio, delta) = Self::merge_modifiers(queries, modifiers);
                let value = base * ratio + delta;
                *queries.attribute_values.get_mut(entity).unwrap() = AttributeValue(Some(value));
                self.cache.insert(entity, value);
                return Some(value);
            }
            Err(_) => {
                return None;
            }
            _ => {}
        }
        let mut graph = DiGraph::<Entity, ()>::new();
        let mut entity_node_map: EntityHashMap<<DiGraph<Entity, ()> as GraphBase>::NodeId> =
            EntityHashMap::with_capacity(2);
        let mut entity_queue = VecDeque::with_capacity(2);
        entity_queue.push_back(entity);
        entity_node_map.insert(entity, graph.add_node(entity));
        while let Some(current_entity) = entity_queue.pop_front() {
            if let AttributeValue(Some(value)) =
                queries.attribute_values.get(current_entity).unwrap()
            {
                self.cache.insert(current_entity, *value);
                continue;
            }
            let current_id = *entity_node_map.get(&current_entity).unwrap();
            match queries.attributes.get(current_entity) {
                Ok((Attribute::BasedOn(base_entity), Some(_))) => {
                    if self.cache.contains_key(base_entity) {
                        continue;
                    }
                    let base_id = if let Some(id) = entity_node_map.get(base_entity) {
                        *id
                    } else {
                        let id = graph.add_node(*base_entity);
                        entity_node_map.insert(*base_entity, id);
                        entity_queue.push_back(*base_entity);
                        id
                    };
                    graph.update_edge(base_id, current_id, ());
                }
                Ok((Attribute::Merged(dependency_entities), _)) => {
                    for dependency_entity in dependency_entities {
                        if self.cache.contains_key(dependency_entity) {
                            continue;
                        }
                        let dependency_id = if let Some(id) = entity_node_map.get(dependency_entity)
                        {
                            *id
                        } else {
                            let id = graph.add_node(*dependency_entity);
                            entity_node_map.insert(*dependency_entity, id);
                            entity_queue.push_back(*dependency_entity);
                            id
                        };
                        graph.update_edge(dependency_id, current_id, ());
                    }
                }
                _ => {}
            };
        }
        let sorted_entities = toposort(&graph, None)
            .unwrap()
            .into_iter()
            .map(|i| *graph.node_weight(i).unwrap());
        for entity in sorted_entities {
            let (attribute, modifiers) = queries.attributes.get(entity).unwrap();
            let merged_ratio = modifiers.map(|modifiers| Self::merge_modifiers(queries, modifiers));
            let value = match attribute {
                Attribute::Fixed => queries.attribute_values.get(entity).unwrap().0.unwrap(),
                Attribute::Plain(base) => {
                    if let Some((ratio, delta)) = merged_ratio {
                        base * ratio + delta
                    } else {
                        0.0
                    }
                }
                Attribute::BasedOn(base_entity) => {
                    if let Some((ratio, delta)) = merged_ratio {
                        let base = self.get_cache_or_fetch_attribute_value(queries, *base_entity);
                        base * ratio + delta
                    } else {
                        0.0
                    }
                }
                Attribute::Merged(dependency_entities) => dependency_entities
                    .iter()
                    .copied()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|e| self.get_cache_or_fetch_attribute_value(queries, e))
                    .sum(),
            };
            *queries.attribute_values.get_mut(entity).unwrap() = AttributeValue(Some(value));
            self.cache.insert(entity, value);
        }
        self.cache.get(&entity).copied()
    }

    fn get_cache_or_fetch_attribute_value(
        &mut self,
        queries: &mut AttributeQueries,
        entity: Entity,
    ) -> f32 {
        if let Some(v) = self.cache.get(&entity) {
            *v
        } else {
            let v = queries.attribute_values.get(entity).unwrap().0.unwrap();
            self.cache.insert(entity, v);
            v
        }
    }

    fn merge_modifiers(queries: &AttributeQueries, modifiers: &Modifiers) -> (f32, f32) {
        println!("Merging {} modifiers", modifiers.len());
        modifiers
            .iter()
            .map(|e| queries.modifier_values.get(e).unwrap())
            .fold((0.0, 0.0), |(ratio, delta), m| {
                println!("Merging modifier: ratio {}, delta {}", m.ratio, m.delta);
                (ratio + m.ratio, delta + m.delta)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[test]
    fn test_attribute_evaluator() {
        let mut world = World::new();

        world.add_observer(|e: On<Insert, AttributeValue>, a: Query<&AttributeValue>| {
            println!(
                "AttributeValue inserted for entity {:?}: {:?}",
                e.entity,
                a.get(e.entity)
            );
        });

        let attr_a = world
            .spawn((Attribute::Fixed, AttributeValue(Some(42.0))))
            .id();

        let attr_b = world.spawn((Attribute::BasedOn(attr_a),)).id();

        let attr_c = world.spawn(Attribute::Plain(100.0)).id();

        world.spawn((
            Modifier(attr_b),
            ModifierValue {
                ratio: 0.5,
                delta: 8.0,
            },
        ));

        world.spawn((
            Modifier(attr_c),
            ModifierValue {
                ratio: 0.7,
                delta: -10.0,
            },
        ));

        let attr_d = world
            .spawn((Attribute::Merged({
                let mut set = EntityHashSet::new();
                set.insert(attr_b);
                set.insert(attr_c);
                set
            }),))
            .id();

        let mut state = AttributeQueries::builder().build_state(&mut world);

        {
            let mut queries = state.get_mut(&mut world);
            let mut evaluator = AttributeEvaluator::default();
            let value_d = evaluator.fetch_value(&mut queries, attr_d).unwrap();
            assert_eq!(value_d, (42.0 * 0.5 + 8.0) + (100.0 * 0.7 - 10.0));
        }

        world.entity_mut(attr_a).insert(AttributeValue(Some(84.0)));

        {
            let mut queries = state.get_mut(&mut world);
            let mut evaluator = AttributeEvaluator::default();
            let value_d = evaluator.fetch_value(&mut queries, attr_d).unwrap();
            assert_eq!(value_d, (84.0 * 0.5 + 8.0) + (100.0 * 0.7 - 10.0));
        }
    }
}
