use amethyst::animation::*;
use amethyst::core::shrev::EventChannel;
use amethyst::core::timing::Time;
use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::Write;
use amethyst::ecs::{
    Entities, Join, NullStorage, Read, ReadStorage, System, SystemData, World, WriteStorage,
};
use amethyst::input::{InputHandler, StringBindings};

use crate::animations::{AnimationAction, AnimationFactory, AnimationId};
use crate::components::game::Player;
use crate::components::game::{Health, Team};
use crate::components::graphics::AnimationCounter;
use crate::components::physics::{GravityDirection, Grounded, PlatformCuboid, Position, Velocity};
use crate::events::Events;
use crate::level::Level;
use crate::states::pizzatopia::{CAM_HEIGHT, TILE_HEIGHT, TILE_WIDTH};
use crate::systems::input::InputManager;
use crate::systems::physics::{
    gravitationally_adapted_velocity, gravitationally_de_adapted_velocity,
};
use crate::utils::Vec2;
use amethyst::prelude::WorldExt;
use std::sync::Arc;

#[derive(SystemDesc)]
pub struct PlayerInputSystem;

impl<'s> System<'s> for PlayerInputSystem {
    type SystemData = (
        WriteStorage<'s, Velocity>,
        WriteStorage<'s, AnimationCounter>,
        ReadStorage<'s, Position>,
        Read<'s, InputManager>,
        ReadStorage<'s, Player>,
        ReadStorage<'s, Health>,
        ReadStorage<'s, Grounded>,
        ReadStorage<'s, GravityDirection>,
        Write<'s, Time>,
        Entities<'s>,
        ReadStorage<'s, AnimationSet<AnimationId, Transform>>,
        WriteStorage<'s, AnimationControlSet<AnimationId, Transform>>,
    );

    fn run(
        &mut self,
        (
            mut velocities,
            mut anim,
            positions,
            input,
            players,
            healths,
            grounded,
            gravities,
            mut time,
            entities,
            sets,
            mut controls,
        ): Self::SystemData,
    ) {
        for (vel, _pos, _player, health, ground, gravity, entity) in (
            &mut velocities,
            &positions,
            &players,
            &healths,
            (&grounded).maybe(),
            (&gravities).maybe(),
            &entities,
        )
            .join()
        {
            if health.0 == 0 {
                continue;
            }
            // Controller input
            let h_move = input.action_status("horizontal").axis;
            let jumping = input.action_status("accept").is_down;
            let release = input.action_just_released("accept");
            let slowing = input.action_status("insert").is_down;
            let attacking = input.action_single_press("attack").is_down;

            if attacking {
                let animation = AnimationCounter::new(
                    0.2,
                    AnimationId::None,
                    Arc::new(move |world| {
                        let pos: Position;
                        let vel: Velocity;
                        {
                            let pos_st = world.read_storage::<Position>();
                            let vel_st = world.read_storage::<Velocity>();
                            let pos_opt = pos_st.get(entity);
                            let vel_opt = vel_st.get(entity);
                            if pos_opt.is_none() || vel_opt.is_none() {
                                return;
                            }
                            pos = pos_opt.unwrap().clone();
                            vel = vel_opt.unwrap().clone();
                        }
                        let width = TILE_WIDTH;
                        let offset = match vel.prev_going_right {
                            true => width / 2.0,
                            false => -width / 2.0,
                        };
                        let offset = Vec2::new(offset, 0.);
                        let p = pos.0.to_vec2().add(&offset);
                        let s = Vec2::new(width, TILE_HEIGHT / 2.0);
                        let t = Team::GoodGuys;
                        Level::initialize_damage_box(world, &p.clone(), &s.clone(), &t.clone());
                    }),
                );
                if !anim.contains(entity) {
                    anim.insert(entity, animation)
                        .expect("Failed to insert AnimationCounter for attack");

                    AnimationFactory::set_animation(
                        &sets,
                        &mut controls,
                        entity,
                        AnimationId::Rotate,
                        AnimationAction::StartAnimationOrSetRate(1.0),
                        None,
                    );
                }
            }

            if slowing {
                time.set_time_scale(0.5);
            } else {
                time.set_time_scale(1.);
            }

            // Get the grounded status to use auto-complete :)
            let ground: Option<&Grounded> = ground;
            let default_ground = Grounded(false);
            let on_ground = ground.unwrap_or(&default_ground);
            let on_ground = on_ground.0;

            let mut grav_vel = vel.vel.clone();
            if let Some(grav) = gravity {
                grav_vel = gravitationally_de_adapted_velocity(&grav_vel, &grav);
            }

            // Do the move logic
            if jumping {
                if on_ground {
                    let jump_velocity = 13.0;
                    grav_vel.y += jump_velocity;
                }
            }
            // letting go of `up` will stop your jump
            // TODO : Make jumping constantly give you up velocity and stop when you release
            if grav_vel.y > 0.0 && release {
                grav_vel.y *= 0.5;
            }

            let mut scaled_amount = 0.30 * h_move as f32;
            if on_ground {
                scaled_amount += (grav_vel.x * 0.025).abs() * h_move;
            }
            grav_vel.x += scaled_amount;

            if let Some(grav) = gravity {
                vel.vel = gravitationally_adapted_velocity(&grav_vel, &grav);
            }
        }
    }
}
