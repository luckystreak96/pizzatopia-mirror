use crate::components::editor::{EditorCursor, EditorState, InsertionGameObject};
use crate::components::game::{SerializedObject, SerializedObjectType};
use crate::components::physics::Position;
use crate::states::pizzatopia::TILE_HEIGHT;
use crate::systems::editor::{EditorEvents, EDITOR_MODIFIERS_ALL, EDITOR_MODIFIERS_UI};
use crate::systems::input::InputManager;
use crate::ui::{UiComponent, COLOR_GOLD, COLOR_GOLDEN_RED, COLOR_GRAY, COLOR_RED};
use crate::utils::Vec2;
use amethyst::prelude::{Builder, WorldExt};
use amethyst::{
    assets::{AssetStorage, Handle, Loader},
    core::{shrev::EventChannel, transform::Transform, HiddenPropagate},
    ecs::prelude::{Component, DenseVecStorage, Entity, Join, NullStorage},
    prelude::World,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
    ui::{
        Anchor, FontAsset, Interactable, Selectable, Selected, TextEditing, TtfFormat, UiEvent,
        UiEventType, UiText, UiTransform,
    },
};
use derivative::Derivative;
use log::{error, warn};
use num_traits::Zero;
use std::fs;
use std::path::PathBuf;

pub const DIR_ASSETS: &str = "assets";
pub const DIR_LEVELS: &str = "levels";

#[derive(Derivative)]
#[derivative(Default)]
pub struct FilePickerUi {
    pub labels: Vec<Entity>,
    pub editable_label: Option<Entity>,
    ui_index: FilePickerUiIndex,
    should_destroy: bool,
}

impl UiComponent for FilePickerUi {
    fn entities_to_remove(&self, mut to_remove: Vec<Entity>) -> Vec<Entity> {
        for entity in self.labels.iter() {
            to_remove.push(entity.clone());
        }
        to_remove.push(self.editable_label.unwrap());
        to_remove
    }

    fn update(&mut self, world: &World) {
        self.update_ui(world);
    }

    fn handle_ui_events(&mut self, world: &World, event: UiEvent) {
        match &event.event_type {
            UiEventType::Click => {
                if let Some(button_info) =
                    world.read_storage::<FilePickerButton>().get(event.target)
                {
                    self.handle_click(&world, button_info)
                }
            }
            UiEventType::HoverStart => {
                if let Some(button_info) =
                    world.read_storage::<FilePickerButton>().get(event.target)
                {
                    self.ui_index.index = button_info.id;
                }
            }
            _ => {}
        }
    }

    fn blocks_all_other_input(&self) -> bool {
        true
    }

    fn should_destroy(&self) -> bool {
        self.should_destroy
    }
}

impl FilePickerUi {
    pub fn new(world: &mut World) -> Self {
        Self::initialize_ui(world)
    }

    fn update_color(&self, world: &World) {
        let mut ui_texts = world.write_storage::<UiText>();
        for i in 0..self.labels.len() {
            let color;
            let selected_index = self.ui_index.selected_index.unwrap_or(99999);
            if i == selected_index && selected_index == self.ui_index.index {
                color = COLOR_GOLDEN_RED;
            } else if i == selected_index {
                color = COLOR_GOLD;
            } else if i == self.ui_index.index {
                color = COLOR_RED;
            } else {
                color = COLOR_GRAY;
            }

            let entity = self.labels[i];
            if let Some(ui_text) = ui_texts.get_mut(entity) {
                ui_text.color = color;
            }
        }
    }

    fn initialize_ui(world: &mut World) -> Self {
        let font = (*world.read_resource::<Handle<FontAsset>>()).clone();

        let mut result: FilePickerUi = FilePickerUi::default();

        let mut filename_list = Vec::new();
        let path = PathBuf::from(DIR_ASSETS)
            .join(DIR_LEVELS)
            .display()
            .to_string();
        let paths = fs::read_dir(path).unwrap();
        for path in paths {
            filename_list.push(path.unwrap().file_name().into_string().unwrap());
        }

        result.ui_index.max_index = filename_list.len() - 1;
        result.ui_index.items_per_column = 15;

        let label_width = 200.0;
        let label_height = 25.0;
        let label_distance_height = 50.;
        let font_size = 18.;
        let num_columns = result.ui_index.max_index / result.ui_index.items_per_column + 1;
        let x_offset = (num_columns - 1) as f32 * label_width / 2.;

        for i in 0..filename_list.len() {
            let current_row = (i % result.ui_index.items_per_column) as f32;
            let current_column = (i / result.ui_index.items_per_column) as f32;
            let y = (label_distance_height * result.ui_index.items_per_column as f32 / 2.)
                - label_distance_height * current_row;

            // Label
            let x = x_offset + (label_width * current_column);
            let transform = Self::create_ui_transform(
                String::from("Filename"),
                x,
                y,
                label_width,
                label_height,
                i,
            );
            let text = Self::create_ui_text(filename_list[i].clone(), font_size, font.clone());
            let entity =
                Self::create_ui_entity(world, i, transform, text, FilePickerButtonType::Label);
            result.labels.push(entity);
        }

        // Editable label
        let current_filename = world.read_resource::<FilePickerFilename>().filename.clone();
        let y = (label_distance_height * result.ui_index.items_per_column as f32 / 2.)
            + label_distance_height;
        let transform = Self::create_ui_transform(
            String::from("EditableFilename"),
            0.,
            y,
            label_width * 10.,
            label_height * 10.,
            0,
        );
        let text = Self::create_ui_text(current_filename, font_size, font.clone());
        let entity = Self::create_ui_entity(
            world,
            0,
            transform,
            text,
            FilePickerButtonType::EditableLabel,
        );
        result.editable_label = Some(entity);

        return result;
    }

