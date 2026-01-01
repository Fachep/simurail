use crate::attribute::Attribute;
use bevy::prelude::*;
use std::fmt::Debug;
use std::hash::Hash;

#[derive(Component, Reflect, Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[reflect(Component)]
#[reflect(Default, PartialEq)]
#[component(immutable)]
pub struct BaseZoneAttribute;

impl AttributeZone for BaseZoneAttribute {}

#[derive(Component, Reflect, Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[reflect(Component)]
#[reflect(Default, PartialEq)]
#[component(immutable)]
pub struct DeltaZoneAttribute;

impl AttributeZone for DeltaZoneAttribute {}

#[derive(Component, Reflect, Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[reflect(Component)]
#[reflect(Default, PartialEq)]
#[component(immutable)]
pub struct ExtraZoneAttribute;

impl AttributeZone for ExtraZoneAttribute {}

#[derive(Component, Reflect, Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[reflect(Component)]
#[reflect(Default, PartialEq)]
#[component(immutable)]
pub struct SafeZoneAttribute;

impl AttributeZone for SafeZoneAttribute {}

#[derive(Component, Reflect, Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[reflect(Component)]
#[reflect(Default, PartialEq)]
#[component(immutable)]
pub struct FinalZoneAttribute;

impl AttributeZone for FinalZoneAttribute {}

pub trait AttributeZone:
    Component + Reflect + Default + Clone + Copy + Debug + PartialEq + Eq + Hash + PartialOrd + Ord
{
}
