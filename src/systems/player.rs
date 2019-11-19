use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::{Join, Read, ReadStorage, System, SystemData, World, WriteStorage};
use amethyst::input::{InputHandler, StringBindings};

// You'll have to mark TILE_HEIGHT as public in pong.rs
use crate::components::physics::Cuboid;
use crate::pizzatopia::{CAM_HEIGHT, TILE_HEIGHT};

#[derive(SystemDesc)]
pub struct PlayerSystem;

impl<'s> System<'s> for PlayerSystem {
    type SystemData = (
        WriteStorage<'s, Transform>,
        ReadStorage<'s, Cuboid>,
        Read<'s, InputHandler<StringBindings>>,
    );

    fn run(&mut self, (mut transforms, tiles, input): Self::SystemData) {
        for (paddle, transform) in (&tiles, &mut transforms).join() {
            let movement = input.axis_value("player");
            if let Some(mv_amount) = movement {
                if mv_amount == 0. {
                    continue;
                }
                let scaled_amount = 1.2 * mv_amount as f32;
                transform.prepend_translation_y(scaled_amount);
                println!("{}", scaled_amount);
            }
        }
    }
}
