use crate::components::game::{
    CameraTarget, CollisionEvent, Health, Invincibility, Player, Projectile, Team, TimedExistence,
};
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
use amethyst::ecs::{
    Entities, Join, LazyUpdate, Read, ReadStorage, System, SystemData, World, Write, WriteStorage,
};
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
use crate::components::editor::{EditorCursor, EditorFlag};
use crate::utils::{Vec2, Vec3};
use amethyst::prelude::WorldExt;

pub const IFRAMES_PER_HIT: f32 = 1.5;

#[derive(SystemDesc)]
#[system_desc(name(EnemyCollisionSystemDesc))]
pub struct EnemyCollisionSystem {
    #[system_desc(event_channel_reader)]
    reader: ReaderId<CollisionEvent>,
}

impl EnemyCollisionSystem {
    pub(crate) fn new(reader: ReaderId<CollisionEvent>) -> Self {
        Self { reader }
    }
}

impl<'s> System<'s> for EnemyCollisionSystem {
    type SystemData = (
        WriteStorage<'s, Health>,
        WriteStorage<'s, Invincibility>,
        WriteStorage<'s, Team>,
        WriteStorage<'s, Velocity>,
        WriteStorage<'s, Position>,
        Entities<'s>,
        Read<'s, EventChannel<CollisionEvent>>,
        Read<'s, AssetStorage<Source>>,
        ReadExpect<'s, Sounds>,
        Option<Read<'s, Output>>,
    );

