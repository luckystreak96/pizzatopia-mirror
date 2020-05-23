use amethyst::derive::SystemDesc;
use amethyst::ecs::{
    Join, NullStorage, Read, ReadStorage, System, SystemData, World, Write, WriteStorage,
};
use amethyst::input::{InputHandler, StringBindings};
use amethyst::prelude::WorldExt;
use derivative::Derivative;
use log::{error, warn};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

#[derive(Derivative, Debug, Clone)]
#[derivative(Default)]
struct InputStatistics {
    #[derivative(Default(value = "Instant::now()"))]
    time_since_last_press_millis: Instant,
    #[derivative(Default(value = "Instant::now()"))]
    press_length_millis: Instant,
    #[derivative(Default(value = "false"))]
    action_is_down: bool,
    same_action_frame_count: u32,
    action_axis_value: f32,
}

#[derive(Derivative, Clone)]
#[derivative(Default)]
pub struct InputResult {
    pub is_down: bool,
    pub axis: f32,
    modifier_keys_down: Vec<String>,
}
impl InputResult {
    fn new(stats: &InputStatistics, modifiers_down: Vec<String>) -> Self {
        InputResult {
            is_down: stats.action_is_down,
            axis: stats.action_axis_value,
            modifier_keys_down: modifiers_down,
        }
    }

    pub fn excluding_modifiers(self, modifiers: &[&str]) -> Self {
        match self.expected_amount_of_modifiers_down(0, modifiers) {
            true => self,
            false => InputResult::default(),
        }
    }

    pub fn including_modifiers(self, modifiers: &[&str]) -> Self {
        match self.expected_amount_of_modifiers_down(modifiers.len(), modifiers) {
            true => self,
            false => InputResult::default(),
        }
    }

    fn expected_amount_of_modifiers_down(&self, amount: usize, modifiers: &[&str]) -> bool {
        return modifiers
            .into_iter()
            .map(|modifier| self.modifier_keys_down.contains(&String::from(*modifier)))
            .filter(|is_down| *is_down == true)
            .count()
            == amount;
    }
}

#[derive(Default)]
pub struct InputManager {
    statistics: BTreeMap<String, InputStatistics>,
    modifier_keys_down: Vec<String>,
    pub frame_counter: u128,
}

impl InputManager {
    pub fn new(world: &World) -> Self {
        let mut result = InputManager {
            statistics: BTreeMap::new(),
            modifier_keys_down: Vec::new(),
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
    ) -> InputResult {
        if let Some(input) = self.statistics.get(action) {
            // 2 cases: the button was JUST pressed (elapsed = 0), or enough time passed
            let elapsed = input.press_length_millis.elapsed().as_millis();
            if input.action_is_down
                && (input.same_action_frame_count == 1
                    || (elapsed >= repeat_delay && self.frame_counter % repeat_every_x_frames == 0))
            {
                return InputResult::new(input, self.modifier_keys_down.clone());
            }
        } else {
            error!("Action {} not registered!", action);
        }
        return InputResult::default();
    }

    pub fn is_valid_cooldown_press(&self, action: &str, cooldown_millis: u128) -> InputResult {
        if let Some(input) = self.statistics.get(action) {
            let is_cooldown_elapsed =
                input.press_length_millis.elapsed().as_millis() >= cooldown_millis;
            if input.action_is_down && is_cooldown_elapsed {
                return InputResult::new(input, self.modifier_keys_down.clone());
            }
        }
        return InputResult::default();
    }

    pub fn action_single_press(&self, action: &str) -> InputResult {
        return self.is_valid_repeat_press(action, 5000, 5000);
    }

    pub fn action_just_released(&self, action: &str) -> bool {
        if let Some(stats) = self.statistics.get(action) {
            if stats.same_action_frame_count == 1 && !stats.action_is_down {
                return true;
            }
        }
        return false;
    }

    pub fn action_status(&self, action: &str) -> InputResult {
        if let Some(input) = self.statistics.get(action) {
            return InputResult::new(input, self.modifier_keys_down.clone());
        }
        return InputResult::default();
    }
}

#[derive(SystemDesc)]
pub struct InputManagementSystem;

const EQUIVALENCES: &[(&str, &str)] = &[
    ("vertical_controller", "vertical"),
    ("horizontal_controller", "horizontal"),
    ("modifier1_controller", "modifier1"),
];

impl InputManagementSystem {
    fn update_equivalences(&mut self, input: &mut InputManager) {
        for (key, value) in EQUIVALENCES {
            let key_stats = input
                .statistics
                .get(*key)
                .unwrap_or(&InputStatistics::default())
                .clone();
            if let Some(value_stats) = input.statistics.get_mut(*value) {
                if key_stats.action_is_down {
                    *value_stats = key_stats.clone();
                }
            }
        }
    }
}

impl<'s> System<'s> for InputManagementSystem {
    type SystemData = (
        Read<'s, InputHandler<StringBindings>>,
        Write<'s, InputManager>,
    );

    fn run(&mut self, (input, mut input_manager): Self::SystemData) {
        input_manager.frame_counter += 1;
        input_manager.modifier_keys_down.clear();

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
                    stats.same_action_frame_count = 0;
                }
                stats.same_action_frame_count += 1;
                stats.action_is_down = true;
                if action.contains("modifier") {
                    for (key, value) in EQUIVALENCES {
                        if key == action {
                            input_manager.modifier_keys_down.push(String::from(*value));
                        }
                    }
                    input_manager.modifier_keys_down.push(action.clone());
                }
            } else {
                if stats.action_is_down {
                    stats.time_since_last_press_millis = Instant::now();
                    stats.same_action_frame_count = 0;
                }
                stats.action_is_down = false;
                stats.action_axis_value = 0.0;
                stats.same_action_frame_count += 1;
            }
        }
        self.update_equivalences(&mut input_manager);
    }
}

pub const REPEAT_DELAY: u128 = 250;
pub const COOLDOWN_DELAY: u128 = 250;
