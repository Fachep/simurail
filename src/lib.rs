mod attribute;
mod utils;

#[cfg(test)]
mod tests {
    use crate::attribute::*;
    use bevy::ecs::entity::EntityHashSet;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;
    use std::fmt::Display;

    #[test]
    fn a() {
        let mut app = App::new();
        app.add_plugins(AttributePlugin);
        app.finish();
        app.cleanup();
        let world = app.world_mut();

        world.flush();

        let robin = spawn_attributes(world, "Robin");
        let danheng = spawn_attributes(world, "DanHeng");
        let phainon = spawn_attributes(world, "Phainon");

        fn print_attributes(
            query: Query<(Entity, &Name), With<Attribute>>,
            mut attribute_queries: AttributeQueries,
        ) {
            let mut attribute_evaluator = AttributeEvaluator::default();
            for (entity, name) in query
                .iter()
                .sort_by_cached_key::<(Entity, &Name), _>(|(e, _)| e.index())
            {
                let value = attribute_evaluator
                    .fetch_value(&mut attribute_queries, entity)
                    .unwrap();
                println!("Entity {:?} ({}): {}", entity, name.as_str(), value);
            }
        }

        world.run_system_once(print_attributes).unwrap();

        // robin base modifier
        world.spawn(Modifier::new(robin[0], 0.0, 640.0 + 635.0));
        // robin delta modifier
        world.spawn(Modifier::new(
            robin[1],
            0.12 + 0.432 * 2.0 + 0.116 + 0.112 + 0.12 + 0.12 + 0.28,
            352.0 + 19.0,
        ));

        // danheng base modifier
        world.spawn(Modifier::new(danheng[0], 0.0, 582.0 + 476.0));
        // danheng delta modifier
        world.spawn(Modifier::new(
            danheng[1],
            0.432 * 2.0 + 0.159 + 0.086 + 0.077 + 0.125 + 0.28,
            352.0 + 16.0 + 19.0 + 61.0 + 35.0,
        ));

        // phainon base modifier
        world.spawn(Modifier::new(phainon[0], 0.0, 582.0 + 687.0 + 1.0));
        // phainon delta modifier
        world.spawn(Modifier::new(phainon[1], 0.432 * 2.0, 352.0 + 21.0));
        let phainon_talent_modifier = world.spawn(Modifier::new(phainon[1], 0.5, 0.0)).id(); // 照见英雄本色
        world.spawn(Modifier::new(phainon[1], 0.12, 0.0));

        world.spawn(DynamicModifier::new_scale(
            phainon[2], danheng[3], 0.0, 0.0, 0.15,
        )); // 神秀

        println!("initial modifiers added");
        world.run_system_once(print_attributes).unwrap();

        let robin_lightcone_modifier = world.spawn(Modifier::new(robin[1], 0.48, 0.0)).id(); // 夜色流光溢彩

        let robin_ultimate_modifiers = [
            world
                .spawn(DynamicModifier::new_scale(
                    robin[2], robin[3], 0.0, 0.0, 0.228,
                ))
                .id(),
            world.spawn(Modifier::new(robin[2], 0.0, 200.0)).id(),
            world
                .spawn(DynamicModifier::new_scale(
                    danheng[2], robin[3], 0.0, 0.0, 0.228,
                ))
                .id(),
            world.spawn(Modifier::new(danheng[2], 0.0, 200.0)).id(),
            world
                .spawn(DynamicModifier::new_scale(
                    phainon[2], robin[3], 0.0, 0.0, 0.228,
                ))
                .id(),
            world.spawn(Modifier::new(phainon[2], 0.0, 200.0)).id(),
        ];
        println!("robin ultimate modifiers added");
        world.run_system_once(print_attributes).unwrap();

        let phainon_suit_modifier = world.spawn(Modifier::new(phainon[1], 0.48, 0.0)).id(); // 船长
        let phainon_ultimate_modifier = world.spawn(Modifier::new(phainon[1], 0.8, 0.0)).id(); // 此躯即神

        println!("phainon ultimate modifiers added");
        world.run_system_once(print_attributes).unwrap();

        world.despawn(phainon_ultimate_modifier);
        println!("phainon ultimate modifier removed");
        world.run_system_once(print_attributes).unwrap();

        world
            .entity_mut(phainon_talent_modifier)
            .insert(ModifierValue {
                ratio: 0.5 * 2.0,
                delta: 0.0,
            });
        println!("phainon talent modifier updated");
        world.run_system_once(print_attributes).unwrap();

        for entity in robin_ultimate_modifiers {
            world.despawn(entity);
        }
        println!("robin ultimate modifiers removed");
        world.run_system_once(print_attributes).unwrap();

        world.despawn(robin_lightcone_modifier);
        println!("robin lightcone modifier removed");
        world.run_system_once(print_attributes).unwrap();

        world.despawn(phainon_suit_modifier);
        println!("phainon suit modifier removed");
        world.run_system_once(print_attributes).unwrap();

        world
            .run_system_once(
                |query: Query<Entity, Or<(With<Attribute>, With<Modifier>)>>,
                 mut commands: Commands| {
                    for entity in query {
                        commands.entity(entity).despawn();
                    }
                },
            )
            .unwrap();
    }

    fn spawn_attributes(world: &mut World, name: impl Display) -> [Entity; 5] {
        let base = world
            .spawn((Attribute::Plain(0.0), Name::new(format!("{} Base", name))))
            .flush();
        let delta = world
            .spawn((
                Attribute::BasedOn(base),
                Name::new(format!("{} Delta", name)),
            ))
            .flush();
        let extra = world
            .spawn((
                Attribute::BasedOn(base),
                Name::new(format!("{} Extra", name)),
            ))
            .flush();
        let safe = world
            .spawn((
                Attribute::Merged({
                    let mut set = EntityHashSet::with_capacity(2);
                    set.insert(base);
                    set.insert(delta);
                    set
                }),
                Name::new(format!("{} Safe", name)),
            ))
            .flush();
        let final_ = world
            .spawn((
                Attribute::Merged({
                    let mut set = EntityHashSet::with_capacity(2);
                    set.insert(safe);
                    set.insert(extra);
                    set
                }),
                Name::new(format!("{} Final", name)),
            ))
            .flush();
        [base, delta, extra, safe, final_]
    }
}
