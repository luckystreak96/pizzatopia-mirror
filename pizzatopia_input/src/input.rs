use amethyst::{
    input::{BindingTypes, InputHandler},
    prelude::{World, WorldExt},
};
use derivative::Derivative;
use log::error;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::time::Instant;

/// Information used to track input state.
#[derive(Derivative)]
#[derivative(Default)]
pub(crate) struct InputStatistics {
    #[derivative(Default(value = "Instant::now()"))]
    /// Updates every time a key is released
    last_released: Instant,
    #[derivative(Default(value = "Instant::now()"))]
    /// Updates whenever an input switches from `action_is_down: false` to `action_is_down: true`
    last_pressed: Instant,
    /// Count the number of frames the input has been pressed for
    same_action_frame_count: u32,
    /// `true` if the input is being pressed, false otherwise
    action_is_down: bool,
    /// The axis value in the range `[-1.0, 1.0]` for controllers, or `0.0` for other input
    action_axis_value: f32,
}

impl InputStatistics {
    /// Return `true` if the axis value is high enough to be considered pressed
    pub(crate) fn set_axis_value(&mut self, axis_value: f32) -> bool {
        self.action_axis_value = axis_value;
        if axis_value.abs() >= 0.3 {
            self.action_is_down = true;
        } else {
            self.action_is_down = false;
        }
        self.action_is_down
    }
}

/// Represents the input status given the desired restrictions
#[derive(Derivative, Clone)]
#[derivative(Default)]
pub struct InputResult {
    /// Is true if all conditions are met and the input is being pressed
    pub is_down: bool,
    /// The axis value in the range `[-1.0, 1.0]` for controllers if all conditions are met,
    /// or `0.0` if any condition is not met.
    pub axis: f32,
}

impl InputResult {
    fn new(stats: &InputStatistics) -> Self {
        InputResult {
            is_down: stats.action_is_down,
            axis: stats.action_axis_value,
        }
    }
}

/// Wrapper that handles queries on the different types of input.
#[derive(Default)]
pub struct InputManager<B, T>
where
    B: BindingTypes + Default,
    T: Clone + Debug + Hash + Eq + Send + Sync + 'static,
{
    pub frame_counter: u128,
    pub(crate) statistics: HashMap<T, InputStatistics>,
    phantom: PhantomData<B>,
}

impl<B, T> InputManager<B, T>
where
    B: BindingTypes + Default,
    T: Clone + Debug + Hash + Eq + Send + Sync + 'static,
{
    fn new() -> Self {
        Self {
            frame_counter: 0,
            statistics: HashMap::new(),
            phantom: Default::default(),
        }
    }

    /// Returns `InputResult` with positive results only on:
    /// - The first frame of a keypress
    /// - Every `repeat_every_x_frames` frames after a delay of `repeat_delay_millis`
    pub fn is_valid_repeat_press(
        &self,
        action: T,
        repeat_delay_millis: u128,
        repeat_every_x_frames: u128,
    ) -> InputResult {
        if let Some(input) = self.statistics.get(&action) {
            // 2 cases: the button was JUST pressed (elapsed = 0), or enough time passed
            let elapsed = input.last_pressed.elapsed().as_millis();
            if input.action_is_down
                && (input.same_action_frame_count == 1
                    || (elapsed >= repeat_delay_millis
                        && self.frame_counter % repeat_every_x_frames == 0))
            {
                return InputResult::new(input);
            }
        } else {
            error!("Action {:?} not registered!", action);
        }
        return InputResult::default();
    }

    /// Returns `InputResult` with positive results only if:
    /// - The input has not been pressed for at least `cooldown_millis` time
    /// - The input gets pressed
    pub fn is_valid_cooldown_press(&self, action: T, cooldown_millis: u128) -> InputResult {
        if let Some(input) = self.statistics.get(&action) {
            let is_cooldown_elapsed = input.last_released.elapsed().as_millis() >= cooldown_millis;
            if input.action_is_down && is_cooldown_elapsed {
                return InputResult::new(input);
            }
        }
        return InputResult::default();
    }

    /// Returns `InputResult` with positive results only if:
    /// - The input gets pressed after having been released
    pub fn action_single_press(&self, action: T) -> InputResult {
        return self.is_valid_repeat_press(action, 5000, 5000);
    }

    /// Returns `true` if the input was released this frame:
    pub fn action_just_released(&self, action: T) -> bool {
        if let Some(stats) = self.statistics.get(&action) {
            if stats.same_action_frame_count == 1 && !stats.action_is_down {
                return true;
            }
        }
        return false;
    }

    /// Returns `InputResult` with the current input status.
    /// Simply indicates whether the input in being pressed or not, without any extra conditions.
    pub fn action_status(&self, action: T) -> InputResult {
        if let Some(input) = self.statistics.get(&action) {
            return InputResult::new(input);
        }
        return InputResult::default();
    }

    pub(crate) fn update_statistics(&mut self, action: T, is_down: bool) {
        self.frame_counter += 1;

        let stats = self.statistics.get_mut(&action).unwrap();
        if is_down {
            if !stats.action_is_down {
                stats.last_pressed = Instant::now();
                stats.same_action_frame_count = 0;
            }
            stats.same_action_frame_count += 1;
            stats.action_is_down = true;
        } else {
            if stats.action_is_down {
                stats.last_released = Instant::now();
                stats.same_action_frame_count = 0;
            }
            stats.action_is_down = false;
            stats.action_axis_value = 0.0;
            stats.same_action_frame_count += 1;
        }
    }
}

