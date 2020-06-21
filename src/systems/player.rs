use amethyst::core::shrev::EventChannel;
use amethyst::core::timing::Time;
use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::Write;
use amethyst::ecs::{
    Join, NullStorage, Read, ReadStorage, System, SystemData, World, WriteStorage,
};
use amethyst::input::{InputHandler, StringBindings};

use crate::components::game::Player;
use crate::components::game::{Health, Team};
use crate::components::physics::{GravityDirection, Grounded, PlatformCuboid, Position, Velocity};
use crate::events::Events;
use crate::states::pizzatopia::{CAM_HEIGHT, TILE_HEIGHT, TILE_WIDTH};
use crate::systems::input::InputManager;
use crate::systems::physics::{
    gravitationally_adapted_velocity, gravitationally_de_adapted_velocity,
};
use crate::utils::Vec2;

#[derive(SystemDesc)]
pub struct PlayerInputSystem;

impl<'s> System<'s> for PlayerInputSystem {
    type SystemData = (
        WriteStorage<'s, Velocity>,
        ReadStorage<'s, Position>,
        Read<'s, InputManager>,
        ReadStorage<'s, Player>,
        ReadStorage<'s, Health>,
        ReadStorage<'s, Grounded>,
        ReadStorage<'s, GravityDirection>,
        Write<'s, Time>,
        Write<'s, EventChannel<Events>>,
    );

    fn run(
        &mut self,
        (
            mut velocities,
            positions,
            input,
            players,
            healths,
            grounded,
            gravities,
            mut time,
            mut event_channel,
        ): Self::SystemData,
    ) {
        for (velocity, position, _player, health, ground, gravity) in (
            &mut velocities,
            &positions,
            &players,
            &healths,
            (&grounded).maybe(),
            (&gravities).maybe(),
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
                let width = TILE_WIDTH;
                let offset = match velocity.prev_going_right {
                    true => width / 2.0,
                    false => -width / 2.0,
                };
                let offset = Vec2::new(offset, 0.);
                let size = Vec2::new(width, TILE_HEIGHT / 2.0);
                event_channel.single_write(Events::CreateDamageBox(
                    position.0.to_vec2().add(&offset),
                    size,
                    Team::GoodGuys,
                ));
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

            let mut grav_vel = velocity.vel.clone();
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
                velocity.vel = gravitationally_adapted_velocity(&grav_vel, &grav);
            }
        }
    }
}