    fn create_ui_text(text: String, font_size: f32, font: Handle<FontAsset>) -> UiText {
        let text = UiText::new(
            font.clone(),
            format!("{}", text).to_string(),
            [1., 1., 1., 1.],
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
            Anchor::BottomMiddle,
            Anchor::Middle,
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
        i: usize,
        transform: UiTransform,
        text: UiText,
        file_picker_button_type: FilePickerButtonType,
    ) -> Entity {
        let mut entity = world
            .create_entity()
            .with(transform)
            .with(text)
            .with(FilePickerButton::new(file_picker_button_type, i));
        match file_picker_button_type {
            FilePickerButtonType::EditableLabel => {
                let mut selectable: Selectable<()> = Selectable::<()>::new(0);
                selectable.consumes_inputs = true;
                entity = entity
                    .with(TextEditing::new(40, COLOR_RED, COLOR_GOLD, true))
                    .with(Interactable)
                    .with(selectable);
            }
            _ => {}
        }
        let entity = entity.build();
        entity
    }

    fn update_ui(&mut self, world: &World) {
        self.handle_input(world);
        self.update_color(world);
    }

    fn handle_input(&mut self, world: &World) {
        let input = world.read_resource::<InputManager>();
        if input.action_single_press("start").is_down {
            let mut ui_texts = world.write_storage::<UiText>();
            let mut text = None;
            if let Some(ui_text) = ui_texts.get_mut(self.editable_label.unwrap()) {
                text = Some(ui_text.text.clone());
            }
            if let Some(text) = text {
                world.write_resource::<FilePickerFilename>().filename = text;
            }
            self.should_destroy = true;
        }

        // Don't process keyboard events if the label is selected for input
        if let Some(_selected) = world
            .read_storage::<Selected>()
            .get(self.editable_label.unwrap())
        {
            return;
        }

        let horizontal = input
            .action_single_press("horizontal")
            .excluding_modifiers(EDITOR_MODIFIERS_ALL)
            .axis;
        let vertical = input
            .action_single_press("vertical")
            .excluding_modifiers(EDITOR_MODIFIERS_ALL)
            .axis;

        if vertical > 0.0 && self.ui_index.index > 0 {
            self.ui_index.index -= 1;
        } else if vertical < 0.0 && self.ui_index.index < self.ui_index.max_index {
            self.ui_index.index += 1;
        }

        if horizontal > 0.0
            && self.ui_index.index + self.ui_index.items_per_column < self.ui_index.max_index
        {
            self.ui_index.index += self.ui_index.items_per_column;
        } else if horizontal < 0.0 && self.ui_index.index >= self.ui_index.items_per_column {
            self.ui_index.index -= self.ui_index.items_per_column;
        }

        if input.action_single_press("accept").is_down {
            let button_info =
                FilePickerButton::new(FilePickerButtonType::Label, self.ui_index.index);
            self.handle_click(world, &button_info);
        }

        if input.action_single_press("cancel").is_down {
            self.should_destroy = true;
        }
    }
}

pub struct FilePickerFilename {
    pub filename: String,
    pub full_path: String,
}

impl FilePickerFilename {
    pub fn new(filename: String, full_path: String) -> FilePickerFilename {
        FilePickerFilename {
            filename,
            full_path,
        }
    }
}

#[derive(Derivative, Clone, Copy, Debug)]
#[derivative(Default)]
pub enum FilePickerButtonType {
    #[derivative(Default)]
    Label,
    EditableLabel,
}

#[derive(Derivative, Copy, Clone, Debug)]
#[derivative(Default)]
pub struct FilePickerButton {
    pub file_picker_button_type: FilePickerButtonType,
    pub id: usize,
}

impl Component for FilePickerButton {
    type Storage = DenseVecStorage<Self>;
}

impl FilePickerButton {
    pub(crate) fn new(
        file_picker_button_type: FilePickerButtonType,
        id: usize,
    ) -> FilePickerButton {
        FilePickerButton {
            file_picker_button_type,
            id,
        }
    }
}

#[derive(Derivative, Copy, Clone)]
#[derivative(Default)]
pub struct FilePickerUiIndex {
    pub index: usize,
    pub selected_index: Option<usize>,
    pub max_index: usize,
    pub items_per_column: usize,
}

impl FilePickerUi {
    fn handle_click(&mut self, world: &World, button_info: &FilePickerButton) {
        match button_info.file_picker_button_type {
            FilePickerButtonType::EditableLabel => {
                self.ui_index.selected_index = None;
                return;
            }
            _ => {}
        }

        self.ui_index.selected_index = Some(button_info.id);
        let label_entity = self.labels[button_info.id];
        let editable_entity = self.editable_label.unwrap();

        let mut ui_texts = world.write_storage::<UiText>();
        let mut label_text = None;
        if let Some(ui_text) = ui_texts.get_mut(label_entity) {
            label_text = Some(ui_text.text.clone());
        }
        if let Some(ui_text) = ui_texts.get_mut(editable_entity) {
            if let Some(filename) = label_text {
                ui_text.text = filename;
            }
        }
    }
}
