use crate::components::game::{AnimatedTile, SpriteRenderData};
use crate::components::graphics::SpriteSheetType;
use crate::{
    components::{
        editor::{CursorState, EditorCursor, InsertionGameObject},
        game::SerializedObjectType,
        physics::Position,
    },
    states::pizzatopia::TILE_HEIGHT,
    systems::editor::EDITOR_MODIFIERS_UI,
    ui::{with_transparent, UiComponent, COLOR_BLACK},
};
use amethyst::{
    assets::AssetStorage,
    assets::Handle,
    core::HiddenPropagate,
    ecs::prelude::{Component, DenseVecStorage, Entity, Join},
    prelude::{Builder, World, WorldExt},
    renderer::SpriteSheet,
    ui::{Anchor, FontAsset, UiEvent, UiEventType, UiImage, UiText, UiTransform},
};
use derivative::Derivative;
use log::warn;
use num_traits::Zero;
use pizzatopia_input::*;
use pizzatopia_utils::EnumCycle;
use std::collections::BTreeMap;

#[derive(Derivative)]
#[derivative(Default)]
pub struct EditorFieldUiComponents {
    pub labels: Vec<Entity>,
    pub left_arrows: Vec<Entity>,
    pub right_arrows: Vec<Entity>,
    ui_index: UiIndex,
}

impl UiComponent for EditorFieldUiComponents {
    fn entities_to_remove(&self, mut to_remove: Vec<Entity>) -> Vec<Entity> {
        for entity in self
            .labels
            .iter()
            .chain(self.left_arrows.iter())
            .chain(self.right_arrows.iter())
        {
            to_remove.push(entity.clone());
        }
        to_remove
    }

    fn update(&mut self, world: &World) {
        self.update_ui(world);
    }

    fn handle_ui_events(&mut self, world: &World, event: UiEvent) {
        match &event.event_type {
            UiEventType::Click => {
                if let Some(button_info) = world.read_storage::<EditorButton>().get(event.target) {
                    self.handle_click(&world, button_info)
                }
            }
            UiEventType::HoverStart => {
                self.ui_index.active = true;
                if let Some(button_info) = world.read_storage::<EditorButton>().get(event.target) {
                    self.ui_index.index = button_info.id;
                }
            }
            UiEventType::HoverStop => {
                self.ui_index.active = false;
            }
            _ => {}
        }
    }

    fn should_capture_input(&self, world: &World) -> bool {
        let state = world.read_resource::<CursorState>();
        match *state {
            CursorState::EditGameObject | CursorState::InsertMode => true,
            CursorState::EditMode => false,
        }
    }
}

impl EditorFieldUiComponents {
    pub fn new(world: &mut World) -> Self {
        Self::initialize_ui(world)
    }

    pub fn hide_components(&mut self, world: &World, first: usize, last: usize) {
        for i in first..=last {
            let comp = self.labels[i];
            world
                .write_storage::<HiddenPropagate>()
                .insert(comp.clone(), HiddenPropagate::new())
                .unwrap();
            let comp = self.left_arrows[i];
            world
                .write_storage::<HiddenPropagate>()
                .insert(comp.clone(), HiddenPropagate::new())
                .unwrap();
            let comp = self.right_arrows[i];
            world
                .write_storage::<HiddenPropagate>()
                .insert(comp.clone(), HiddenPropagate::new())
                .unwrap();
        }
    }

    pub fn show_components(&mut self, world: &World, first: usize, last: usize) {
        for i in first..=last {
            let comp = self.labels[i];
            world
                .write_storage::<HiddenPropagate>()
                .remove(comp.clone());
            let comp = self.left_arrows[i];
            world
                .write_storage::<HiddenPropagate>()
                .remove(comp.clone());
            let comp = self.right_arrows[i];
            world
                .write_storage::<HiddenPropagate>()
                .remove(comp.clone());
        }
    }

