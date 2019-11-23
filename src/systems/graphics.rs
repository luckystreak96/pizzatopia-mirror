use crate::components::physics::{PlatformCuboid, Position, Velocity};
use crate::pizzatopia::{TILE_HEIGHT, TILE_WIDTH};
use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::{Join, Read, ReadStorage, System, SystemData, World, WriteStorage};

#[derive(SystemDesc)]
pub struct PositionDrawUpdateSystem;

impl<'s> System<'s> for PositionDrawUpdateSystem {
    type SystemData = (WriteStorage<'s, Transform>, ReadStorage<'s, Position>);

    fn run(&mut self, (mut transforms, positions): Self::SystemData) {
        for (transform, position) in (&mut transforms, &positions).join() {
            transform.set_translation_xyz(position.0.x, position.0.y, 0.0);
        }
    }
}