/// Resource to query input state. Abstracts different common input situations.
#[derive(Default)]
pub struct Input<B>
where
    B: BindingTypes + Default,
{
    pub actions: InputManager<B, B::Action>,
    pub axes: InputManager<B, B::Axis>,
}
impl<B> Input<B>
where
    B: BindingTypes + Default,
{
    /// Initialize action and axis `InputManager`s
    pub fn new(world: &World) -> Self {
        let mut result = Self {
            actions: InputManager::new(),
            axes: InputManager::new(),
        };

        let input = world.read_resource::<InputHandler<B>>();
        for action in input.bindings.actions() {
            result
                .actions
                .statistics
                .insert(action.clone(), InputStatistics::default());
        }
        for axis in input.bindings.axes() {
            result
                .axes
                .statistics
                .insert(axis.clone(), InputStatistics::default());
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amethyst::input::StringBindings;

    #[test]
    fn input_statistics_axis() {
        let mut stats: InputStatistics = InputStatistics::default();
        assert_eq!(stats.set_axis_value(0.25), false);
        assert_eq!(stats.set_axis_value(-0.25), false);
        assert_eq!(stats.set_axis_value(0.3), true);
    }

    #[test]
    fn input_manager() {
        let mut manager = InputManager::<StringBindings, String>::new();
        let test_action = String::from("test");
        manager
            .statistics
            .insert(test_action.clone(), InputStatistics::default());

        // Default
        let result = manager.action_status(test_action.clone());
        assert_eq!(result.is_down, false);

        // Action status
        manager.update_statistics(test_action.clone(), true);
        let result = manager.action_status(test_action.clone());
        assert_eq!(result.is_down, true);
        assert_eq!(result.axis, 0.0);

        manager.update_statistics(test_action.clone(), false);
        let result = manager.action_status(test_action.clone());
        assert_eq!(result.is_down, false);

        // Action single press
        manager.update_statistics(test_action.clone(), true);
        let result = manager.action_single_press(test_action.clone());
        assert_eq!(result.is_down, true);

        manager.update_statistics(test_action.clone(), true);
        let result = manager.action_single_press(test_action.clone());
        assert_eq!(result.is_down, false);

        // Cool down press
        manager.update_statistics(test_action.clone(), true);
        let result = manager.is_valid_cooldown_press(test_action.clone(), 1000);
        assert_eq!(result.is_down, false);
        let result = manager.is_valid_cooldown_press(test_action.clone(), 0);
        assert_eq!(result.is_down, true);

        // Repeat press
        let mut counter = 0;
        for i in 0..10 {
            manager.update_statistics(test_action.clone(), true);
            if manager
                .is_valid_repeat_press(test_action.clone(), 0, 2)
                .is_down
            {
                counter += 1;
            }
        }
        assert_eq!(counter, 5);

        let result = manager.is_valid_repeat_press(test_action.clone(), 10000, 0);
        assert_eq!(result.is_down, false);
    }
}