    fn update_color(&self, world: &World) {
        let mut ui_texts = world.write_storage::<UiText>();
        for i in 0..self.labels.len() {
            let color = match i == self.ui_index.index && self.ui_index.active {
                true => [1., 0., 0., 1.],
                false => [0.75, 0.75, 0.75, 0.75],
            };
            let entities = [self.labels[i], self.left_arrows[i], self.right_arrows[i]];
            for entity in entities.iter() {
                if let Some(ui_text) = ui_texts.get_mut(*entity) {
                    ui_text.color = color;
                }
            }
        }
    }

    fn initialize_ui(world: &mut World) -> Self {
        let font = (*world.read_resource::<Handle<FontAsset>>()).clone();

        let width = 200.0;
        let font_size = 18.;
        let arrow_font_size = font_size * 1.5;
        let arrow_width = arrow_font_size;

        let mut result: EditorFieldUiComponents = EditorFieldUiComponents::default();
        for i in 0..10 {
            let height = 25.0;
            let y = -50. + -(height * 1.2) * i as f32;

            // Label
            let x = width / 2.0 + arrow_width * 2.;
            let transform =
                Self::create_ui_transform(String::from("Label"), x, y, width, height, i);
            let text = Self::create_ui_text(String::from("DEFAULT TEXT"), font_size, font.clone());
            let entity = Self::create_ui_entity(world, i, transform, text, EditorButtonType::Label);
            result.labels.push(entity);

            // Right Arrow
            let x = width + arrow_width * 3.;
            let transform =
                Self::create_ui_transform(String::from("ArrowR"), x, y, arrow_width, height, i);
            let text = Self::create_ui_text(String::from(">>"), arrow_font_size, font.clone());
            let entity =
                Self::create_ui_entity(world, i, transform, text, EditorButtonType::RightArrow);
            result.right_arrows.push(entity);

            // Left Arrow
            let x = font_size;
            let transform =
                Self::create_ui_transform(String::from("ArrowL"), x, y, arrow_width, height, i);
            let text = Self::create_ui_text(String::from("<<"), arrow_font_size, font.clone());
            let entity =
                Self::create_ui_entity(world, i, transform, text, EditorButtonType::LeftArrow);
            result.left_arrows.push(entity);
        }
        return result;
    }

    fn create_ui_text(text: String, font_size: f32, font: Handle<FontAsset>) -> UiText {
        let mut text = UiText::new(
            font,
            format!("{}", text).to_string(),
            [1., 1., 1., 1.],
            font_size,
        );
        text.align = Anchor::Middle;
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
            Anchor::TopLeft,
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
        editor_button_type: EditorButtonType,
    ) -> Entity {
        let entity = world
            .create_entity()
            .with(transform)
            .with(text)
            .with(EditorButton::new(editor_button_type, i))
            .with(HiddenPropagate::new())
            .with(UiImage::SolidColor(with_transparent(COLOR_BLACK, 0.8)))
            .build();
        entity
    }

    fn update_ui(&mut self, world: &World) {
        self.handle_input(world);
        self.update_color(world);

        let state = (*world.read_resource::<CursorState>()).clone();
        match state {
            CursorState::EditMode => {
                self.hide_components(world, 0, 9);
            }
            CursorState::EditGameObject | CursorState::InsertMode => {
                self.update_ui_text_general_properties(world);
                self.update_ui_text_object_specific_properties(world);
            }
        }
    }

    fn handle_input(&mut self, world: &World) {
        let state = world.read_resource::<CursorState>();
        let input = world.read_resource::<Input<StringBindings>>();
        match *state {
            CursorState::EditGameObject | CursorState::InsertMode => {
                let horizontal = input
                    .axes
                    .repeat_press("horizontal".to_string(), 250, 10)
                    .axis;
                let vertical = input
                    .axes
                    .repeat_press("vertical".to_string(), 250, 10)
                    .axis;
                if input.actions.status("modifier1".to_string()).is_down {
                    self.ui_index.active = true;
                    if vertical > 0.0 && self.ui_index.index > 0 {
                        self.ui_index.index -= 1;
                    } else if vertical < 0.0 {
                        self.ui_index.index += 1;
                    }

                    let mut arrow_direction = EditorButtonType::Label;
                    if horizontal > 0.0 {
                        arrow_direction = EditorButtonType::RightArrow;
                    } else if horizontal < 0.0 {
                        arrow_direction = EditorButtonType::LeftArrow;
                    }
                    match arrow_direction {
                        EditorButtonType::RightArrow | EditorButtonType::LeftArrow => {
                            let button_action =
                                EditorButton::new(arrow_direction, self.ui_index.index);
                            self.handle_click(world, &button_action);
                        }
                        _ => {}
                    }
                } else if input.actions.just_released("modifier1".to_string()) {
                    self.ui_index.active = false;
                }
            }
            _ => {}
        }
    }

