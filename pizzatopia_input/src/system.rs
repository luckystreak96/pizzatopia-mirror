use crate::input::Input;
use amethyst::{
    ecs::{Read, System, Write},
    input::{BindingTypes, InputHandler},
};
use std::marker::PhantomData;

/// Updates input statistics, ideally called at the start of every frame
pub struct InputManagementSystem<B>
where
    B: BindingTypes + Default,
{
    phantom_bindings: PhantomData<B>,
}

impl<'s, B> System<'s> for InputManagementSystem<B>
where
    B: BindingTypes + Default,
    B::Axis: Default + Copy,
    B::Action: Default + Copy,
{
    type SystemData = (Read<'s, InputHandler<B>>, Write<'s, Input<B>>);

    fn run(&mut self, (input_handler, mut input): Self::SystemData) {
        for action in input_handler.bindings.actions() {
            let action_is_down = input_handler.action_is_down(action).unwrap_or(false);
            input.actions.update_statistics(*action, action_is_down);
        }

        for axis in input_handler.bindings.axes() {
            let action_is_down = if let Some(value) = input_handler.axis_value(axis) {
                let stats = input.axes.statistics.get_mut(axis).unwrap();
                stats.set_axis_value(value)
            } else {
                false
            };

            input.axes.update_statistics(*axis, action_is_down);
        }
    }
}
