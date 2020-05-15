use amethyst::derive::SystemDesc;
use amethyst::ecs::{
    Join, NullStorage, Read, ReadStorage, System, SystemData, World, Write, WriteStorage,
};
use amethyst::input::{InputHandler, StringBindings};
use amethyst::prelude::WorldExt;
use derivative::Derivative;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

#[derive(Derivative)]
#[derivative(Default)]
struct InputStatistics {
    #[derivative(Default(value = "Instant::now()"))]
    time_since_last_press_millis: Instant,
    #[derivative(Default(value = "Instant::now()"))]
    press_length_millis: Instant,
    #[derivative(Default(value = "false"))]
    action_is_down: bool,
    action_down_frame_count: u32,
    action_axis_value: f32,
}

#[derive(Default)]
pub struct InputManager {
    statistics: BTreeMap<String, InputStatistics>,
    pub frame_counter: u128,
}

impl InputManager {
    pub fn new(world: &World) -> Self {
        let mut result = InputManager {
            statistics: BTreeMap::new(),
            frame_counter: 0,
        };

        let input = world.read_resource::<InputHandler<StringBindings>>();
        for action in input.bindings.actions().chain(input.bindings.axes()) {
            result
                .statistics
                .insert(action.clone(), InputStatistics::default());
        }

        result
    }

    pub fn is_valid_repeat_press(
        &self,
        action: &str,
        repeat_delay: u128,
        repeat_every_x_frames: u128,
    ) -> bool {
        if let Some(input) = self.statistics.get(action) {
            if !input.action_is_down {
                return false;
            }
            // 2 cases: the button was JUST pressed (elapsed = 0), or enough time passed
            let elapsed = input.press_length_millis.elapsed().as_millis();
            if input.action_down_frame_count == 1
                || (elapsed >= repeat_delay && self.frame_counter % repeat_every_x_frames == 0)
            {
                return true;
            }
        }
        return false;
    }

    pub fn axis_repeat_press(
        &self,
        action: &str,
        repeat_delay: u128,
        repeat_every_x_frames: u128,
    ) -> f32 {
        if let Some(input) = self.statistics.get(action) {
            if self.is_valid_repeat_press(action, repeat_delay, repeat_every_x_frames) {
                return input.action_axis_value;
            }
        }
        return 0.0;
    }

    pub fn is_valid_cooldown_press(&self, action: &str, cooldown_millis: u128) -> bool {
        if let Some(input) = self.statistics.get(action) {
            if !input.action_is_down {
                return false;
            }
            return input.press_length_millis.elapsed().as_millis() >= cooldown_millis;
        }
        false
    }

    pub fn is_action_single_press(&self, action: &str) -> bool {
        return self.is_valid_repeat_press(action, 5000, 5000);
    }

    pub fn is_action_down(&self, action: &str) -> bool {
        if let Some(input) = self.statistics.get(action) {
            return input.action_is_down;
        }
        false
    }

    pub fn axis_value(&self, action: &str) -> f32 {
        if let Some(input) = self.statistics.get(action) {
            return input.action_axis_value;
        }
        0.0
    }
}

#[derive(SystemDesc)]
pub struct InputManagementSystem;

impl<'s> System<'s> for InputManagementSystem {
    type SystemData = (
        Read<'s, InputHandler<StringBindings>>,
        Write<'s, InputManager>,
    );

    fn run(&mut self, (input, mut input_manager): Self::SystemData) {
        input_manager.frame_counter += 1;

        for action in input.bindings.actions().chain(input.bindings.axes()) {
            let stats = input_manager.statistics.get_mut(action).unwrap();

            let mut action_is_down = false;
            if let Some(value) = input.axis_value(action) {
                stats.action_axis_value = value;
                if value != 0.0 {
                    action_is_down = true;
                }
            } else if let Some(is_down) = input.action_is_down(action) {
                action_is_down = is_down;
            }

            if action_is_down {
                if !stats.action_is_down {
                    stats.press_length_millis = Instant::now();
                }
                stats.action_is_down = true;
                stats.action_down_frame_count += 1;
            } else {
                if stats.action_is_down {
                    stats.time_since_last_press_millis = Instant::now();
                }
                stats.action_is_down = false;
                stats.action_down_frame_count = 0;
            }
        }
    }
}

pub const REPEAT_DELAY: u128 = 250;
pub const COOLDOWN_DELAY: u128 = 250;
