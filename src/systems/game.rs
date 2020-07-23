use crate::{
    components::{
        game::{
            CameraTarget, CollisionEvent, Health, Invincibility, Player, Projectile, Team,
            TimedExistence,
        },
        graphics::{AnimationCounter, CameraLimit, Lerper},
        physics::{Collidee, GravityDirection, PlatformCuboid, Position, Velocity},
    },
    events::PlayerEvent,
    states::pizzatopia::{TILE_HEIGHT, TILE_WIDTH},
    systems::physics::{gravitationally_de_adapted_velocity, CollisionDirection},
};
use amethyst::{
    core::{
        math::Vector3,
        shrev::{EventChannel, ReaderId},
        timing::Time,
        SystemDesc, Transform,
    },
    derive::SystemDesc,
    ecs::{
        Entities, Join, LazyUpdate, Read, ReadStorage, System, SystemData, World, Write,
        WriteStorage,
    },
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
};
use log::{error, info, warn};
use std::{cmp::min, ops::Deref};

use amethyst::{
    assets::AssetStorage,
    audio::{output::Output, Source},
    ecs::ReadExpect,
};

use crate::components::entity_builder::entity_builder::initialize_pickup;
use crate::components::game::{Drops, PicksThingsUp};
use crate::events::Events;
use crate::{
    audio::{play_damage_sound, Sounds},
    components::editor::{EditorCursor, EditorFlag},
    utils::{Vec2, Vec3},
};
use amethyst::prelude::WorldExt;
use rand::{random, Rng};

pub const IFRAMES_PER_HIT: f32 = 1.;

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
        WriteStorage<'s, PicksThingsUp>,
        Entities<'s>,
        Read<'s, EventChannel<CollisionEvent>>,
        Read<'s, LazyUpdate>,
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
            mut pickers,
            entities,
            event_channel,
            lazy,
            storage,
            sounds,
            audio_output,
        ): Self::SystemData,
    ) {
        for event in event_channel.read(&mut self.reader) {
            match event {
                CollisionEvent::EnemyCollision(entity_id, hitter, damage) => {
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

                                if dmg > 0 {
                                    let entity = entities.entity(*entity_id);
                                    if let Some(pos) = positions.get(entity) {
                                        let pos_clone = pos.clone();
                                        lazy.exec_mut(move |world| {
                                            let player = {
                                                let storage = world.read_storage::<Player>();
                                                let player =
                                                    storage.get(entity).unwrap_or(&Player(false));
                                                player.0
                                            };
                                            if player {
                                                let drops = {
                                                    let mut storage =
                                                        world.write_storage::<PicksThingsUp>();
                                                    let mut def = PicksThingsUp::default();
                                                    let picked_up =
                                                        storage.get_mut(entity).unwrap_or(&mut def);
                                                    let total_dropped;
                                                    if picked_up.amount_gathered <= 2 {
                                                        total_dropped = picked_up.amount_gathered;
                                                        picked_up.amount_gathered = 0;
                                                    } else {
                                                        total_dropped = 2;
                                                        picked_up.amount_gathered -= 2;
                                                    }
                                                    total_dropped
                                                };
                                                for _i in 0..drops {
                                                    let mut rng = rand::thread_rng();
                                                    let x_vel: f32 = rng.gen_range(-4.0, 4.0);
                                                    let y_vel: f32 = rng.gen_range(7.0, 15.0);
                                                    initialize_pickup(
                                                        world,
                                                        &pos_clone
                                                            .0
                                                            .to_vec2()
                                                            .add(&Vec2::new(0.0, TILE_HEIGHT)),
                                                        &Vec2::new(x_vel, y_vel),
                                                    );
                                                }
                                            }
                                        });
                                    }
                                }

                                warn!("Health is now {}", health.0);
                                if health.0 == 0 {
                                    let entity = entities.entity(*entity_id);
                                    if let Some(pos) = positions.get_mut(entity) {
                                        let pos_clone = pos.clone();
                                        lazy.exec_mut(move |world| {
                                            let drops = {
                                                let drops = world.read_storage::<Drops>();
                                                let amount = drops.get(entity).unwrap_or(&Drops(0));
                                                amount.0
                                            };
                                            for _i in 0..drops {
                                                let mut rng = rand::thread_rng();
                                                let x_vel: f32 = rng.gen_range(-2.0, 2.0);
                                                let y_vel: f32 = rng.gen_range(7.0, 15.0);
                                                initialize_pickup(
                                                    world,
                                                    &pos_clone.0.to_vec2(),
                                                    &Vec2::new(x_vel, y_vel),
                                                );
                                            }
                                        });
                                        pos.0.y = -999.;
                                    }
                                }

                                let hitter = entities.entity(*hitter);
                                let hitee = entities.entity(*entity_id);
                                if let Some(vel) = velocities.get_mut(hitee) {
                                    let hitter_pos = positions.get(hitter).unwrap().clone();
                                    let hitee_pos = positions.get(hitee).unwrap().clone();
                                    let going_right = hitter_pos.0.x < hitee_pos.0.x;
                                    let knock_back = Vec2::new(8., 6.);
                                    vel.vel = vel.vel.add(&match going_right {
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
                CollisionEvent::ProjectileBlock(entity_id) => {
                    entities
                        .delete(entities.entity(*entity_id))
                        .expect("Failed to delete blocked projectile.");
                }
                CollisionEvent::ItemCollect(character_id, item_id) => {
                    let picker = pickers.get_mut(entities.entity(*character_id)).unwrap();
                    picker.amount_gathered += 1;
                    info!("Picked up item! New amount: {:?}", picker.amount_gathered);
                    entities
                        .delete(entities.entity(*item_id))
                        .expect("Failed to delete pickup");
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
