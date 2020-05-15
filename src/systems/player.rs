use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::{
    Join, NullStorage, Read, ReadStorage, System, SystemData, World, WriteStorage,
};
use amethyst::input::{InputHandler, StringBindings};

use crate::components::game::Health;
use crate::components::game::Player;
use crate::components::physics::{GravityDirection, Grounded, PlatformCuboid, Position, Velocity};
use crate::states::pizzatopia::{CAM_HEIGHT, TILE_HEIGHT};
use crate::systems::physics::{
    gravitationally_adapted_velocity, gravitationally_de_adapted_velocity,
};

#[derive(SystemDesc)]
pub struct PlayerInputSystem;

impl<'s> System<'s> for PlayerInputSystem {
    type SystemData = (
        WriteStorage<'s, Velocity>,
        Read<'s, InputHandler<StringBindings>>,
        ReadStorage<'s, Player>,
        ReadStorage<'s, Health>,
        ReadStorage<'s, Grounded>,
        ReadStorage<'s, GravityDirection>,
    );

    fn run(
        &mut self,
        (mut velocities, input, players, healths, grounded, gravities): Self::SystemData,
    ) {
        for (velocity, _player, health, ground, gravity) in (
            &mut velocities,
            &players,
            &healths,
            (&grounded).maybe(),
            (&gravities).maybe(),
        )
            .join()
        {
            if health.0 == 0 {
                continue;
            }
            // Controller input
            let v_move = input.axis_value("vertical_move");
            let h_move = input.axis_value("horizontal_move");

            // Get the grounded status to use auto-complete :)
            let ground: Option<&Grounded> = ground;
            let default_ground = Grounded(false);
            let on_ground = ground.unwrap_or(&default_ground);
            let on_ground = on_ground.0;

            let mut grav_vel = velocity.0.clone();
            if let Some(grav) = gravity {
                grav_vel = gravitationally_de_adapted_velocity(&grav_vel, &grav);
            }

            // Do the move logic
            if let Some(mv_amount) = v_move {
                if mv_amount > 0.0 {
                    if on_ground {
                        let jump_velocity = 13.0;
                        grav_vel.y += jump_velocity;
                    }
                }
                // letting go of `up` will stop your jump
                else if grav_vel.y > 0.0 {
                    grav_vel.y *= 0.85;
                }
            }
            if let Some(mv_amount) = h_move {
                let mut scaled_amount = 0.30 * mv_amount as f32;
                if on_ground {
                    scaled_amount += (grav_vel.x * 0.025).abs() * mv_amount;
                }
                grav_vel.x += scaled_amount;
            }

            if let Some(grav) = gravity {
                velocity.0 = gravitationally_adapted_velocity(&grav_vel, &grav);
            }
        }
    }
}
