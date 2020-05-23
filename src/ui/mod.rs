use amethyst::{
    ecs::prelude::{DenseVecStorage, Entity},
    prelude::World,
    ui::UiEvent,
};
use derivative::Derivative;

pub mod file_picker;
pub mod tile_characteristics;

pub trait UiComponent {
    fn entities_to_remove(&self, to_remove: Vec<Entity>) -> Vec<Entity>;
    fn update(&mut self, world: &World);
    fn handle_ui_events(&mut self, world: &World, event: UiEvent);
    fn should_capture_input(&self, _world: &World) -> bool {
        true
    }
    fn blocks_all_other_input(&self) -> bool {
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

    pub fn handle_ui_events(&mut self, world: &World, event: &UiEvent) {
        for ui in &mut self.stack {
            if ui.should_capture_input(world) {
                ui.handle_ui_events(world, event.clone());
                if ui.blocks_all_other_input() {
                    break;
                }
            }
        }
    }
}