    fn update_ui_text_object_specific_properties(&mut self, world: &World) {
        let insertion = *world.read_resource::<InsertionGameObject>().clone();
        let mut ui_text_storage = world.write_storage::<UiText>();
        self.show_components(world, 0, 9);
        let mut counter = 4;

        // Object-specific properties
        match insertion.0.object_type {
            SerializedObjectType::StaticTile { animation } => {
                if let Some(text) = ui_text_storage.get_mut(self.labels[counter]) {
                    text.text = format!("Animated: {}", animation.is_some());
                    counter += 1;
                }
                if animation.is_some() {
                    let animated = animation.unwrap();
                    if let Some(text) = ui_text_storage.get_mut(self.labels[counter]) {
                        text.text = format!("Animation length: {}", animated.num_frames);
                        counter += 1;
                    }
                    if let Some(text) = ui_text_storage.get_mut(self.labels[counter]) {
                        text.text = format!("Animation speed: {}", animated.time_per_frame);
                        counter += 1;
                    }
                }
            }
            SerializedObjectType::Player { is_player } => {
                if let Some(text) = ui_text_storage.get_mut(self.labels[counter]) {
                    text.text = format!("Player-controlled: {}", is_player.0);
                    counter += 1;
                }
            }
        }
        self.hide_components(world, counter, 9);
        self.ui_index.index = self.ui_index.index.max(0).min(counter - 1);
    }

    fn update_ui_text_general_properties(&mut self, world: &World) {
        let insertion = world.read_resource::<InsertionGameObject>();
        let mut ui_text_storage = world.write_storage::<UiText>();
        if let Some(text) = ui_text_storage.get_mut(self.labels[0]) {
            if let Some(size) = insertion.0.size {
                text.text = format!("Width: {:?}", size.x / TILE_HEIGHT);
            }
        }
        if let Some(text) = ui_text_storage.get_mut(self.labels[1]) {
            if let Some(size) = insertion.0.size {
                text.text = format!("Height: {:?}", size.y / TILE_HEIGHT);
            }
        }
        if let Some(text) = ui_text_storage.get_mut(self.labels[2]) {
            if let Some(sprite) = insertion.0.sprite {
                text.text = format!("Sprite sheet: {:?}", sprite.sheet);
            }
        }
        if let Some(text) = ui_text_storage.get_mut(self.labels[3]) {
            if let Some(sprite) = insertion.0.sprite {
                text.text = format!("Sprite number: {}", sprite.number);
            }
        }
    }
}

#[derive(Derivative, Clone, Copy, Debug)]
#[derivative(Default)]
pub enum EditorButtonType {
    #[derivative(Default)]
    Label,
    RightArrow,
    LeftArrow,
}

#[derive(Derivative, Copy, Clone, Debug)]
#[derivative(Default)]
pub struct EditorButton {
    pub editor_button_type: EditorButtonType,
    pub id: usize,
}

impl Component for EditorButton {
    type Storage = DenseVecStorage<Self>;
}

impl EditorButton {
    pub(crate) fn new(editor_button_type: EditorButtonType, id: usize) -> EditorButton {
        EditorButton {
            editor_button_type,
            id,
        }
    }
}

#[derive(Derivative, Copy, Clone)]
#[derivative(Default)]
pub struct UiIndex {
    pub index: usize,
    pub active: bool,
}

fn sprite_max(world: &World, sprite: &SpriteRenderData) -> usize {
    let sheet = {
        let sprite_sheets = &world.read_resource::<BTreeMap<u8, Handle<SpriteSheet>>>();
        let sheet = sprite_sheets.get(&(sprite.sheet as u8)).unwrap();
        sheet.clone()
    };

    let sheets = &world.read_resource::<AssetStorage<SpriteSheet>>();
    if let Some(sheet) = sheets.get(&sheet) {
        sheet.sprites.len()
    } else {
        0
    }
}

