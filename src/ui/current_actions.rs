use crate::components::editor::{EditorCursor, EditorState, InsertionGameObject};
use crate::components::game::{SerializedObject, SerializedObjectType};
use crate::components::physics::Position;
use crate::states::pizzatopia::TILE_HEIGHT;
use crate::systems::editor::{EditorEvents, EDITOR_MODIFIERS_ALL, EDITOR_MODIFIERS_UI};
use crate::systems::input::InputManager;
use crate::ui::{
    with_transparent, UiComponent, COLOR_BLACK, COLOR_GOLD, COLOR_GOLDEN_RED, COLOR_GRAY,
    COLOR_RED, COLOR_WHITE,
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
use crate::ui::file_picker::FilePickerFilename;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(PartialOrd, Ord, PartialEq, Clone, Copy, Eq, EnumIter)]
pub enum EditorActions {
    EnterPlayMode,
    EnterInsertMode,
    EnterEditModeFromInsert,
    EnterEditGameObject,
    PlaceEditGameObject,
    DeleteEditGameObject,
    InsertModePlayer,
    InsertModeTile,
    SaveLevel,
    ChooseSaveFile,
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
    pub labels: BTreeMap<EditorActions, ActionPackage>,
    hover_game_object: bool,
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
        self.update_component_visibility(world);
        self.update_component_positions(world);
        self.hover_game_object = false;
    }

    fn handle_ui_events(&mut self, _world: &World, _event: UiEvent) {}
    fn handle_custom_events(&mut self, _world: &World, event: Events) {
        match event {
            Events::HoverGameObject => {
                self.hover_game_object = true;
            }
            _ => {}
        }
    }
}

const LABEL_WIDTH: f32 = 200.0;
const LABEL_HEIGHT: f32 = 25.0;
const BUTTON_WIDTH: f32 = 40.0;
const BUTTON_HEIGHT: f32 = 40.0;
const FONT_SIZE: f32 = 18.;
const X_OFFSET: f32 = 0.0;

