use crate::{
    components::{
        game::{CameraTarget, CollisionEvent, Health, Invincibility, Player, Team},
        graphics::{AnimationCounter, CameraLimit, Lerper},
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
use crate::components::physics::Grounded;
use crate::{
    audio::{play_damage_sound, Sounds},
    components::{
        ai::{BasicShootAi, BasicWalkAi},
        editor::{EditorCursor, EditorFlag},
    },
    utils::{Vec2, Vec3},
};

const WALK_SPEED: f32 = 4.0;
const PROJECTILE_SPEED: f32 = 12.0;

#[derive(SystemDesc)]
pub struct BasicWalkAiSystem;

impl BasicWalkAiSystem {
    fn lerper(&self, target: f32) -> Lerper {
        let mut result = Lerper::default();
        result.target = Vec2::new(target, 0.0);
        result.amount = 0.5;
        result.epsilon = 0.05;
        result
    }
}

impl<'s> System<'s> for BasicWalkAiSystem {
    type SystemData = (
        WriteStorage<'s, Velocity>,
        WriteStorage<'s, BasicWalkAi>,
        ReadStorage<'s, Collidee>,
        ReadStorage<'s, Grounded>,
        Read<'s, Time>,
    );

    fn run(&mut self, (mut velocities, mut ai, collidees, groundeds, time): Self::SystemData) {
        for (velocity, ai, collidee, grounded) in
            (&mut velocities, &mut ai, &collidees, (&groundeds).maybe()).join()
        {
            if let Some(_col) = &collidee.horizontal {
                ai.going_right = !ai.going_right;
            }

            velocity.prev_going_right = ai.going_right;

            if let Some(grounded) = grounded {
                if grounded.0 {
                    let target = WALK_SPEED
                        * match ai.going_right {
                            true => 1.,
                            false => -1.,
                        };
                    let result = self
                        .lerper(target)
                        .linear_lerp(velocity.vel, time.time_scale());
                    velocity.vel.x = result.x;
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
        ReadStorage<'s, Velocity>,
        ReadStorage<'s, Team>,
        Write<'s, EventChannel<Events>>,
        Read<'s, Time>,
    );

    fn run(
        &mut self,
        (mut shoot_ai, positions, velocities, teams, mut events_channel, time): Self::SystemData,
    ) {
        for (shoot, pos, vel, team) in (&mut shoot_ai, &positions, &velocities, &teams).join() {
            shoot.counter += time.delta_seconds();

            let mut velocity = vel.vel.clone();
            velocity.y = 0.0;
            velocity.x = PROJECTILE_SPEED
                * match vel.prev_going_right {
                    true => 1.0,
                    false => -1.0,
                };
            if shoot.counter > 2.0 {
                shoot.counter = 0.0;

                let mut pos = pos.0.to_vec2();
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
