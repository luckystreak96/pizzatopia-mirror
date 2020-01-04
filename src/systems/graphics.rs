use crate::components::physics::{PlatformCuboid, Position, Velocity};
use crate::pizzatopia::{TILE_HEIGHT, TILE_WIDTH};
use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::{Join, Read, ReadStorage, System, SystemData, World, WriteStorage};
use amethyst::renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture};

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

#[derive(SystemDesc)]
pub struct SpriteUpdateSystem;

impl<'s> System<'s> for SpriteUpdateSystem {
    type SystemData = (WriteStorage<'s, SpriteRender>, ReadStorage<'s, Velocity>);

    fn run(&mut self, (mut sprites, velocities): Self::SystemData) {
        for (sprite, velocity) in (&mut sprites, &velocities).join() {
            let mut sprite_number;
            match velocity.0.x > 0.0 {
                true => {sprite_number = 1;},
                false => {sprite_number = 0;}
            };
            match velocity.0.y != 0.0 {
                true => {sprite_number += 2;},
                false => {}
            };
            sprite.sprite_number = sprite_number;
        }
    }
}
