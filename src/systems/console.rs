use amethyst::{
    prelude::*,
    core::{
        bundle::SystemBundle,
        frame_limiter::FrameRateLimitStrategy,
        shrev::{EventChannel, ReaderId},
        SystemDesc,
    },
    derive::SystemDesc,
    ecs::{Component, DenseVecStorage, Write, Join, Read, ReadStorage, System, SystemData, World, WriteStorage},
    input::{InputHandler, StringBindings},
};

use crate::utils::{read_line_from_console};
use crate::components::physics::Grounded;
use crate::events::Events;

#[derive(SystemDesc)]
pub struct ConsoleInputSystem;

impl<'s> System<'s> for ConsoleInputSystem {
    type SystemData = (
        ReadStorage<'s, Grounded>,
        Read<'s, InputHandler<StringBindings>>,
        Write<'s, EventChannel<Events>>,
    );

    fn run(&mut self, (grounded, input, mut events_channel): Self::SystemData) {
        if input.action_is_down("console").unwrap_or(false) {
            let console_input = read_line_from_console();

            let mut args = Vec::new();
            for s in console_input.split(" ") {
                args.push(s);
            }

            if args.is_empty() {
                return;
            }

            match args[0] {
                "reset" => {
                    events_channel.single_write(Events::Reset);
                },
                _ => {}
            }
        }
    }
}
