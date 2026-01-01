use bevy::ecs::component::Tick;
use bevy::ecs::query::FilteredAccessSet;
use bevy::ecs::system::{ReadOnlySystemParam, SystemMeta, SystemParam};
use bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy::prelude::{Deref, DerefMut, World};

#[derive(Deref, DerefMut)]
pub struct FromDefault<T: Default>(T);

unsafe impl<T: Default> SystemParam for FromDefault<T> {
    type State = ();
    type Item<'world, 'state> = FromDefault<T>;

    fn init_state(world: &mut World) -> Self::State {}

    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut FilteredAccessSet,
        world: &mut World,
    ) {
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        FromDefault(T::default())
    }
}

unsafe impl<T: Default> ReadOnlySystemParam for FromDefault<T> {}
