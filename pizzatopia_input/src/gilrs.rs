use amethyst::prelude::World;
use amethyst::{
    ecs::{System, Write},
    input::{BindingTypes, InputHandler},
};
use std::marker::PhantomData;

use gilrs::{Axis, Button, Event, EventType, Gilrs};

use amethyst::{
    core::shrev::EventChannel,
    input::{ControllerAxis, ControllerButton, ControllerEvent, InputEvent},
};

/// Add [gilrs] bindings to amethyst controller input
#[derive(Default)]
pub struct GilRsControllerSystem<B: BindingTypes> {
    gilrs: Option<Gilrs>,
    controllers_registered: bool,
    phantom: PhantomData<B>,
}

impl<'s, B: BindingTypes> System<'s> for GilRsControllerSystem<B> {
    type SystemData = (
        Write<'s, InputHandler<B>>,
        Write<'s, EventChannel<InputEvent<B>>>,
    );

    fn run(&mut self, (mut input_handler, mut events): Self::SystemData) {
        if let Some(ref mut gilrs) = &mut self.gilrs {
            if !self.controllers_registered {
                for (id, _gamepad) in gilrs.gamepads() {
                    let id: usize = id.into();
                    let which = id as u32;
                    input_handler.send_controller_event(
                        &ControllerEvent::ControllerConnected { which },
                        &mut events,
                    );
                }

                self.controllers_registered = true;
            }

            // Examine new events
            while let Some(gilrs_event) = gilrs.next_event() {
                let controller_event = controller_event_from_event(gilrs_event);
                if let Some(event) = controller_event {
                    input_handler.send_controller_event(&event, &mut events);
                }
            }
        }
    }

    fn setup(&mut self, _world: &mut World) {
        let gilrs = Gilrs::new();
        match gilrs {
            Ok(success) => {
                self.gilrs = Some(success);
            }
            Err(message) => eprintln!(
                "Controller input could be initialized. Error: {}",
                message.to_string()
            ),
        }
    }
}

fn controller_event_from_event(e: Event) -> Option<ControllerEvent> {
    let Event { id, event, time: _ } = e;
    let id: usize = id.into();
    let which = id as u32;

    match event {
        EventType::AxisChanged(axis, value, _code) => match controller_axis_from_axis(axis) {
            Some(axis) => Some(ControllerEvent::ControllerAxisMoved { which, axis, value }),
            _ => None,
        },
        EventType::ButtonPressed(button, _code) => match controller_button_from_button(button) {
            Some(button) => Some(ControllerEvent::ControllerButtonPressed { which, button }),
            _ => match controller_axis_from_button(button) {
                Some(axis) => Some(ControllerEvent::ControllerAxisMoved {
                    which,
                    axis,
                    value: 1.0,
                }),
                _ => None,
            },
        },
        EventType::ButtonReleased(button, _code) => match controller_button_from_button(button) {
            Some(button) => Some(ControllerEvent::ControllerButtonReleased { which, button }),
            _ => match controller_axis_from_button(button) {
                Some(axis) => Some(ControllerEvent::ControllerAxisMoved {
                    which,
                    axis,
                    value: 1.0,
                }),
                _ => None,
            },
        },
        _ => None,
    }
}

fn controller_button_from_button(b: Button) -> Option<ControllerButton> {
    match b {
        Button::South => Some(ControllerButton::A),
        Button::East => Some(ControllerButton::B),
        Button::West => Some(ControllerButton::X),
        Button::North => Some(ControllerButton::Y),
        Button::LeftTrigger => Some(ControllerButton::LeftShoulder),
        Button::RightTrigger => Some(ControllerButton::RightShoulder),
        Button::Select => Some(ControllerButton::Guide),
        Button::Start => Some(ControllerButton::Start),
        Button::LeftThumb => Some(ControllerButton::LeftStick),
        Button::RightThumb => Some(ControllerButton::RightStick),
        _ => None,
    }
}

fn controller_axis_from_button(b: Button) -> Option<ControllerAxis> {
    match b {
        Button::LeftTrigger2 => Some(ControllerAxis::LeftTrigger),
        Button::RightTrigger2 => Some(ControllerAxis::RightTrigger),
        _ => None,
    }
}

fn controller_axis_from_axis(a: Axis) -> Option<ControllerAxis> {
    match a {
        Axis::LeftStickX | Axis::DPadX => Some(ControllerAxis::LeftX),
        Axis::LeftStickY | Axis::DPadY => Some(ControllerAxis::LeftY),
        Axis::RightStickX => Some(ControllerAxis::RightX),
        Axis::RightStickY => Some(ControllerAxis::RightY),
        Axis::LeftZ => Some(ControllerAxis::LeftTrigger),
        Axis::RightZ => Some(ControllerAxis::RightTrigger),
        _ => None,
    }
}
