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
};

use crate::{
    events::{Events, PlayerEvent},
    utils::read_line_from_console,
};
use bami::Input;

#[derive(SystemDesc)]
pub struct ConsoleInputSystem;

impl<'s> System<'s> for ConsoleInputSystem {
    type SystemData = (
        Read<'s, Input<StringBindings>>,
        Write<'s, EventChannel<Events>>,
        Write<'s, EventChannel<PlayerEvent>>,
    );

    fn run(&mut self, (input, mut events_channel, mut player_event_channel): Self::SystemData) {
        let input_string;

        if input.actions.single_press("console".to_string()).is_down {
            input_string = read_line_from_console();
        } else if input.actions.single_press("reset".to_string()).is_down {
            input_string = String::from("reset");
        } else if input.actions.single_press("revive".to_string()).is_down {
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
            _ => {}
        }
    }
}
