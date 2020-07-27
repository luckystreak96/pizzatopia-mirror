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

use crate::components::physics::{MoveIntent, Orientation};
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
    systems::physics::{gravitationally_adapted_velocity, gravitationally_de_adapted_velocity},
};
use amethyst::prelude::WorldExt;
use bami::Input;
use log::error;
use num_traits::identities::Zero;
use std::sync::Arc;
use ultraviolet::Vec2;

#[derive(SystemDesc)]
pub struct PlayerInputSystem;

impl<'s> System<'s> for PlayerInputSystem {
    type SystemData = (
        WriteStorage<'s, Velocity>,
        WriteStorage<'s, Orientation>,
        WriteStorage<'s, MoveIntent>,
        WriteStorage<'s, AnimationCounter>,
        ReadStorage<'s, Position>,
        Read<'s, Input<StringBindings>>,
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
            mut orientations,
            mut move_intents,
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
        for (vel, intent, _pos, _player, health, ground, gravity, ducking, orientation, entity) in (
            &mut velocities,
            &mut move_intents,
            &positions,
            &players,
            &healths,
            (&grounded).maybe(),
            (&gravities).maybe(),
            (&duckings).maybe(),
            (&mut orientations).maybe(),
            &entities,
        )
            .join()
        {
            if health.0 == 0 {
                continue;
            }

            {
                // Controller input
                let h_move = input.axes.status("horizontal".to_string()).axis;
                let v_move = input.axes.status("vertical".to_string()).axis;

                intent.vec.x = h_move;
                intent.vec.y = v_move;
            }

            let jumping = input.actions.status("accept".to_string()).is_down;
            let release = input.actions.just_released("accept".to_string());
            let slowing = input.actions.status("insert".to_string()).is_down;
            let attacking = input.actions.single_press("attack".to_string()).is_down;

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
                            let size_st = world.read_storage::<Scale>();
                            let duck_st = world.read_storage::<Ducking>();
                            ducking = duck_st.contains(entity);
                            let size_opt = size_st.get(entity);
                            if size_opt.is_none() {
                                return;
                            }
                            size = size_opt.unwrap().0;
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

            if !intent.vec.x.is_zero() && orientation.is_some() {
                orientation.unwrap().vec.x = match intent.vec.x > 0. {
                    true => 1.0,
                    false => -1.0,
                };
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

            let mut grav_vel = vel.0.clone();
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
            let mut scaled_amount = movement * intent.vec.x as f32;
            if on_ground {
                let bonus = (grav_vel.x * 0.025).abs() * intent.vec.x;
                if ducking.is_some() {
                    scaled_amount -= bonus;
                } else {
                    scaled_amount += bonus;
                }
            }
            grav_vel.x += scaled_amount;

            if let Some(grav) = gravity {
                vel.0 = gravitationally_adapted_velocity(&grav_vel, &grav);
            }
        }
    }
}
