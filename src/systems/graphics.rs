use crate::animations::{AnimationAction, AnimationFactory, AnimationId, SamplerAction};
use crate::components::editor::{
    EditorCursor, EditorCursorState, EditorState, InsertionGameObject,
};
use crate::components::game::Health;
use crate::components::game::{SerializedObjectType, SpriteRenderData};
use crate::components::graphics::{
    AbsolutePositioning, AnimationCounter, CameraLimit, Lerper, PulseAnimation, Scale,
    SpriteSheetType,
};
use crate::components::physics::{
    GravityDirection, PlatformCollisionPoints, PlatformCuboid, Position, Velocity,
};
use crate::states::loading::DrawDebugLines;
use crate::states::pizzatopia::{DEPTH_UI, TILE_HEIGHT, TILE_WIDTH};
use crate::systems::physics::{gravitationally_de_adapted_velocity, CollisionDirection};
use crate::ui::tile_characteristics::{EditorFieldUiComponents, UiIndex};
use crate::ui::UiStack;
use amethyst::animation::*;
use amethyst::assets::{AssetStorage, Handle};
use amethyst::core::math::Vector3;
use amethyst::core::timing::Time;
use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::{
    Entities, Join, Read, ReadExpect, ReadStorage, System, SystemData, World, Write, WriteStorage,
};
use amethyst::renderer::debug_drawing::{DebugLines, DebugLinesComponent, DebugLinesParams};
use amethyst::renderer::{
    palette::Srgba, resources::Tint, Camera, ImageFormat, SpriteRender, SpriteSheet,
    SpriteSheetFormat, Texture,
};
use amethyst::ui::UiText;
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
        for (pos, _cam, limit) in (&mut positions, &cameras, &camera_limits).join() {
            pos.0.x = pos.0.x.clamp(limit.left, limit.right);
            pos.0.y = pos.0.y.clamp(limit.bottom, limit.top);
        }
    }
}

#[derive(SystemDesc)]
pub struct LerperSystem;

impl<'s> System<'s> for LerperSystem {
    type SystemData = (
        WriteStorage<'s, Lerper>,
        WriteStorage<'s, Position>,
        Read<'s, Time>,
    );

    fn run(&mut self, (mut lerpers, mut positions, time): Self::SystemData) {
        for (lerper, mut position) in (&mut lerpers, &mut positions).join() {
            let mut pos = lerper
                .lerp(position.0.to_vec2(), time.time_scale())
                .to_vec3();
            pos.z = position.0.z;
            position.0 = pos;
        }
    }
}

#[derive(SystemDesc)]
// This system is necessary only for use with animations.
// Animations set the trans/rot/scale, so we need to adapt after the fact.
// Cleanest way to do this : reset -> animate -> add your pos/rot/scale -> draw system will do the rest properly
pub struct TransformResetSystem;

impl<'s> System<'s> for TransformResetSystem {
    type SystemData = WriteStorage<'s, Transform>;

    fn run(&mut self, mut transforms: Self::SystemData) {
        for transform in (&mut transforms).join() {
            *transform = Transform::default();
        }
    }
}

#[derive(SystemDesc)]
pub struct PositionUpdateSystem;

impl<'s> System<'s> for PositionUpdateSystem {
    type SystemData = (WriteStorage<'s, Transform>, ReadStorage<'s, Position>);

    fn run(&mut self, (mut transforms, positions): Self::SystemData) {
        for (transform, position) in (&mut transforms, &positions).join() {
            transform.prepend_translation_x(position.0.x);
            transform.prepend_translation_y(position.0.y);
            transform.prepend_translation_z(position.0.z);
        }
    }
}

#[derive(SystemDesc)]
pub struct AbsolutePositionUpdateSystem;

impl<'s> System<'s> for AbsolutePositionUpdateSystem {
    type SystemData = (
        WriteStorage<'s, Transform>,
        ReadStorage<'s, Position>,
        ReadStorage<'s, AbsolutePositioning>,
    );

    fn run(&mut self, (mut transforms, positions, is_abs_pos): Self::SystemData) {
        for (transform, position, _abs) in (&mut transforms, &positions, &is_abs_pos).join() {
            transform.set_translation_x(position.0.x);
            transform.set_translation_y(position.0.y);
            transform.set_translation_z(position.0.z);
        }
    }
}

#[derive(SystemDesc)]
pub struct CollisionDebugLinesSystem;

impl<'s> System<'s> for CollisionDebugLinesSystem {
    type SystemData = (
        ReadStorage<'s, Position>,
        ReadStorage<'s, PlatformCuboid>,
        ReadStorage<'s, PlatformCollisionPoints>,
        Read<'s, DrawDebugLines>,
        Write<'s, DebugLines>,
    );

