use crate::components::editor::{EditorCursor, EditorState, InsertionGameObject};
use crate::components::game::{SerializedObject, SerializedObjectType};
use crate::components::physics::Position;
use crate::states::pizzatopia::TILE_HEIGHT;
use crate::systems::editor::{EditorEvents, EDITOR_MODIFIERS_ALL, EDITOR_MODIFIERS_UI};
use crate::systems::input::InputManager;
use crate::ui::{
    UiComponent, COLOR_BLACK, COLOR_GOLD, COLOR_GOLDEN_RED, COLOR_GRAY, COLOR_RED, COLOR_WHITE,
};
use crate::utils::Vec2;
use amethyst::prelude::{Builder, WorldExt};
use amethyst::{
    assets::{AssetStorage, Handle, Loader},
    core::{shrev::EventChannel, transform::Transform, HiddenPropagate},
    ecs::prelude::{Component, DenseVecStorage, Entity, Join, NullStorage},
    prelude::World,
    renderer::{
        Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture, Transparent,
    },
    ui::{
        Anchor, FontAsset, Interactable, Selectable, Selected, TextEditing, TtfFormat, UiEvent,
        UiEventType, UiImage, UiText, UiTransform,
    },
};
use derivative::Derivative;
use log::{error, warn};
use num_traits::Zero;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use crate::components::graphics::SpriteSheetType;
use crate::events::Events;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(PartialOrd, Ord, PartialEq, Clone, Copy, Eq, EnumIter)]
pub enum CurrentActions {
    EnterInsertMode,
    EnterEditModeFromInsert,
    EnterEditGameObject,
    InsertModeTile,
    InsertModePlayer,
    EnterPlayMode,
    EnterEditorMode,
    SaveLevel,
    ChooseSaveFile,
    ShowCurrentSaveFile,
}

pub struct ActionPackage {
    button: Entity,
    label: Entity,
    show: bool,
}

impl ActionPackage {
    pub fn new(button: Entity, label: Entity) -> ActionPackage {
        ActionPackage {
            button,
            label,
            show: false,
        }
    }
}

#[derive(Derivative)]
#[derivative(Default)]
pub struct CurrentActionsUi {
    pub labels: BTreeMap<CurrentActions, ActionPackage>,
}

impl UiComponent for CurrentActionsUi {
    fn entities_to_remove(&self, mut to_remove: Vec<Entity>) -> Vec<Entity> {
        for action_package in self.labels.values() {
            to_remove.push(action_package.button.clone());
            to_remove.push(action_package.label.clone());
        }
        to_remove
    }

    fn update(&mut self, world: &World) {
        self.update_component_visibility();
        self.update_component_positions(world);
    }

    fn handle_ui_events(&mut self, _world: &World, _event: UiEvent) {}
    fn handle_custom_events(&mut self, world: &World, event: Events) {
        match event {
            Events::HoverGameObject => {
                self.labels
                    .get_mut(&CurrentActions::EnterEditGameObject)
                    .unwrap()
                    .show = true;
                self.update_component_positions(world);
            }
            _ => {}
        }
    }
}

impl CurrentActionsUi {
    pub fn update_component_visibility(&mut self) {
        // TODO : based on editor state
        // TODO : Set `show`
        // TODO : Hide and show components
        unimplemented!();
    }
    pub fn update_component_positions(&mut self, world: &World) {
        // TODO : Set the component positions based on how many there are per space
        /*
        i = 0
        foreach possible component in the bottom left corner (just a match that only matches actions in that area)
            if show == true
                set pos = i * width height etc
                i++
            else
                continue // don't update i

         repeat for every area that has competition
         */
        unimplemented!();
    }

    pub fn hide_all_components(&mut self, world: &World) {
        let mut actions = Vec::new();
        for (action, _) in &self.labels {
            actions.push(action.clone());
        }
        for action in actions {
            self.hide_component(world, &action);
        }
    }