impl EditorFieldUiComponents {
    fn handle_click(&mut self, world: &World, button_info: &EditorButton) {
        let positions = &mut world.write_storage::<Position>();
        let entities = &world.entities();
        let cursors = &world.read_storage::<EditorCursor>();
        let insertion_serialized_object = &mut world.write_resource::<InsertionGameObject>();

        self.ui_index.index = button_info.id;
        self.ui_index.active = true;
        const START_ID: usize = 4;
        let sprite_render = insertion_serialized_object.0.sprite.clone();
        match button_info.id {
            0..=1 => {
                let is_x_axis = button_info.id == 0;
                for (pos, _, _) in (positions, entities, cursors).join() {
                    let mut position = pos.0;
                    match button_info.editor_button_type {
                        EditorButtonType::RightArrow => {
                            insertion_serialized_object
                                .0
                                .next_size(&mut position, is_x_axis);
                        }
                        EditorButtonType::LeftArrow => {
                            insertion_serialized_object
                                .0
                                .prev_size(&mut position, is_x_axis);
                        }
                        EditorButtonType::Label => {}
                    }
                    pos.0.x = position.x;
                    pos.0.y = position.y;
                }
            }
            2 => {
                if let Some(ref mut sprite) = insertion_serialized_object.0.sprite {
                    sprite.sheet = match button_info.editor_button_type {
                        EditorButtonType::Label => sprite.sheet,
                        EditorButtonType::RightArrow => sprite.sheet.next(),
                        EditorButtonType::LeftArrow => sprite.sheet.prev(),
                    };
                }
            }
            3 => {
                if let Some(ref mut sprite) = insertion_serialized_object.0.sprite {
                    sprite.number = match button_info.editor_button_type {
                        EditorButtonType::Label => sprite.number,
                        EditorButtonType::RightArrow => {
                            (sprite.number + 1).min(sprite_max(world, sprite) - 1)
                        }
                        EditorButtonType::LeftArrow => {
                            if !sprite.number.is_zero() {
                                sprite.number - 1
                            } else {
                                sprite.number
                            }
                        }
                    };
                }
            }
            _ => {}
        }
        match insertion_serialized_object.0.object_type {
            SerializedObjectType::StaticTile { ref mut animation } => {
                match button_info.editor_button_type {
                    EditorButtonType::Label => {}
                    EditorButtonType::RightArrow | EditorButtonType::LeftArrow => {
                        let sign = match button_info.editor_button_type {
                            EditorButtonType::LeftArrow => -1,
                            _ => 1,
                        };
                        if button_info.id == START_ID {
                            if animation.is_some() {
                                *animation = None;
                            } else {
                                *animation = Some(AnimatedTile::default());
                            }
                        } else if button_info.id == START_ID + 1 && animation.is_some() {
                            // NUM ANIMATION TILES
                            if let Some(ref mut anim) = *animation {
                                let mut result = sign + anim.num_frames as i32;
                                if let Some(sprite) = &sprite_render {
                                    let max = sprite_max(world, sprite) as i32
                                        - (sprite.number as i32 + 1);
                                    result = result.max(0).min(max);
                                }
                                anim.num_frames = result as usize;
                            }
                        } else if button_info.id == START_ID + 2 && animation.is_some() {
                            // ANIMATION LEN
                            if let Some(ref mut anim) = *animation {
                                let mut result = anim.time_per_frame as f32;
                                result += sign as f32 * 0.1;
                                anim.time_per_frame = result.max(0.);
                            }
                        }
                    }
                }
            }
            SerializedObjectType::Player { ref mut is_player } => {
                if button_info.id == START_ID {
                    match button_info.editor_button_type {
                        EditorButtonType::Label => {}
                        EditorButtonType::RightArrow | EditorButtonType::LeftArrow => {
                            is_player.0 = !is_player.0;
                        }
                    }
                }
            }
        }
    }
}
