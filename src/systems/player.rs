use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::{
    Join, NullStorage, Read, ReadStorage, System, SystemData, World, WriteStorage,
};
use amethyst::input::{InputHandler, StringBindings};

// You'll have to mark TILE_HEIGHT as public in pong.rs
use crate::components::physics::{Grounded, PlatformCuboid, Position, Velocity};
use crate::components::player::Player;
use crate::pizzatopia::{CAM_HEIGHT, TILE_HEIGHT};

#[derive(SystemDesc)]
pub struct PlayerInputSystem;

impl<'s> System<'s> for PlayerInputSystem {
    type SystemData = (
        WriteStorage<'s, Velocity>,
        Read<'s, InputHandler<StringBindings>>,
        ReadStorage<'s, Player>,
        ReadStorage<'s, Grounded>,
    );

    fn run(&mut self, (mut velocities, input, players, grounded): Self::SystemData) {
        for (velocity, player, ground) in (&mut velocities, &players, (&grounded).maybe()).join() {
            // Controller input
            let v_move = input.axis_value("vertical_move");
            let h_move = input.axis_value("horizontal_move");

            // Get the grounded status to use auto-complete :)
            let ground: Option<&Grounded> = ground;
            let default_ground = Grounded(false);
            let on_ground = ground.unwrap_or(&default_ground);
            let on_ground = on_ground.0;

            // Do the move logic
            if let Some(mv_amount) = v_move {
                if mv_amount > 0.0 {
                    if on_ground {
                        let jump_velocity = 12.0;
                        velocity.0.y += jump_velocity;
                    }
                }
                // letting go of `up` will stop your jump
                else if velocity.0.y > 0.0 {
                    velocity.0.y *= 0.85;
                }
            }
            if let Some(mv_amount) = h_move {
                let mut scaled_amount = 0.30 * mv_amount as f32;
                if on_ground {
                    scaled_amount += (velocity.0.x * 0.025).abs() * mv_amount;
                }
                velocity.0.x += scaled_amount;
            }
        }
    }
}
