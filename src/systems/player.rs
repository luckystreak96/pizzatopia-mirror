use amethyst::{
    animation::*,
    core::{shrev::EventChannel, timing::Time, SystemDesc, Transform},
    derive::SystemDesc,
    ecs::{
        Entities, Join, NullStorage, Read, ReadStorage, System, SystemData, World, Write,
        WriteStorage,
    },
    input::{InputHandler, StringBindings},
};

use crate::{
    animations::{AnimationAction, AnimationFactory, AnimationId},
    components::{
        entity_builder::entity_builder,
        game::{Health, Player, Team},
        graphics::{AnimationCounter, Scale},
        physics::{Ducking, GravityDirection, Grounded, PlatformCuboid, Position, Velocity},
    },
    events::Events,
    level::Level,
    states::pizzatopia::{CAM_HEIGHT, TILE_HEIGHT, TILE_WIDTH},
    systems::{
        input::InputManager,
        physics::{gravitationally_adapted_velocity, gravitationally_de_adapted_velocity},
    },
    utils::Vec2,
};
use amethyst::prelude::WorldExt;
use log::error;
use num_traits::identities::Zero;
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
        ReadStorage<'s, Ducking>,
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
            duckings,
            mut time,
            entities,
            sets,
            mut controls,
        ): Self::SystemData,
    ) {
        for (vel, _pos, _player, health, ground, gravity, ducking, entity) in (
            &mut velocities,
            &positions,
            &players,
            &healths,
            (&grounded).maybe(),
            (&gravities).maybe(),
            (&duckings).maybe(),
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
                    0.15,
                    AnimationId::None,
                    Arc::new(move |world| {
                        // let pos: Position;
                        // let vel: Velocity;
                        let size: Vec2;
                        let ducking: bool;
                        {
                            let pos_st = world.read_storage::<Position>();
                            let vel_st = world.read_storage::<Velocity>();
                            let size_st = world.read_storage::<Scale>();
                            let duck_st = world.read_storage::<Ducking>();
                            ducking = duck_st.contains(entity);
                            let pos_opt = pos_st.get(entity);
                            let vel_opt = vel_st.get(entity);
                            let size_opt = size_st.get(entity);
                            if pos_opt.is_none() || vel_opt.is_none() || size_opt.is_none() {
                                return;
                            }
                            // pos = pos_opt.unwrap().clone();
                            // vel = vel_opt.unwrap().clone();
                            size = size_opt.unwrap().0.clone();
                        }
                        let width = size.x.abs() * TILE_WIDTH;
                        let height = size.y.abs() * TILE_HEIGHT;
                        let offset_x = width / 2.;
                        let offset_y = height
                            * 0.25
                            * match ducking {
                                true => -1.,
                                false => 1.0,
                            };
                        // Multiply height by one quarter to go from half height to upper body
                        let offset = Vec2::new(offset_x, offset_y);
                        // let p = pos.0.to_vec2().add(&offset);
                        let p = offset;
                        let s = Vec2::new(width, TILE_HEIGHT / 4.0);
                        let t = Team::GoodGuys;
                        entity_builder::initialize_damage_box(
                            world,
                            Some(entity),
                            &p.clone(),
                            &s.clone(),
                            &t.clone(),
                        );
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

            if !h_move.is_zero() {
                vel.prev_going_right = h_move > 0.;
            }

            if slowing {
                time.set_time_scale(0.5);
            } else {
                time.set_time_scale(1.);
            }

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

            let mut movement = 0.30;
            if ducking.is_some() {
                movement *= 0.5;
            }
            let mut scaled_amount = movement * h_move as f32;
            if on_ground {
                let bonus = (grav_vel.x * 0.025).abs() * h_move;
                if ducking.is_some() {
                    scaled_amount -= bonus;
                } else {
                    scaled_amount += bonus;
                }
            }
            grav_vel.x += scaled_amount;

            if let Some(grav) = gravity {
                vel.vel = gravitationally_adapted_velocity(&grav_vel, &grav);
            }
        }
    }
}
