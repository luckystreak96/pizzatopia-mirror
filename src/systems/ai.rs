use crate::{
    components::{
        game::{CameraTarget, CollisionEvent, Health, Invincibility, Player, Team},
        graphics::{AnimationCounter, CameraLimit},
        physics::{Collidee, GravityDirection, PlatformCuboid, Position, Velocity},
    },
    events::{Events, PlayerEvent},
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

use crate::components::ai::BasicAttackAi;
use crate::components::physics::{Grounded, MoveIntent, Orientation};
use crate::{
    audio::{play_damage_sound, Sounds},
    components::{
        ai::{BasicShootAi, BasicWalkAi},
        editor::{EditorCursor, EditorFlag},
    },
};
use std::ops::Mul;
use ultraviolet::{Lerp, Vec2};

const WALK_SPEED: f32 = 4.0;
const PROJECTILE_SPEED: f32 = 12.0;

#[derive(SystemDesc)]
pub struct BasicWalkAiSystem;

impl<'s> System<'s> for BasicWalkAiSystem {
    type SystemData = (
        WriteStorage<'s, Velocity>,
        WriteStorage<'s, Orientation>,
        WriteStorage<'s, MoveIntent>,
        WriteStorage<'s, BasicWalkAi>,
        ReadStorage<'s, Collidee>,
        ReadStorage<'s, Grounded>,
        Read<'s, Time>,
    );

    fn run(
        &mut self,
        (mut velocities, mut orientations, mut move_intents, mut ai, collidees, groundeds, time): Self::SystemData,
    ) {
        for (velocity, orientation, intent, ai, collidee, grounded) in (
            &mut velocities,
            &mut orientations,
            &mut move_intents,
            &mut ai,
            &collidees,
            (&groundeds).maybe(),
        )
            .join()
        {
            if collidee.horizontal.is_some() {
                ai.orientation.vec.x *= -1.0;
            }
            if collidee.vertical.is_some() {
                ai.orientation.vec.y *= -1.0;
            }

            orientation.vec = ai.orientation.vec;
            intent.vec = ai.orientation.vec;

            if let Some(grounded) = grounded {
                if grounded.0 {
                    let target = ai.orientation.vec.mul(WALK_SPEED);
                    let result = velocity
                        .0
                        .lerp(Vec2::new(target.x, 0.0), time.delta_seconds() * 4.0);
                    velocity.0.x = result.x;
                }
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct BasicShootAiSystem;

impl<'s> System<'s> for BasicShootAiSystem {
    type SystemData = (
        WriteStorage<'s, BasicShootAi>,
        ReadStorage<'s, Position>,
        ReadStorage<'s, Orientation>,
        ReadStorage<'s, Team>,
        Write<'s, EventChannel<Events>>,
        Read<'s, Time>,
    );

    fn run(
        &mut self,
        (mut shoot_ai, positions, orientations, teams, mut events_channel, time): Self::SystemData,
    ) {
        for (shoot, pos, orientation, team) in
            (&mut shoot_ai, &positions, &orientations, &teams).join()
        {
            shoot.counter += time.delta_seconds();

            let velocity = orientation.vec.mul(PROJECTILE_SPEED);
            if shoot.counter > 2.0 {
                shoot.counter = 0.0;

                let mut pos = pos.0;
                pos.y += match rand::random() {
                    true => TILE_HEIGHT / 4.0,
                    false => -TILE_HEIGHT / 4.,
                };
                events_channel.single_write(Events::FireProjectile(pos, velocity, team.clone()));
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct BasicAttackAiSystem;

impl<'s> System<'s> for BasicAttackAiSystem {
    type SystemData = (
        WriteStorage<'s, BasicAttackAi>,
        ReadStorage<'s, Team>,
        Write<'s, EventChannel<Events>>,
        Read<'s, Time>,
        Entities<'s>,
    );

    fn run(&mut self, (mut shoot_ai, teams, mut events_channel, time, entities): Self::SystemData) {
        for (shoot, team, entity) in (&mut shoot_ai, &teams, &entities).join() {
            shoot.counter += time.delta_seconds();

            if shoot.counter > 2.0 {
                shoot.counter = 0.0;

                let parent = Some(entity);
                let pos = Vec2::new(TILE_WIDTH / 1.5, TILE_HEIGHT / 4.);
                let size = Vec2::new(TILE_WIDTH, TILE_HEIGHT / 4.);
                events_channel.single_write(Events::CreateDamageBox(parent, pos, size, *team));
            }
        }
    }
}
