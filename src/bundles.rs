use crate::{
    systems,
    systems::game::{EnemyCollisionSystemDesc, PlayerEventsSystemDesc},
};
use amethyst::{
    core::{bundle::SystemBundle, SystemDesc},
    ecs::DispatcherBuilder,
    prelude::World,
    Error,
};

#[derive(Debug)]
pub(crate) struct GameLogicBundle;

impl Default for GameLogicBundle {
    fn default() -> Self {
        GameLogicBundle {}
    }
}

impl<'a, 'b> SystemBundle<'a, 'b> for GameLogicBundle {
    fn build(
        self,
        world: &mut World,
        builder: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        builder.add(
            systems::game::InvincibilitySystem,
            "invincibility_system",
            &["apply_velocity_system"],
        );
        builder.add(
            EnemyCollisionSystemDesc::default().build(world),
            "enemy_collision_system",
            &["invincibility_system"],
        );
        builder.add(
            PlayerEventsSystemDesc::default().build(world),
            "player_events_system",
            &["enemy_collision_system"],
        );
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct GraphicsBundle;

impl Default for GraphicsBundle {
    fn default() -> Self {
        GraphicsBundle {}
    }
}

impl<'a, 'b> SystemBundle<'a, 'b> for GraphicsBundle {
    fn build(
        self,
        _world: &mut World,
        builder: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        builder.add(
            systems::graphics::SpriteUpdateSystem,
            "sprite_update_system",
            &["apply_velocity_system"],
        );
        builder.add(
            systems::graphics::TransformUpdateSystem,
            "transform_update_system",
            &["sprite_update_system"],
        );
        builder.add(
            systems::graphics::DeadDrawUpdateSystem,
            "dead_draw_update_system",
            &["transform_update_system"],
        );
        Ok(())
    }
}
