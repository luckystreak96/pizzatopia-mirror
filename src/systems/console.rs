use amethyst::{
    core::{
        bundle::SystemBundle,
        frame_limiter::FrameRateLimitStrategy,
        shrev::{EventChannel, ReaderId},
        SystemDesc,
    },
    derive::SystemDesc,
    ecs::{
        Component, DenseVecStorage, Join, Read, ReadStorage, System, SystemData, World, Write,
        WriteStorage,
    },
    input::{InputHandler, StringBindings},
    prelude::*,
};

use crate::components::physics::Grounded;
use crate::events::{Events, PlayerEvent};
use crate::systems::input::InputManager;
use crate::utils::read_line_from_console;

#[derive(SystemDesc)]
pub struct ConsoleInputSystem;

impl<'s> System<'s> for ConsoleInputSystem {
    type SystemData = (
        Read<'s, InputManager>,
        Write<'s, EventChannel<Events>>,
        Write<'s, EventChannel<PlayerEvent>>,
    );

    fn run(&mut self, (input, mut events_channel, mut player_event_channel): Self::SystemData) {
        let input_string;

        if input.action_single_press("console").is_down {
            input_string = read_line_from_console();
        } else if input.action_single_press("reset").is_down {
            input_string = String::from("reset");
        } else if input.action_single_press("start").is_down {
            input_string = String::from("filepicker");
        } else if input.action_single_press("revive").is_down {
            input_string = String::from("revive");
        } else {
            input_string = String::new();
        }

        let args: Vec<_> = input_string.split_whitespace().collect();
        if args.is_empty() {
            return;
        }

        match args[0] {
            "reset" => {
                events_channel.single_write(Events::Reset);
            }
            "revive" => {
                player_event_channel.single_write(PlayerEvent::Revive(5));
            }
            "filepicker" => {
                events_channel.single_write(Events::OpenFilePickerUi);
            }
            _ => {}
        }
    }
}
