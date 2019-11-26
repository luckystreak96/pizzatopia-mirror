use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::{
    Join, NullStorage, Read, ReadStorage, System, SystemData, World, WriteStorage,
};
use amethyst::input::{InputHandler, StringBindings};

// You'll have to mark TILE_HEIGHT as public in pong.rs
use crate::components::physics::{PlatformCuboid, Position, Velocity};
use crate::components::player::Player;
use crate::pizzatopia::{CAM_HEIGHT, TILE_HEIGHT};

#[derive(SystemDesc)]
pub struct PlayerInputSystem;

impl<'s> System<'s> for PlayerInputSystem {
    type SystemData = (
        WriteStorage<'s, Velocity>,
        Read<'s, InputHandler<StringBindings>>,
        ReadStorage<'s, Player>,
    );

    fn run(&mut self, (mut velocities, input, players): Self::SystemData) {
        for (velocity, player) in (&mut velocities, &players).join() {
            let v_move = input.axis_value("vertical_move");
            let h_move = input.axis_value("horizontal_move");
            if let Some(mv_amount) = v_move {
                let scaled_amount = 0.3 * mv_amount as f32;
                velocity.0.y += scaled_amount;
            }
            if let Some(mv_amount) = h_move {
                let scaled_amount = 0.1 * mv_amount as f32;
                velocity.0.x += scaled_amount;
            }
        }
    }
}
