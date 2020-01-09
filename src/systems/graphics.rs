use crate::components::physics::{PlatformCuboid, Position, Velocity};
use crate::pizzatopia::{TILE_HEIGHT, TILE_WIDTH};
use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::{Join, Read, ReadStorage, System, SystemData, World, WriteStorage};
use amethyst::renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture};
use amethyst::core::math::Vector3;
use crate::components::graphics::AnimationCounter;

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
    type SystemData = (WriteStorage<'s, Transform>, WriteStorage<'s, SpriteRender>, WriteStorage<'s, AnimationCounter>, ReadStorage<'s, Velocity>);

    fn run(&mut self, (mut transforms, mut sprites, mut counters, velocities): Self::SystemData) {
        for (transform, sprite, counter, velocity) in (&mut transforms, &mut sprites, &mut counters, &velocities).join() {
            let mut sprite_number = sprite.sprite_number % 2;
            if velocity.0.x != 0.0 {
                counter.0 = counter.0 + velocity.0.x.abs() as u32;
                if counter.0 >= 100 {
                    sprite_number = (sprite_number + 1) % 2;
                    counter.0 = 0;
                }
                match velocity.0.x < 0.0 {
                    true => {transform.set_scale(Vector3::new(-1.0, 1.0, 1.0));},
                    false => {transform.set_scale(Vector3::new(1.0, 1.0, 1.0));}
                };
            } else {
                sprite_number = 0;
            }
            match velocity.0.y != 0.0 {
                true => {sprite_number += 2;},
                false => {}
            };
            sprite.sprite_number = sprite_number;
        }
    }
}
