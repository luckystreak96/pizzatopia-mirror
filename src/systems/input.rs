use amethyst::ecs::{
    Join, NullStorage, Read, ReadStorage, System, SystemData, World, Write, WriteStorage,
};
use amethyst::derive::SystemDesc;
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
}

#[derive(Default)]
pub struct InputManager {
    statistics: BTreeMap<String, InputStatistics>,
}

impl InputManager {
    pub fn new(world: &World) -> Self {
        let mut result = InputManager {
            statistics: BTreeMap::new(),
        };

        let input = world.read_resource::<InputHandler<StringBindings>>();
        for action in input.bindings.actions() {
            result
                .statistics
                .insert(action.clone(), InputStatistics::default());
        }

        result
    }

    pub fn is_valid_repeat_press(&self, action: String, repeat_delay: u128) -> bool {
        if let Some(input) = self.statistics.get(action.as_str()) {
            if !input.action_is_down {
                return false;
            }
            // 2 cases: the button was JUST pressed (elapsed = 0), or enough time passed
            let elapsed = input.press_length_millis.elapsed().as_millis();
            return elapsed == 0 || elapsed >= repeat_delay;
        }
        return false;
    }

    pub fn is_valid_cooldown_press(&self, action: String, cooldown_millis: u128) -> bool {
        if let Some(input) = self.statistics.get(action.as_str()) {
            if !input.action_is_down {
                return false;
            }
            return input.press_length_millis.elapsed().as_millis() >= cooldown_millis;
        }
        false
    }

    pub fn is_action_down(&self, action: String) -> bool {
        if let Some(input) = self.statistics.get(action.as_str()) {
            return input.action_is_down;
        }
        false
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
        for action in input.bindings.actions() {
            let stats = input_manager.statistics.get_mut(action).unwrap();

            if input.action_is_down(action).unwrap() {
                if !stats.action_is_down {
                    stats.press_length_millis = Instant::now();
                }
                stats.action_is_down = true;
            } else {
                if stats.action_is_down {
                    stats.time_since_last_press_millis = Instant::now();
                }
                stats.action_is_down = false;
            }
        }
    }
}

pub const REPEAT_DELAY: u128 = 250;
pub const COOLDOWN_DELAY: u128 = 250;