impl CurrentActionsUi {
    pub fn update_component_positions(&mut self, world: &World) {
        let mut bottom_left_index = 0;
        let mut top_middle_index = 0;
        for action in EditorActions::iter() {
            // Bottom left
            match action {
                EditorActions::EnterInsertMode
                | EditorActions::EnterEditModeFromInsert
                | EditorActions::EnterEditGameObject
                | EditorActions::InsertModeTile
                | EditorActions::InsertModePlayer
                | EditorActions::DeleteEditGameObject
                | EditorActions::PlaceEditGameObject
                | EditorActions::EnterPlayMode => {
                    let pack = self.labels.get(&action).unwrap();
                    if pack.show {
                        let button_text = match action {
                            EditorActions::EnterPlayMode => "LCTRL",
                            _ => "",
                        };
                        let button_len = button_text.len() as f32 * FONT_SIZE / 1.5;
                        let comb = vec![
                            (pack.button, X_OFFSET, BUTTON_WIDTH + button_len),
                            (
                                pack.label,
                                X_OFFSET + button_len + BUTTON_WIDTH,
                                LABEL_WIDTH,
                            ),
                        ];
                        for (ent, x_pos, width) in comb {
                            if let Some(component) =
                                world.write_storage::<UiTransform>().get_mut(ent)
                            {
                                component.local_x = x_pos;
                                component.local_y = BUTTON_HEIGHT.max(LABEL_HEIGHT)
                                    * (bottom_left_index as f32 + 0.5);
                                component.width = width;
                            }
                        }
                        bottom_left_index += 1;
                    }
                }
                _ => {}
            }

            // Top middle
            match action {
                EditorActions::SaveLevel => {
                    let pack = self.labels.get(&action).unwrap();
                    if pack.show {
                        let button_text = "INSERT";
                        let button_len = button_text.len() as f32 * FONT_SIZE / 1.5;
                        let comb = vec![
                            (pack.button, X_OFFSET, button_len),
                            (pack.label, X_OFFSET + button_len, LABEL_WIDTH),
                        ];
                        for (ent, x_pos, width) in comb {
                            if let Some(component) =
                                world.write_storage::<UiTransform>().get_mut(ent)
                            {
                                component.local_x = x_pos
                                    + ((BUTTON_WIDTH + LABEL_WIDTH) * (top_middle_index as f32));
                                component.local_y = -BUTTON_HEIGHT / 2.0;
                                component.anchor = Anchor::TopMiddle;
                                component.pivot = Anchor::MiddleLeft;
                                component.width = width;
                            }
                        }
                        if let Some(component) =
                            world.write_storage::<UiText>().get_mut(pack.button)
                        {
                            component.text = button_text.to_string();
                            component.font_size = button_len / button_text.len() as f32;
                        }
                        top_middle_index += 1;
                    }
                }
                _ => {}
            }

            // Top right
            match action {
                EditorActions::ChooseSaveFile => {
                    let pack = self.labels.get(&action).unwrap();
                    if pack.show {
                        let filename = world.read_resource::<FilePickerFilename>().filename.clone();
                        let button_text = "ENTER";
                        let button_len = BUTTON_WIDTH + button_text.len() as f32 * FONT_SIZE / 1.5;
                        let comb = vec![
                            (pack.button, X_OFFSET, button_text, button_len),
                            (
                                pack.label,
                                X_OFFSET + button_len,
                                filename.as_str(),
                                LABEL_WIDTH,
                            ),
                        ];
                        for (ent, x_pos, text, width) in comb {
                            if let Some(component) =
                                world.write_storage::<UiTransform>().get_mut(ent)
                            {
                                component.local_x = x_pos - (button_len + LABEL_WIDTH);
                                component.local_y = -BUTTON_HEIGHT / 2.0;
                                component.anchor = Anchor::TopRight;
                                component.width = width;
                            }
                            if let Some(component) = world.write_storage::<UiText>().get_mut(ent) {
                                component.text = text.to_string();
                            }
                        }
                        if let Some(component) =
                            world.write_storage::<UiImage>().get_mut(pack.button)
                        {
                            match component {
                                UiImage::Sprite(ref mut sprite) => {
                                    sprite.sprite_number = 1;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn show(&mut self, action: EditorActions, show: bool) {
        self.labels.get_mut(&action).unwrap().show = show;
    }

    pub fn update_component_visibility(&mut self, world: &World) {
        let state = (*world.read_resource::<EditorState>()).clone();
        for action in EditorActions::iter() {
            self.show(action, false);
            match action {
                EditorActions::EnterInsertMode => match state {
                    EditorState::EditMode => {
                        self.show(action, true);
                    }
                    _ => {}
                },
                EditorActions::EnterEditModeFromInsert => match state {
                    EditorState::InsertMode => {
                        self.show(action, true);
                    }
                    _ => {}
                },
                EditorActions::EnterEditGameObject => match state {
                    EditorState::EditMode => {
                        if self.hover_game_object {
                            self.show(action, true);
                        }
                    }
                    _ => {}
                },
                EditorActions::PlaceEditGameObject => match state {
                    EditorState::EditGameObject => {
                        self.show(action, true);
                    }
                    _ => {}
                },
                EditorActions::DeleteEditGameObject => match state {
                    EditorState::EditMode => {
                        if self.hover_game_object {
                            self.show(action, true);
                        }
                    }
                    EditorState::EditGameObject => {
                        self.show(action, true);
                    }
                    _ => {}
                },
                EditorActions::InsertModeTile => match state {
                    EditorState::InsertMode => {
                        self.show(action, true);
                    }
                    _ => {}
                },
                EditorActions::InsertModePlayer => match state {
                    EditorState::InsertMode => {
                        self.show(action, true);
                    }
                    _ => {}
                },
                EditorActions::EnterPlayMode => {
                    self.show(action, true);
                }
                EditorActions::SaveLevel | EditorActions::ChooseSaveFile => {
                    self.show(action, true);
                }
            }
        }

        for (_, component) in &self.labels {
            if component.show {
                Self::show_component(world, component);
            } else {
                Self::hide_component(world, component);
            }
        }
    }

    pub fn hide_all_components(&mut self, world: &World) {
        for (_, pack) in &self.labels {
            Self::hide_component(world, pack);
        }
    }

    pub fn hide_component(world: &World, pack: &ActionPackage) {
        world
            .write_storage::<HiddenPropagate>()
            .insert(pack.button.clone(), HiddenPropagate::new())
            .unwrap();
        world
            .write_storage::<HiddenPropagate>()
            .insert(pack.label.clone(), HiddenPropagate::new())
            .unwrap();
    }

    pub fn show_component(world: &World, comp: &ActionPackage) {
        world
            .write_storage::<HiddenPropagate>()
            .remove(comp.button.clone());
        world
            .write_storage::<HiddenPropagate>()
            .remove(comp.label.clone());
    }

    pub fn new(world: &mut World) -> Self {
        let mut result = Self::initialize_ui(world);
        result.hide_all_components(world);
        return result;
    }

    fn initialize_ui(world: &mut World) -> Self {
        let mut result: CurrentActionsUi = CurrentActionsUi::default();
        let font = (*world.read_resource::<Handle<FontAsset>>()).clone();

        let mut i = 0;
        for action in EditorActions::iter() {
            let y = BUTTON_HEIGHT.max(LABEL_HEIGHT) * (i as f32 + 0.5);

            // Label
            let transform = Self::create_ui_transform(
                String::from("ActionButton"),
                X_OFFSET,
                y,
                BUTTON_WIDTH,
                BUTTON_HEIGHT,
                i,
            );
            let filename = world.read_resource::<FilePickerFilename>().filename.clone();
            let text_string = match action {
                EditorActions::ChooseSaveFile => ("ENTER", filename.as_str()),
                EditorActions::EnterInsertMode => ("A", "Add Objects"),
                EditorActions::EnterEditModeFromInsert => ("Z", "Return To Edit"),
                EditorActions::EnterEditGameObject => ("X", "Edit Object"),
                EditorActions::InsertModeTile => ("1", "Switch To Tiles"),
                EditorActions::InsertModePlayer => ("2", "Switch To Players"),
                EditorActions::EnterPlayMode => ("LCTRL", "Play Level"),
                EditorActions::SaveLevel => ("INSERT", "Save"),
                EditorActions::PlaceEditGameObject => ("X", "Place Object"),
                EditorActions::DeleteEditGameObject => ("Z", "Remove Object"),
            };
            let text = Self::create_ui_text(String::from(text_string.0), font.clone(), true);
            let button_entity = Self::create_ui_entity(world, transform, text, true);

            let transform = Self::create_ui_transform(
                String::from("ActionLabel"),
                X_OFFSET + BUTTON_WIDTH,
                y,
                LABEL_WIDTH,
                LABEL_HEIGHT,
                i,
            );
            let text = Self::create_ui_text(String::from(text_string.1), font.clone(), false);
            let label_entity = Self::create_ui_entity(world, transform, text, false);

            result
                .labels
                .insert(action, ActionPackage::new(button_entity, label_entity));
            i += 1;
        }

        return result;
    }

    fn create_ui_text(text: String, font: Handle<FontAsset>, bg: bool) -> UiText {
        let mut text = UiText::new(
            font.clone(),
            format!("{}", text).to_string(),
            match bg {
                false => COLOR_WHITE,
                true => COLOR_BLACK,
            },
            FONT_SIZE,
        );
        if !bg {
            text.align = Anchor::MiddleLeft;
        }
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
        } else {
            entity = entity.with(UiImage::SolidColor(with_transparent(COLOR_GRAY, 0.05)))
        }

        entity.build()
    }
}
