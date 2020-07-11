use crate::events::Events;
use amethyst::{
    ecs::prelude::Entity,
    prelude::{World, WorldExt},
    ui::UiEvent,
};
use derivative::Derivative;

pub mod current_actions;
pub mod file_picker;
pub mod tile_characteristics;

const COLOR_WHITE: [f32; 4] = [1., 1., 1., 1.];
const COLOR_BLACK: [f32; 4] = [0., 0., 0., 1.];
const COLOR_RED: [f32; 4] = [1., 0., 0., 1.];
const COLOR_GOLD: [f32; 4] = [1., 0.8, 0., 1.];
const COLOR_GOLDEN_RED: [f32; 4] = [1., 0.6, 0.2, 1.];
const COLOR_GRAY: [f32; 4] = [0.75, 0.75, 0.75, 0.75];

pub fn with_transparent(mut color: [f32; 4], alpha: f32) -> [f32; 4] {
    color[3] *= alpha;
    color
}

pub trait UiComponent {
    fn entities_to_remove(&self, to_remove: Vec<Entity>) -> Vec<Entity>;
    fn update(&mut self, world: &World);
    fn handle_ui_events(&mut self, world: &World, event: UiEvent);
    fn handle_custom_events(&mut self, _world: &World, _event: Events) {}
    fn should_capture_input(&self, _world: &World) -> bool {
        true
    }
    fn blocks_all_other_input(&self) -> bool {
        false
    }
    fn delete_components(&self, world: &mut World) {
        let mut to_remove = Vec::new();
        to_remove = self.entities_to_remove(to_remove);
        world
            .delete_entities(to_remove.as_slice())
            .expect("Failed to delete ui entities.");
    }
    fn should_destroy(&self) -> bool {
        false
    }
}

#[derive(Derivative)]
#[derivative(Default)]
pub struct UiStack {
    pub stack: Vec<Box<dyn UiComponent + Send + Sync>>,
}

impl UiStack {
    pub fn top(&mut self) -> &mut Box<dyn UiComponent + Send + Sync> {
        return &mut self.stack[0];
    }

    pub fn handle_ui_events(
        &mut self,
        world: &World,
        ui_event: Option<UiEvent>,
        event: Option<Events>,
    ) {
        for ui in &mut self.stack {
            if ui.should_capture_input(world) {
                if let Some(ev) = &ui_event {
                    ui.handle_ui_events(world, ev.clone());
                }
                if let Some(ev) = &event {
                    ui.handle_custom_events(world, ev.clone());
                }
                if ui.blocks_all_other_input() {
                    break;
                }
            }
        }
    }
    pub fn is_blocking_all_input(&self) -> bool {
        for ui in &self.stack {
            if ui.blocks_all_other_input() {
                return true;
            }
        }
        return false;
    }
}
