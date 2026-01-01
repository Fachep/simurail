use crate::attribute::dynamic_modifier_on_dependency_attribute_dirty_observer;
use crate::attribute::modifier::DynamicModifierOnInsertCache;
use bevy::prelude::*;

pub struct AttributePlugin;

impl Plugin for AttributePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(dynamic_modifier_on_dependency_attribute_dirty_observer)
            .init_resource::<DynamicModifierOnInsertCache>();
    }
}
