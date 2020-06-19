use crate::components::game::{CameraTarget, CollisionEvent, Health, Invincibility, Player};
use crate::components::graphics::{AnimationCounter, CameraLimit, Lerper};
use crate::components::physics::{Collidee, GravityDirection, PlatformCuboid, Position, Velocity};
use crate::events::PlayerEvent;
use crate::states::pizzatopia::{TILE_HEIGHT, TILE_WIDTH};
use crate::systems::physics::{gravitationally_de_adapted_velocity, CollisionDirection};
use amethyst::core::math::Vector3;
use amethyst::core::shrev::{EventChannel, ReaderId};
use amethyst::core::timing::Time;
use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::{Entities, Join, Read, ReadStorage, System, SystemData, World, WriteStorage};
use amethyst::renderer::{
    Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture,
};
use log::{error, info, warn};
use std::cmp::{max, min};
use std::ops::Deref;

use amethyst::{
    assets::AssetStorage,
    audio::{output::Output, Source},
    ecs::ReadExpect,
};

use crate::audio::{play_damage_sound, Sounds};
use crate::components::ai::BasicWalkAi;
use crate::components::editor::{EditorCursor, EditorFlag};
use crate::utils::Vec3;

const WALK_SPEED: f32 = 4.0;

#[derive(SystemDesc)]
pub struct BasicWalkAiSystem;

impl<'s> System<'s> for BasicWalkAiSystem {
    type SystemData = (
        WriteStorage<'s, Velocity>,
        WriteStorage<'s, BasicWalkAi>,
        ReadStorage<'s, Collidee>,
    );

    fn run(&mut self, (mut velocities, mut ai, collidees): Self::SystemData) {
        for (velocity, ai, collidee) in (&mut velocities, &mut ai, &collidees).join() {
            if let Some(_col) = &collidee.horizontal {
                ai.going_right = !ai.going_right;
            }

            velocity.vel.x = WALK_SPEED;
            velocity.vel.x *= match ai.going_right {
                true => 1.,
                false => -1.,
            };
        }
    }
}
