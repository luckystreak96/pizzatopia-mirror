use crate::components::game::Health;
use crate::components::graphics::AnimationCounter;
use crate::components::physics::{GravityDirection, PlatformCuboid, Position, Velocity};
use crate::states::pizzatopia::{TILE_HEIGHT, TILE_WIDTH};
use crate::systems::physics::{gravitationally_de_adapted_velocity, CollisionDirection};
use amethyst::core::math::Vector3;
use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::{Join, Read, ReadStorage, System, SystemData, World, WriteStorage};
use amethyst::renderer::{
    Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture,
};

#[derive(SystemDesc)]
pub struct PositionDrawUpdateSystem;

impl<'s> System<'s> for PositionDrawUpdateSystem {
    type SystemData = (WriteStorage<'s, Transform>, ReadStorage<'s, Position>);

    fn run(&mut self, (mut transforms, positions): Self::SystemData) {
        for (transform, position) in (&mut transforms, &positions).join() {
            transform.set_translation_xyz(position.0.x, position.0.y, position.0.z);
        }
    }
}

#[derive(SystemDesc)]
pub struct DeadDrawUpdateSystem;

impl<'s> System<'s> for DeadDrawUpdateSystem {
    type SystemData = (WriteStorage<'s, Transform>, ReadStorage<'s, Health>);

    fn run(&mut self, (mut transforms, healths): Self::SystemData) {
        for (transform, health) in (&mut transforms, &healths).join() {
            if health.0 == 0 {
                transform.set_translation_xyz(-9999.0, -9999.0, 0.0);
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct SpriteUpdateSystem;

impl<'s> System<'s> for SpriteUpdateSystem {
    type SystemData = (
        WriteStorage<'s, Transform>,
        WriteStorage<'s, SpriteRender>,
        WriteStorage<'s, AnimationCounter>,
        ReadStorage<'s, Velocity>,
        ReadStorage<'s, GravityDirection>,
    );

    fn run(
        &mut self,
        (mut transforms, mut sprites, mut counters, velocities, gravities): Self::SystemData,
    ) {
        for (transform, sprite, counter, velocity, gravity) in (
            &mut transforms,
            &mut sprites,
            &mut counters,
            &velocities,
            (&gravities).maybe(),
        )
            .join()
        {
            let mut grav_dir = CollisionDirection::FromTop;
            if let Some(grav) = gravity {
                grav_dir = grav.0;
            }

            let grav_vel =
                gravitationally_de_adapted_velocity(&velocity.0, &GravityDirection(grav_dir));

            let mut sprite_number = sprite.sprite_number % 2;
            if grav_vel.x != 0.0 {
                counter.0 = counter.0 + grav_vel.x.abs() as u32;
                if counter.0 >= 100 {
                    sprite_number = (sprite_number + 1) % 2;
                    counter.0 = 0;
                }
                let mut cur_scale = transform.scale().clone();
                match grav_vel.x < 0.0 {
                    true => {
                        cur_scale.x = -1.0 * cur_scale.x.abs();
                        transform.set_scale(cur_scale);
                    }
                    false => {
                        cur_scale.x = cur_scale.x.abs();
                        transform.set_scale(cur_scale);
                    }
                };
            } else {
                sprite_number = 0;
            }
            match grav_vel.y != 0.0 {
                true => {
                    sprite_number += 2;
                }
                false => {}
            };
            sprite.sprite_number = sprite_number;

            // Set the rotation for sticky nerds
            match grav_dir {
                CollisionDirection::FromTop => {
                    transform.set_rotation_z_axis(0.0);
                }
                CollisionDirection::FromBottom => {
                    transform.set_rotation_z_axis(std::f32::consts::PI);
                }
                CollisionDirection::FromLeft => {
                    transform.set_rotation_z_axis(std::f32::consts::FRAC_PI_2);
                }
                CollisionDirection::FromRight => {
                    transform
                        .set_rotation_z_axis(std::f32::consts::PI + std::f32::consts::FRAC_PI_2);
                }
            }
        }
    }
}