    fn run(
        &mut self,
        (positions, platform_cuboids, collision_points, draw, mut debug_lines): Self::SystemData,
    ) {
        if !draw.0 {
            return;
        }
        for (platform, position) in (&platform_cuboids, &positions).join() {
            debug_lines.draw_rectangle(
                [
                    position.0.x - platform.half_width,
                    position.0.y - platform.half_height,
                ]
                .into(),
                [
                    position.0.x + platform.half_width,
                    position.0.y + platform.half_height,
                ]
                .into(),
                DEPTH_UI,
                Srgba::new(1., 0., 0., 1.),
            );
        }

        for (col, position) in (&collision_points, &positions).join() {
            for point in &col.collision_points {
                let offset_x = match point.is_horizontal {
                    true => point.half_reach,
                    false => 0.,
                };
                let offset_y = match point.is_horizontal {
                    true => 0.,
                    false => point.half_reach,
                };
                debug_lines.draw_line(
                    [
                        position.0.x + point.point.x - offset_x,
                        position.0.y + point.point.y - offset_y,
                        DEPTH_UI,
                    ]
                    .into(),
                    [
                        position.0.x + point.point.x + offset_x,
                        position.0.y + point.point.y + offset_y,
                        DEPTH_UI,
                    ]
                    .into(),
                    Srgba::new(1., 0., 0., 1.),
                );
            }
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
        WriteStorage<'s, Tint>,
    );

    fn run(&mut self, (mut transforms, mut pulses, mut scales, mut tints): Self::SystemData) {
        for (transform, pulse, scale, tint) in
            (&mut transforms, &mut pulses, &mut scales, &mut tints).join()
        {
            pulse.0 += 1;
            // sin() swaps ever 3.14, so this will swap ~1x/sec
            let cf = pulse.0 as f32 / 20.0;
            // We want the amplitude to be 0.125
            let amplitude = cf.sin() / 100.0;
            // From 1.5 to 0.5
            let scale_sin = amplitude + 1.0;
            transform.set_scale(Vector3::new(
                scale_sin * scale.0.x,
                scale_sin * scale.0.y,
                1.0,
            ));

            let amplitude = cf.sin() / 2.0;
            let tint_sin = amplitude + 1.25;
            tint.0.red *= tint_sin;
            tint.0.green *= tint_sin;
            tint.0.blue *= tint_sin;
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
                    tint.0 = Srgba::new(1.0, 1.0, 1.0, 0.85).into();
                }
                EditorCursorState::Error => {
                    tint.0 = Srgba::new(1.0, 0.0, 0.0, 0.85).into();
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
        WriteStorage<'s, Scale>,
        ReadStorage<'s, Velocity>,
        ReadStorage<'s, GravityDirection>,
        ReadStorage<'s, AnimationSet<AnimationId, Transform>>,
        WriteStorage<'s, AnimationControlSet<AnimationId, Transform>>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (
            mut transforms,
            mut sprites,
            mut scales,
            velocities,
            gravities,
            sets,
            mut controls,
            entities,
        ): Self::SystemData,
    ) {
        for (transform, _sprite, scale, velocity, entity, gravity) in (
            &mut transforms,
            &mut sprites,
            &mut scales,
            &velocities,
            &entities,
            (&gravities).maybe(),
        )
            .join()
        {
            let mut grav_dir = CollisionDirection::FromTop;
            if let Some(grav) = gravity {
                grav_dir = grav.0;
            }

            let grav_vel =
                gravitationally_de_adapted_velocity(&velocity.vel, &GravityDirection(grav_dir));

            // let mut sprite_number = sprite.sprite_number % 2;
            if grav_vel.x != 0.0 {
                AnimationFactory::set_animation(
                    &sets,
                    &mut controls,
                    entity,
                    AnimationId::Animate,
                    AnimationAction::StartAnimationOrSetRate(grav_vel.x.abs()),
                    None,
                );

                let mut cur_scale = &mut scale.0;
                match grav_vel.x < 0.0 {
                    true => {
                        cur_scale.x = -1.0 * cur_scale.x.abs();
                    }
                    false => {
                        cur_scale.x = cur_scale.x.abs();
                    }
                };
            } else {
                AnimationFactory::set_animation(
                    &sets,
                    &mut controls,
                    entity,
                    AnimationId::Animate,
                    AnimationAction::AbortAnimation,
                    None,
                );
                // sprite_number = 0;
            }
            match grav_vel.y != 0.0 {
                true => {
                    // sprite_number += 2;
                    AnimationFactory::set_animation(
                        &sets,
                        &mut controls,
                        entity,
                        AnimationId::Animate,
                        AnimationAction::AbortAnimation,
                        None,
                    );
                }
                false => {}
            };
            // Uncomment me to regain spite animation
            // sprite.sprite_number = sprite_number;

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
