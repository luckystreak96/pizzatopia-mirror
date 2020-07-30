use crate::{
    animations::{AnimationAction, AnimationFactory, AnimationId, SamplerAction},
    components::{
        editor::{CursorState, EditorCursor, EditorCursorState, InsertionGameObject},
        game::{Health, Player, SerializedObjectType, SpriteRenderData},
        graphics::{
            AnimationCounter, BackgroundParallax, CameraLimit, PulseAnimation, Scale,
            SpriteSheetType,
        },
        physics::{
            Ducking, GravityDirection, PlatformCollisionPoints, PlatformCuboid, Position, Velocity,
        },
    },
    states::{
        loading::DrawDebugLines,
        pizzatopia::{CAM_HEIGHT, CAM_WIDTH, DEPTH_UI, TILE_HEIGHT, TILE_WIDTH},
    },
    systems::physics::{gravitationally_de_adapted_velocity, CollisionDirection},
    ui::{
        tile_characteristics::{EditorFieldUiComponents, UiIndex},
        UiStack,
    },
};
use amethyst::{
    animation::*,
    assets::{AssetStorage, Handle},
    core::{math::Vector3, timing::Time, SystemDesc, Transform},
    derive::SystemDesc,
    ecs::{
        Entities, Join, Read, ReadExpect, ReadStorage, System, SystemData, World, Write,
        WriteStorage,
    },
    renderer::{
        debug_drawing::{DebugLines, DebugLinesComponent, DebugLinesParams},
        palette::Srgba,
        resources::Tint,
        Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture,
    },
    ui::UiText,
};

use crate::components::game::AnimatedTileComp;
use crate::components::graphics::Pan;
use crate::components::physics::Orientation;
use amethyst::ui::{ScaleMode, UiTransform};
use std::collections::BTreeMap;
use std::ops::Sub;
use ultraviolet::{Lerp, Vec2, Vec3};

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
            pos.0.x = pos.0.x.max(limit.left).min(limit.right);
            // pos.0.x = pos.0.x.clamp(limit.left, limit.right);
            pos.0.y = pos.0.y.max(limit.bottom);
            // pos.0.y = pos.0.y.clamp(limit.bottom, limit.top);
        }
    }
}

#[derive(SystemDesc)]
pub struct PanSystem;

impl<'s> System<'s> for PanSystem {
    type SystemData = (
        WriteStorage<'s, Position>,
        ReadStorage<'s, Pan>,
        Read<'s, Time>,
    );

    fn run(&mut self, (mut positions, pans, time): Self::SystemData) {
        for (mut position, pan) in (&mut positions, &pans).join() {
            position.0 = position
                .0
                .lerp(pan.destination, pan.speed_factor * time.delta_seconds());
        }
    }
}

#[derive(SystemDesc)]
pub struct TransformUpdateSystem;

impl<'s> System<'s> for TransformUpdateSystem {
    type SystemData = (
        WriteStorage<'s, Transform>,
        ReadStorage<'s, Position>,
        ReadStorage<'s, Scale>,
    );

