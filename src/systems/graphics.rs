use crate::components::editor::{
    EditorCursor, EditorCursorState, EditorState, InsertionGameObject,
};
use crate::components::game::Health;
use crate::components::game::{SerializedObjectType, SpriteRenderData};
use crate::components::graphics::{
    AnimationCounter, CameraLimit, Lerper, PulseAnimation, Scale, SpriteSheetType,
};
use crate::components::physics::{GravityDirection, PlatformCuboid, Position, Velocity};
use crate::states::pizzatopia::{TILE_HEIGHT, TILE_WIDTH};
use crate::systems::physics::{gravitationally_de_adapted_velocity, CollisionDirection};
use amethyst::assets::{AssetStorage, Handle};
use amethyst::core::math::Vector3;
use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::{
    Join, Read, ReadExpect, ReadStorage, System, SystemData, World, Write, WriteStorage,
};
use amethyst::renderer::{
    palette::Srgba, resources::Tint, Camera, ImageFormat, SpriteRender, SpriteSheet,
    SpriteSheetFormat, Texture,
};
use log::info;
use std::collections::BTreeMap;

#[derive(SystemDesc)]
pub struct CameraEdgeClampSystem;

impl<'s> System<'s> for CameraEdgeClampSystem {
    type SystemData = (
        WriteStorage<'s, Position>,
        ReadStorage<'s, Camera>,
        ReadStorage<'s, CameraLimit>,
    );

    fn run(&mut self, (mut positions, cameras, camera_limits): Self::SystemData) {
        for (pos, cam, limit) in (&mut positions, &cameras, &camera_limits).join() {
            pos.0.x = pos.0.x.clamp(limit.left, limit.right);
            pos.0.y = pos.0.y.clamp(limit.bottom, limit.top);
        }
    }
}

#[derive(SystemDesc)]
pub struct LerperSystem;

impl<'s> System<'s> for LerperSystem {
    type SystemData = (WriteStorage<'s, Lerper>, WriteStorage<'s, Position>);

    fn run(&mut self, (mut lerpers, mut positions): Self::SystemData) {
        for (lerper, mut position) in (&mut lerpers, &mut positions).join() {
            let mut pos = lerper.lerp(position.0.to_vec2()).to_vec3();
            pos.z = position.0.z;
            position.0 = pos;
        }
    }
}

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
pub struct ScaleDrawUpdateSystem;

impl<'s> System<'s> for ScaleDrawUpdateSystem {
    type SystemData = (WriteStorage<'s, Transform>, ReadStorage<'s, Scale>);

    fn run(&mut self, (mut transforms, scales): Self::SystemData) {
        for (transform, scale) in (&mut transforms, &scales).join() {
            transform.set_scale(Vector3::new(scale.0.x, scale.0.y, 1.0));
        }
    }
}

#[derive(SystemDesc)]
pub struct PulseAnimationSystem;

impl<'s> System<'s> for PulseAnimationSystem {
    type SystemData = (
        WriteStorage<'s, Transform>,
        WriteStorage<'s, PulseAnimation>,
        WriteStorage<'s, Scale>,
    );

    fn run(&mut self, (mut transforms, mut pulses, mut scales): Self::SystemData) {
        for (transform, pulse, scale) in (&mut transforms, &mut pulses, &mut scales).join() {
            pulse.0 += 1;
            // sin() swaps ever 3.14, so this will swap ~1x/sec
            let cf = pulse.0 as f32 / 20.0;
            // We want the amplitude to be 0.125
            let sin = cf.sin() / 8.0;
            // From 1.5 to 0.5
            let sin = sin + 1.0;
            transform.set_scale(Vector3::new(sin * scale.0.x, sin * scale.0.y, 1.0));
        }
    }
}

#[derive(SystemDesc)]
pub struct CursorColorUpdateSystem;

impl<'s> System<'s> for CursorColorUpdateSystem {
    type SystemData = (WriteStorage<'s, Tint>, ReadStorage<'s, EditorCursor>);

    fn run(&mut self, (mut tint, cursors): Self::SystemData) {
        for (tint, cursor) in (&mut tint, &cursors).join() {
            match cursor.state {
                EditorCursorState::Normal => {
                    tint.0 = Srgba::new(1.0, 1.0, 1.0, 0.95).into();
                }
                EditorCursorState::Error => {
                    tint.0 = Srgba::new(1.0, 0.0, 0.0, 0.95).into();
                }
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct CursorSpriteUpdateSystem;

impl<'s> System<'s> for CursorSpriteUpdateSystem {
    type SystemData = (
        WriteStorage<'s, SpriteRender>,
        ReadStorage<'s, EditorCursor>,
        Write<'s, InsertionGameObject>,
        ReadExpect<'s, EditorState>,
        ReadExpect<'s, BTreeMap<u8, Handle<SpriteSheet>>>,
        Read<'s, AssetStorage<SpriteSheet>>,
    );

    fn run(
        &mut self,
        (
            mut sprites,
            cursors,
            mut insertion_serialized_object,
            editor_state,
            sprite_sheets,
            sheets,
        ): Self::SystemData,
    ) {
        for (sprite, _) in (&mut sprites, &cursors).join() {
            match *editor_state {
                EditorState::InsertMode | EditorState::EditGameObject => {
                    if insertion_serialized_object.0.sprite.is_none() {
                        insertion_serialized_object.0.sprite =
                            Some(SpriteRenderData::new(SpriteSheetType::Tiles, 0));
                    }
                    if let Some(ref mut sprite_data) = insertion_serialized_object.0.sprite {
                        let sheet = sprite_sheets.get(&(sprite_data.sheet as u8)).unwrap();
                        sprite.sprite_sheet = sheet.clone();
                        if let Some(sheet) = sheets.get(sheet) {
                            sprite_data.number =
                                sprite_data.number.clamp(0, sheet.sprites.len() - 1);
                            sprite.sprite_number = sprite_data.number.clamp(0, sprite_data.number);
                        }
                    }
                }
                _ => {
                    // Cursor sprite
                    sprite.sprite_sheet = sprite_sheets
                        .get(&(SpriteSheetType::Tiles as u8))
                        .unwrap()
                        .clone();
                    sprite.sprite_number = 4;
                }
            }
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