    fn run(
        &mut self,
        (
            mut healths,
            mut invincibilities,
            mut teams,
            mut velocities,
            mut positions,
            entities,
            event_channel,
            storage,
            sounds,
            audio_output,
        ): Self::SystemData,
    ) {
        for event in event_channel.read(&mut self.reader) {
            match event {
                CollisionEvent::EnemyCollision(entity_id, damage) => {
                    if let Some(iframes) = &mut invincibilities.get_mut(entities.entity(*entity_id))
                    {
                        if let Some(health) = &mut healths.get_mut(entities.entity(*entity_id)) {
                            if health.0 > 0 && iframes.0 == 0.0 {
                                // Don't deal more damage than the character has hp
                                let dmg = min(*damage, health.0);
                                health.0 -= dmg;
                                iframes.0 += IFRAMES_PER_HIT;
                                play_damage_sound(
                                    &*sounds,
                                    &storage,
                                    audio_output.as_ref().map(|o| o.deref()),
                                );
                                warn!("Health is now {}", health.0);
                                if health.0 == 0 {
                                    let entity = entities.entity(*entity_id);
                                    if let Some(pos) = positions.get_mut(entity) {
                                        pos.0.y = -999.;
                                    }
                                }

                                if let Some(vel) = velocities.get_mut(entities.entity(*entity_id)) {
                                    let knock_back = Vec2::new(-8., 6.);
                                    vel.vel = vel.vel.add(&match vel.prev_going_right {
                                        true => knock_back,
                                        false => knock_back.mul(&Vec2::new(-1., 1.0)),
                                    });
                                }
                            }
                        }
                    }
                }
                CollisionEvent::ProjectileReflection(entity_id, team) => {
                    if let Some(team_comp) = teams.get_mut(entities.entity(*entity_id)) {
                        if let Some(vel) = velocities.get_mut(entities.entity(*entity_id)) {
                            *team_comp = *team;
                            vel.vel = vel.vel.mul_f32(-1.0);
                        }
                    }
                }
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct TimedExistenceSystem;

impl<'s> System<'s> for TimedExistenceSystem {
    type SystemData = (
        WriteStorage<'s, TimedExistence>,
        Entities<'s>,
        Read<'s, Time>,
    );

    fn run(&mut self, (mut times, entities, time): Self::SystemData) {
        for (timed, entity) in (&mut times, &entities).join() {
            timed.0 -= time.delta_seconds();
            if timed.0 <= 0.0 {
                if let Err(err) = entities.delete(entity) {
                    error!("Failed to delete TimedExistence entity - {}", err);
                }
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct AnimationCounterSystem;

impl<'s> System<'s> for AnimationCounterSystem {
    type SystemData = (
        WriteStorage<'s, AnimationCounter>,
        Read<'s, Time>,
        Read<'s, LazyUpdate>,
        Entities<'s>,
    );

    fn run(&mut self, (mut counters, time, lazy, entities): Self::SystemData) {
        for (mut counter, entity) in (&mut counters, &entities).join() {
            counter.count_down -= time.delta_seconds();
            if counter.count_down <= 0.0 {
                let clone = counter.clone();
                lazy.exec_mut(move |world| {
                    (clone.on_complete)(world);
                    world.write_storage::<AnimationCounter>().remove(entity);
                });
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct InvincibilitySystem;

impl<'s> System<'s> for InvincibilitySystem {
    type SystemData = (
        WriteStorage<'s, Invincibility>,
        Entities<'s>,
        Read<'s, Time>,
    );

    fn run(&mut self, (mut invincibilities, entities, time): Self::SystemData) {
        for (mut invinc, _entity) in (&mut invincibilities, &entities).join() {
            if invinc.0 > 0.0 {
                invinc.0 -= time.delta_seconds();
                invinc.0 = invinc.0.max(0.0);
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct CameraTargetSystem;

impl<'s> System<'s> for CameraTargetSystem {
    type SystemData = (
        WriteStorage<'s, Lerper>,
        ReadStorage<'s, Camera>,
        ReadStorage<'s, Position>,
        ReadStorage<'s, CameraTarget>,
        ReadStorage<'s, Player>,
        ReadStorage<'s, EditorCursor>,
        ReadStorage<'s, EditorFlag>,
    );

    fn run(
        &mut self,
        (mut lerpers, cameras, positions, targets, players, cursors, editor_flag): Self::SystemData,
    ) {
        let mut position = Vec3::default();
        for (_camera, target) in (&cameras, &targets).join() {
            match target {
                CameraTarget::Player => {
                    for (_player, player_pos, _) in (&players, &positions, !&editor_flag).join() {
                        position = player_pos.0.clone();
                    }
                }
                CameraTarget::Cursor => {
                    for (_cursor, cursor_pos) in (&cursors, &positions).join() {
                        position = cursor_pos.0.clone();
                    }
                }
                CameraTarget::GameObject(_) => {
                    error!("CameraTarget::GameObject(id) is not yet implemented!");
                }
            };
        }
        for (mut lerper, _camera) in (&mut lerpers, &cameras).join() {
            lerper.target.x = position.x;
            lerper.target.y = position.y;
        }
    }
}

#[derive(SystemDesc)]
pub struct ApplyProjectileCollisionSystem;

impl<'s> System<'s> for ApplyProjectileCollisionSystem {
    type SystemData = (
        ReadStorage<'s, Collidee>,
        ReadStorage<'s, Projectile>,
        Entities<'s>,
    );

    fn run(&mut self, (collidees, projectiles, entities): Self::SystemData) {
        for (collidee, _projectile, entity) in (&collidees, &projectiles, &entities).join() {
            if collidee.horizontal.is_some() || collidee.vertical.is_some() {
                let result = entities.delete(entity).is_ok();
                if !result {
                    error!("Failed to delete projectile entity");
                }
            }
        }
    }
}

#[derive(SystemDesc)]
#[system_desc(name(PlayerEventsSystemDesc))]
pub struct PlayerEventsSystem {
    #[system_desc(event_channel_reader)]
    reader: ReaderId<PlayerEvent>,
}

impl PlayerEventsSystem {
    pub(crate) fn new(reader: ReaderId<PlayerEvent>) -> Self {
        Self { reader }
    }
}

impl<'s> System<'s> for PlayerEventsSystem {
    type SystemData = (
        WriteStorage<'s, Health>,
        WriteStorage<'s, Invincibility>,
        Read<'s, EventChannel<PlayerEvent>>,
    );

    fn run(&mut self, (mut healths, mut invincibilities, event_channel): Self::SystemData) {
        for event in event_channel.read(&mut self.reader) {
            for (mut health, mut invincibility) in (&mut healths, &mut invincibilities).join() {
                match event {
                    PlayerEvent::Revive(new_health) => {
                        if health.0 == 0 {
                            health.0 = *new_health;
                            invincibility.0 = IFRAMES_PER_HIT;
                            info!("Player revived.");
                        }
                    }
                }
            }
        }
    }
}