    fn run(&mut self, (mut transforms, positions, scales): Self::SystemData) {
        for (transform, position, scale) in (&mut transforms, &positions, (&scales).maybe()).join()
        {
            transform.set_translation_x(position.0.x);
            transform.set_translation_y(position.0.y);
            if let Some(scale) = scale {
                transform.set_scale(Vector3::new(scale.0.x, scale.0.y, 1.0));
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct UiTransformUpdateSystem;

impl<'s> System<'s> for UiTransformUpdateSystem {
    type SystemData = (
        WriteStorage<'s, UiTransform>,
        ReadStorage<'s, Position>,
        ReadStorage<'s, Camera>,
    );

    fn run(&mut self, (mut transforms, positions, cameras): Self::SystemData) {
        let mut camera_pos = None;
        for (position, _camera) in (&positions, &cameras).join() {
            camera_pos = Some(position.0);
        }
        if let Some(cam_pos) = camera_pos {
            for (transform, position) in (&mut transforms, &positions).join() {
                transform.scale_mode = ScaleMode::Percent;
                let new_pos = position.0.sub(cam_pos);
                let new_pos_scaled = Vec2::new(new_pos.x / CAM_WIDTH, new_pos.y / CAM_HEIGHT);
                transform.local_x = new_pos_scaled.x;
                transform.local_y = new_pos_scaled.y;
            }
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
        ReadExpect<'s, CursorState>,
        ReadExpect<'s, BTreeMap<u8, Handle<SpriteSheet>>>,
        Read<'s, AssetStorage<SpriteSheet>>,
    );

    fn run(
        &mut self,
        (
            mut sprites,
            cursors,
            mut insertion_serialized_object,
            cursor_state,
            sprite_sheets,
            sheets,
        ): Self::SystemData,
    ) {
        for (sprite, _) in (&mut sprites, &cursors).join() {
            match *cursor_state {
                CursorState::InsertMode | CursorState::EditGameObject => {
                    if insertion_serialized_object.0.sprite.is_none() {
                        insertion_serialized_object.0.sprite =
                            Some(SpriteRenderData::new(SpriteSheetType::Tiles, 0));
                    }
                    if let Some(ref mut sprite_data) = insertion_serialized_object.0.sprite {
                        let sheet = sprite_sheets.get(&(sprite_data.sheet as u8)).unwrap();
                        sprite.sprite_sheet = sheet.clone();
                        if let Some(sheet) = sheets.get(sheet) {
                            sprite_data.number =
                                // sprite_data.number.clamp(0, sheet.sprites.len() - 1);
                            sprite_data.number.max(0).min(sheet.sprites.len() - 1);
                            // sprite.sprite_number = sprite_data.number.clamp(0, sprite_data.number);
                            sprite.sprite_number =
                                sprite_data.number.max(0).min(sprite_data.number);
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
pub struct AnimatedTileSystem;

impl<'s> System<'s> for AnimatedTileSystem {
    type SystemData = (
        WriteStorage<'s, SpriteRender>,
        WriteStorage<'s, AnimatedTileComp>,
        Read<'s, Time>,
    );

    fn run(&mut self, (mut sprites, mut anims, time): Self::SystemData) {
        for (sprite, anim) in (&mut sprites, &mut anims).join() {
            anim.counter += time.delta_seconds();
            if anim.counter > anim.anim.time_per_frame {
                anim.counter = 0.0;
                if anim.anim.num_frames > 0 {
                    sprite.sprite_number += 1;
                    if sprite.sprite_number > anim.base_sprite + anim.anim.num_frames {
                        sprite.sprite_number = anim.base_sprite;
                    }
                }
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct BackgroundDrawUpdateSystem;

impl<'s> System<'s> for BackgroundDrawUpdateSystem {
    type SystemData = (
        WriteStorage<'s, Position>,
        WriteStorage<'s, BackgroundParallax>,
        ReadStorage<'s, Camera>,
    );

    fn run(&mut self, (mut positions, mut bgs, cameras): Self::SystemData) {
        let mut translate = Vec2::new(0., 0.);
        for (position, _camera) in (&positions, &cameras).join() {
            translate = position.0;
        }
        for (position, bg) in (&mut positions, &mut bgs).join() {
            let id = bg.0;
            let ratio = 0.25 / (1. + id as f32);
            let calc_final_x = |offset_index: i32| -> f32 {
                return translate.x - (translate.x * ratio) + offset_index as f32 * CAM_WIDTH;
            };
            let mut final_x = calc_final_x(bg.1);
            let off_screen_left = final_x + CAM_WIDTH < translate.x;
            let off_screen_right = final_x - CAM_WIDTH > translate.x;
            if off_screen_left {
                bg.1 += 2;
            } else if off_screen_right {
                bg.1 -= 2;
            }
            final_x = calc_final_x(bg.1);
            position.0.x = final_x;
            position.0.y = translate.y;
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
        ReadStorage<'s, Orientation>,
        ReadStorage<'s, GravityDirection>,
        ReadStorage<'s, AnimationCounter>,
        ReadStorage<'s, Ducking>,
        ReadStorage<'s, Player>,
        ReadStorage<'s, AnimationSet<AnimationId, Transform>>,
        WriteStorage<'s, AnimationControlSet<AnimationId, Transform>>,
        ReadStorage<'s, AnimationSet<AnimationId, SpriteRender>>,
        WriteStorage<'s, AnimationControlSet<AnimationId, SpriteRender>>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (
            mut transforms,
            mut sprites,
            mut scales,
            velocities,
            orientations,
            gravities,
            anim_counters,
            duckings,
            players,
            sets,
            mut controls,
            sprite_sets,
            mut sprite_controls,
            entities,
        ): Self::SystemData,
    ) {
        for (sprite, _player, _anim, ducking) in (
            &mut sprites,
            &players,
            (&anim_counters).maybe(),
            (&duckings).maybe(),
        )
            .join()
        {
            if ducking.is_some() {
                sprite.sprite_number = 1;
            } else {
                sprite.sprite_number = 0;
            }
            // if let Some(anim) = anim {
            // if anim.animation_type == AnimationId::None {
            //     sprite.sprite_number += 2;
            // }
            // }
        }
        for (transform, _sprite, scale, velocity, orientation, entity, gravity) in (
            &mut transforms,
            &mut sprites,
            &mut scales,
            &velocities,
            &orientations,
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
                gravitationally_de_adapted_velocity(&velocity.0, &GravityDirection(grav_dir));

            // let mut sprite_number = sprite.sprite_number % 2;
            if grav_vel.x != 0.0 {
                // AnimationFactory::set_animation(
                //     &sets,
                //     &mut controls,
                //     entity,
                //     AnimationId::Animate,
                //     AnimationAction::StartAnimationOrSetRate(grav_vel.x.abs()),
                //     None,
                // );
                AnimationFactory::set_sprite_animation(
                    &sprite_sets,
                    &mut sprite_controls,
                    entity,
                    AnimationId::Animate,
                    AnimationAction::StartAnimationOrSetRate(grav_vel.x.abs()),
                    None,
                );

                let mut cur_scale = &mut scale.0;
                match orientation.vec.x > 0. {
                    // match grav_vel.x < 0.0 {
                    false => {
                        cur_scale.x = -1.0 * cur_scale.x.abs();
                    }
                    true => {
                        cur_scale.x = cur_scale.x.abs();
                    }
                };
            } else {
                AnimationFactory::set_sprite_animation(
                    &sprite_sets,
                    &mut sprite_controls,
                    entity,
                    AnimationId::Animate,
                    AnimationAction::AbortAnimation,
                    None,
                );
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