    pub fn hide_component(&mut self, world: &World, action: &CurrentActions) {
        if let Some(pack) = self.labels.get(action) {
            world
                .write_storage::<HiddenPropagate>()
                .insert(pack.button.clone(), HiddenPropagate::new())
                .unwrap();
            world
                .write_storage::<HiddenPropagate>()
                .insert(pack.label.clone(), HiddenPropagate::new())
                .unwrap();
        }
    }

    pub fn show_component(&mut self, world: &World, action: &CurrentActions) {
        if let Some(comp) = self.labels.get(action) {
            world
                .write_storage::<HiddenPropagate>()
                .remove(comp.button.clone());
            world
                .write_storage::<HiddenPropagate>()
                .remove(comp.label.clone());
        }
    }

    pub fn new(world: &mut World) -> Self {
        let mut result = Self::initialize_ui(world);
        result.hide_all_components(world);
        return result;
    }

    fn initialize_ui(world: &mut World) -> Self {
        let mut result: CurrentActionsUi = CurrentActionsUi::default();
        let font = (*world.read_resource::<Handle<FontAsset>>()).clone();

        let label_width = 200.0;
        let label_height: f32 = 25.0;
        let button_width = 40.0;
        let button_height: f32 = 40.0;
        let font_size = 18.;
        let x_offset = 0.0;

        let mut i = 0;
        for action in CurrentActions::iter() {
            let y = button_height.max(label_height) * (i as f32 + 0.5);

            // Label
            let transform = Self::create_ui_transform(
                String::from("ActionButton"),
                x_offset,
                y,
                button_width,
                button_height,
                i,
            );
            match action {
                CurrentActions::ChooseSaveFile => {}
                CurrentActions::EnterInsertMode => {}
                CurrentActions::EnterEditModeFromInsert => {}
                CurrentActions::EnterEditGameObject => {}
                CurrentActions::InsertModeTile => {}
                CurrentActions::InsertModePlayer => {}
                CurrentActions::EnterPlayMode => {}
                CurrentActions::EnterEditorMode => {}
                CurrentActions::SaveLevel => {}
                CurrentActions::ShowCurrentSaveFile => {}
            }
            let text = Self::create_ui_text(String::from("X"), font_size, font.clone(), true);
            let button_entity = Self::create_ui_entity(world, transform, text, true);

            let transform = Self::create_ui_transform(
                String::from("ActionLabel"),
                x_offset + button_width / 2.0,
                y,
                label_width,
                label_height,
                i,
            );
            let text =
                Self::create_ui_text(String::from("PLACEHOLDER"), font_size, font.clone(), false);
            let label_entity = Self::create_ui_entity(world, transform, text, false);

            result
                .labels
                .insert(action, ActionPackage::new(button_entity, label_entity));
            i += 1;
        }

        return result;
    }

    fn create_ui_text(text: String, font_size: f32, font: Handle<FontAsset>, bg: bool) -> UiText {
        let text = UiText::new(
            font.clone(),
            format!("{}", text).to_string(),
            match bg {
                false => COLOR_WHITE,
                true => COLOR_BLACK,
            },
            font_size,
        );
        text
    }

    fn create_ui_transform(
        id: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        i: usize,
    ) -> UiTransform {
        let transform = UiTransform::new(
            format!("{}{}", id, i).to_string(),
            Anchor::BottomLeft,
            Anchor::MiddleLeft,
            x,
            y,
            1.,
            width,
            height,
        );
        transform
    }

    fn create_ui_entity(
        world: &mut World,
        transform: UiTransform,
        text: UiText,
        bg: bool,
    ) -> Entity {
        // Assign the sprite
        let sprite_sheet = world.read_resource::<BTreeMap<u8, Handle<SpriteSheet>>>()
            [&(SpriteSheetType::Ui as u8)]
            .clone();
        let sprite_render = SpriteRender {
            sprite_sheet: sprite_sheet.clone(),
            sprite_number: 0,
        };
        let sprite_render = UiImage::Sprite(sprite_render);
        let mut entity = world.create_entity().with(transform).with(text);
        if bg {
            entity = entity.with(sprite_render);
        }

        entity.build()
    }
}
