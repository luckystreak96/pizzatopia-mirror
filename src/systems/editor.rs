use amethyst::core::shrev::{EventChannel, ReaderId};
use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::{Entities, Entity};
use amethyst::ecs::{
    Join, NullStorage, Read, ReadStorage, System, SystemData, World, Write, WriteStorage,
};
use amethyst::input::{InputHandler, StringBindings};
use amethyst::renderer::debug_drawing::{DebugLines, DebugLinesComponent, DebugLinesParams};
use amethyst::renderer::palette::Srgba;

use crate::components::editor::{
    CursorWasInThisEntity, EditorButton, EditorButtonType, EditorCursor, EditorCursorState,
    EditorState, InsertionGameObject, InstanceEntityId, RealCursorPosition, SizeForEditorGrid,
};
use crate::components::game::{Health, SerializedObjectType};
use crate::components::game::{Player, SerializedObject};
use crate::components::graphics::{Scale, SpriteSheetType};
use crate::components::physics::{GravityDirection, Grounded, PlatformCuboid, Position, Velocity};
use crate::events::Events;
use crate::level::Level;
use crate::states::editor::EDITOR_GRID_SIZE;
use crate::states::pizzatopia::{CAM_HEIGHT, DEPTH_UI, TILE_HEIGHT, TILE_WIDTH};
use crate::systems::input::{InputManager, REPEAT_DELAY};
use crate::systems::physics::{
    gravitationally_adapted_velocity, gravitationally_de_adapted_velocity, ActorCollisionSystem,
};
use crate::utils::{Vec2, Vec3};
use amethyst::assets::Handle;
use amethyst::ecs::prelude::ReadExpect;
use amethyst::prelude::WorldExt;
use amethyst::renderer::SpriteSheet;
use log::{error, info, warn};
use num_traits::Zero;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum EditorEvents {
    AddGameObject,
    RemoveGameObject,
    SaveLevelToFile,
    ChangeInsertionGameObject(u8),
    SetInsertionGameObject(SerializedObject),
    ChangeState(EditorState),
    UiClick(EditorButton),
}

pub fn align_cursor_position_with_grid(position: &mut Vec2, size: &Vec2) {
    let offset_x = (position.x.abs() + (size.x / 2.0)) % EDITOR_GRID_SIZE;
    let offset_y = (position.y.abs() + (size.y / 2.0)) % EDITOR_GRID_SIZE;
    if offset_x != 0.0 || offset_y != 0.0 {
        let backup = position.clone();
        if size.x % TILE_WIDTH == 0.0 {
            snap_cursor_position_to_grid_corner(position);
        } else {
            snap_cursor_position_to_grid_center(position);
        }
        position.y = backup.y;

        let backup = position.clone();
        if size.y % TILE_HEIGHT == 0.0 {
            snap_cursor_position_to_grid_corner(position);
        } else {
            snap_cursor_position_to_grid_center(position);
        }
        position.x = backup.x;
    }
}

fn snap_cursor_position_to_grid_center(position: &mut Vec2) {
    position.x -= (position.x.abs() % EDITOR_GRID_SIZE) - EDITOR_GRID_SIZE / 2.0;
    position.y -= (position.y.abs() % EDITOR_GRID_SIZE) - EDITOR_GRID_SIZE / 2.0;
}

fn snap_cursor_position_to_grid_corner(position: &mut Vec2) {
    position.x -= position.x.abs() % EDITOR_GRID_SIZE;
    position.y -= position.y.abs() % EDITOR_GRID_SIZE;
}

//(&positions, &size_for_editor, &entities, !&cursors).join()
fn get_tile_at_position(
    pos: &Vec2,
    positions: &WriteStorage<Position>,
    size_for_editor: &ReadStorage<SizeForEditorGrid>,
    cursors: &ReadStorage<EditorCursor>,
    entities: &Entities,
) -> Option<(Vec3, Vec2, u32)> {
    for (position, size, entity, _) in (positions, size_for_editor, entities, !cursors).join() {
        let cuboid = PlatformCuboid::create(size.0.x, size.0.y);
        if cuboid.intersects_point(pos, &position.0.to_vec2()) {
            return Some((position.0.clone(), size.0.clone(), entity.id()));
        }
    }
    return None;
}

#[derive(SystemDesc)]
pub struct CursorPositionSystem;

impl<'s> System<'s> for CursorPositionSystem {
    type SystemData = (
        ReadExpect<'s, EditorState>,
        Write<'s, DebugLines>,
        Read<'s, InputManager>,
        WriteStorage<'s, Position>,
        ReadStorage<'s, EditorCursor>,
        WriteStorage<'s, RealCursorPosition>,
        ReadStorage<'s, SizeForEditorGrid>,
        WriteStorage<'s, CursorWasInThisEntity>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (
            editor_state,
            mut debug_lines_resource,
            input,
            mut positions,
            cursors,
            mut real_positions,
            size_for_editor,
            mut prev_block,
            entities,
        ): Self::SystemData,
    ) {
        let mut cursor_position = Vec2::new(0.0, 0.0);
        let mut prev_block_id: u32 = 999999;
        for (_, cursor_pos, prev_block) in (&cursors, &mut real_positions, &prev_block).join() {
            prev_block_id = prev_block.0.unwrap_or(999999);
            snap_cursor_position_to_grid_center(&mut cursor_pos.0);
            cursor_position = cursor_pos.0.clone();
        }

        let vertical_move = input.axis_repeat_press("vertical_move", REPEAT_DELAY, 2);
        let horizontal_move = input.axis_repeat_press("horizontal_move", REPEAT_DELAY, 2);

        let cursor_is_moving = horizontal_move != 0.0 || vertical_move != 0.0;
        let mut new_cursor_display_pos;
        let mut new_tile_id: Option<u32>;
        loop {
            new_tile_id = None;
            new_cursor_display_pos = cursor_position;

            cursor_position.x += horizontal_move * EDITOR_GRID_SIZE;
            cursor_position.y += vertical_move * EDITOR_GRID_SIZE;

            let tile_at_cursor = get_tile_at_position(
                &cursor_position,
                &positions,
                &size_for_editor,
                &cursors,
                &entities,
            );

            if let Some((position, _size, tile_at_cursor_id)) = tile_at_cursor {
                new_cursor_display_pos = position.to_vec2();
                new_tile_id = Some(tile_at_cursor_id);

                if prev_block_id == tile_at_cursor_id && cursor_is_moving {
                    continue;
                }
            }
            break;
        }

        for (mut display_position, _, cursor_pos, mut previous) in (
            &mut positions,
            &cursors,
            &mut real_positions,
            &mut prev_block,
        )
            .join()
        {
            match *editor_state {
                EditorState::EditMode => {
                    display_position.0.x = new_cursor_display_pos.x;
                    display_position.0.y = new_cursor_display_pos.y;
                    cursor_pos.0.x = cursor_position.x;
                    cursor_pos.0.y = cursor_position.y;
                }
                _ => {
                    display_position.0.x += horizontal_move * EDITOR_GRID_SIZE;
                    display_position.0.y += vertical_move * EDITOR_GRID_SIZE;
                    cursor_pos.0.x += horizontal_move * EDITOR_GRID_SIZE;
                    cursor_pos.0.y += vertical_move * EDITOR_GRID_SIZE;
                }
            }

            previous.0 = new_tile_id.clone();

            // TODO : Draw debug lines in separate system
            debug_lines_resource.draw_line(
                [cursor_pos.0.x - 16.0, cursor_pos.0.y + 16.0, DEPTH_UI].into(),
                [cursor_pos.0.x + 16.0, cursor_pos.0.y - 16.0, DEPTH_UI].into(),
                Srgba::new(0.1, 0.1, 0.4, 1.0),
            );

            debug_lines_resource.draw_line(
                [cursor_pos.0.x + 16.0, cursor_pos.0.y + 16.0, DEPTH_UI].into(),
                [cursor_pos.0.x - 16.0, cursor_pos.0.y - 16.0, DEPTH_UI].into(),
                Srgba::new(0.1, 0.1, 0.4, 1.0),
            );

            // Draw axis
            debug_lines_resource.draw_line(
                [-10.0, 0.0, DEPTH_UI].into(),
                [10000.0, 0.0, DEPTH_UI].into(),
                Srgba::new(1.0, 0.9, 0.4, 1.0),
            );
            debug_lines_resource.draw_line(
                [0.0, -10.0, DEPTH_UI].into(),
                [0.0, 10000.0, DEPTH_UI].into(),
                Srgba::new(1.0, 0.9, 0.4, 1.0),
            );
        }
    }
}

#[derive(SystemDesc)]
pub struct CursorStateSystem;

impl<'s> System<'s> for CursorStateSystem {
    type SystemData = (
        ReadExpect<'s, EditorState>,
        WriteStorage<'s, EditorCursor>,
        ReadStorage<'s, SizeForEditorGrid>,
        ReadStorage<'s, Position>,
    );

    fn run(&mut self, (editor_state, mut cursors, size_for_editor, positions): Self::SystemData) {
        let state: EditorState = editor_state.clone();
        let mut cursor_pos = Vec2::default();
        let mut cursor_size = Vec2::default();
        let mut cursor_state = EditorCursorState::Normal;
        // Get cursor stats
        for (pos, size, _cursor) in (&positions, &size_for_editor, &cursors).join() {
            cursor_pos = pos.0.to_vec2();
            cursor_size = size.0.clone();
        }
        let half_size = Vec2::new(cursor_size.x / 2.0, cursor_size.y / 2.0);
        let tl1 = Vec2::new(cursor_pos.x - half_size.x, cursor_pos.y + half_size.y);
        let br1 = Vec2::new(cursor_pos.x + half_size.x, cursor_pos.y - half_size.y);
        // Figure out the cursor state
        match state {
            EditorState::EditMode => {
                // change the cursor state to NOT_OVERLAPPING
                cursor_state = EditorCursorState::Normal;
            }
            EditorState::EditGameObject | EditorState::InsertMode => {
                // change the cursor state to overlap if it overlaps
                for (pos, size, _) in (&positions, &size_for_editor, !&cursors).join() {
                    let half_size = Vec2::new(size.0.x / 2.0, size.0.y / 2.0);
                    let tl2 = Vec2::new(pos.0.x - half_size.x, pos.0.y + half_size.y);
                    let br2 = Vec2::new(pos.0.x + half_size.x, pos.0.y - half_size.y);
                    if ActorCollisionSystem::cuboid_intersection(&tl1, &br1, &tl2, &br2) {
                        cursor_state = EditorCursorState::Error;
                        break;
                    }
                }
            }
        }
        // update cursor state
        for (cursor, _) in (&mut cursors, &positions).join() {
            cursor.state = cursor_state;
        }
    }
}

#[derive(SystemDesc)]
pub struct CursorSizeSystem;

impl<'s> System<'s> for CursorSizeSystem {
    type SystemData = (
        ReadExpect<'s, EditorState>,
        ReadExpect<'s, InsertionGameObject>,
        ReadStorage<'s, EditorCursor>,
        WriteStorage<'s, SizeForEditorGrid>,
        ReadStorage<'s, CursorWasInThisEntity>,
        WriteStorage<'s, Scale>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (
            editor_state,
            insertion_serialized_object,
            cursors,
            mut size_for_editor,
            previous_block,
            mut scales,
            entities,
        ): Self::SystemData,
    ) {
        let state: EditorState = editor_state.clone();
        match state {
            EditorState::EditMode => {
                for (prev_block, _) in (&previous_block, &cursors).join() {
                    if let Some(id) = prev_block.0 {
                        let entity = entities.entity(id);
                        if let Some(size) = size_for_editor.get(entity) {
                            for (scale, _) in (&mut scales, &cursors).join() {
                                let scale_size =
                                    Vec2::new(size.0.x / TILE_WIDTH, size.0.y / TILE_HEIGHT);
                                scale.0 = scale_size;
                            }
                        }
                    } else {
                        for (scale, _) in (&mut scales, &cursors).join() {
                            let scale_size = Vec2::new(
                                EDITOR_GRID_SIZE / TILE_WIDTH,
                                EDITOR_GRID_SIZE / TILE_HEIGHT,
                            );
                            scale.0 = scale_size;
                        }
                    }
                }
            }
            EditorState::EditGameObject | EditorState::InsertMode => {
                for (scale, size, _) in (&mut scales, &mut size_for_editor, &cursors).join() {
                    let ser_size = insertion_serialized_object
                        .0
                        .size
                        .unwrap_or(Vec2::new(TILE_WIDTH, TILE_HEIGHT));
                    size.0 = ser_size.clone();
                    scale.0 = ser_size.clone();
                    scale.0.x /= TILE_WIDTH;
                    scale.0.y /= TILE_HEIGHT;
                }
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct EditorButtonEventSystem {
    input_last_press: BTreeMap<String, (bool, Instant)>,
}

impl EditorButtonEventSystem {
    fn update_input(&mut self, input: &InputHandler<StringBindings>) {
        for action in input.bindings.actions() {
            let elapsed = self.input_last_press.get(action).unwrap().1.elapsed();
            if input.action_is_down(action).unwrap() && elapsed.as_millis() > 250 {
                self.input_last_press
                    .insert(action.clone(), (true, Instant::now()));
            } else {
                self.input_last_press.get_mut(action).unwrap().0 = false;
            }
        }
    }

    fn action_down(&self, action: &str) -> bool {
        if let Some((pressed, _time)) = self.input_last_press.get(action) {
            *pressed
        } else {
            false
        }
    }

    pub(crate) fn new(world: &World) -> Self {
        let mut input_last_press = BTreeMap::new();
        for action in world
            .read_resource::<InputHandler<StringBindings>>()
            .bindings
            .actions()
        {
            input_last_press.insert(action.clone(), (false, Instant::now()));
        }

        EditorButtonEventSystem { input_last_press }
    }
}

impl<'s> System<'s> for EditorButtonEventSystem {
    type SystemData = (
        ReadExpect<'s, EditorState>,
        Read<'s, InputHandler<StringBindings>>,
        Write<'s, EventChannel<EditorEvents>>,
    );

    fn run(&mut self, (state, input, mut editor_event_writer): Self::SystemData) {
        self.update_input(&input);

        if self.action_down("save") {
            editor_event_writer.single_write(EditorEvents::SaveLevelToFile);
        }

        match *state {
            EditorState::EditMode => {
                // Controller input
                if self.action_down("cancel") {
                    editor_event_writer.single_write(EditorEvents::RemoveGameObject);
                } else if self.action_down("accept") {
                    editor_event_writer
                        .single_write(EditorEvents::ChangeState(EditorState::EditGameObject));
                } else if self.action_down("insert") {
                    editor_event_writer
                        .single_write(EditorEvents::ChangeState(EditorState::InsertMode));
                }
            }
            EditorState::InsertMode => {
                // Controller input
                if self.action_down("cancel") {
                    editor_event_writer
                        .single_write(EditorEvents::ChangeState(EditorState::EditMode));
                } else if self.action_down("accept") {
                    editor_event_writer.single_write(EditorEvents::AddGameObject);
                } else if self.action_down("1") {
                    editor_event_writer.single_write(EditorEvents::ChangeInsertionGameObject(0));
                } else if self.action_down("2") {
                    editor_event_writer.single_write(EditorEvents::ChangeInsertionGameObject(1));
                }
            }
            EditorState::EditGameObject => {
                if self.action_down("cancel") {
                    editor_event_writer
                        .single_write(EditorEvents::ChangeState(EditorState::EditMode));
                } else if self.action_down("accept") {
                    editor_event_writer.single_write(EditorEvents::AddGameObject);
                    editor_event_writer
                        .single_write(EditorEvents::ChangeState(EditorState::EditMode));
                }
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct EditorEventHandlingSystem {
    reader: ReaderId<EditorEvents>,
}

impl EditorEventHandlingSystem {
    pub fn new(world: &mut World) -> Self {
        <Self as System<'_>>::SystemData::setup(world);
        let reader = world
            .fetch_mut::<EventChannel<EditorEvents>>()
            .register_reader();
        Self { reader }
    }
}

impl<'s> System<'s> for EditorEventHandlingSystem {
    type SystemData = (
        Read<'s, EventChannel<EditorEvents>>,
        Write<'s, EventChannel<Events>>,
        Write<'s, EditorState>,
        Write<'s, InsertionGameObject>,
        ReadStorage<'s, EditorCursor>,
        WriteStorage<'s, Position>,
        WriteStorage<'s, CursorWasInThisEntity>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (
            editor_event_channel,
            mut world_events_channel,
            mut editor_state,
            mut insertion_serialized_object,
            cursors,
            mut positions,
            previous_block,
            entities,
        ): Self::SystemData,
    ) {
        // If an event in the queue gets cancelled, interrupt events that go after it
        // this is used for error-adding a tile in EditGameObject mode
        let mut cancel_others = false;
        for event in editor_event_channel.read(&mut self.reader) {
            if cancel_others {
                continue;
            }
            match event {
                // Writing an event here is fine - entities are created lazily (only at frame end)
                // May as well use World and save the trouble for the tile creation
                // https://book.amethyst.rs/master/concepts/system.html?highlight=create#creating-new-entities-in-a-system
                EditorEvents::AddGameObject => {
                    for (cursor, position) in (&cursors, &positions).join() {
                        // We only add the GameObject if the cursor isn't currently in a tile
                        match cursor.state {
                            EditorCursorState::Normal => {
                                let pos = position.0.to_vec2();
                                insertion_serialized_object.0.pos = Some(pos);
                                world_events_channel.single_write(Events::AddGameObject);
                            }
                            EditorCursorState::Error => {
                                cancel_others = true;
                            }
                        }
                    }
                }
                EditorEvents::RemoveGameObject => {
                    for (_cursor, previous_block) in (&cursors, &previous_block).join() {
                        if let Some(id) = previous_block.0 {
                            world_events_channel.single_write(Events::DeleteGameObject(id));
                        }
                    }
                }
                EditorEvents::SaveLevelToFile => {
                    world_events_channel.single_write(Events::SaveLevel);
                }
                EditorEvents::ChangeInsertionGameObject(id) => {
                    world_events_channel.single_write(Events::ChangeInsertionGameObject(*id));
                }
                EditorEvents::SetInsertionGameObject(serialized_object) => {
                    world_events_channel
                        .single_write(Events::SetInsertionGameObject(serialized_object.clone()));
                }
                EditorEvents::ChangeState(new_state) => {
                    let mut change = true;
                    match new_state {
                        EditorState::EditGameObject => {
                            for (_cursor, previous_block) in (&cursors, &previous_block).join() {
                                // Change state if selecting block
                                if previous_block.0.is_none() {
                                    change = false;
                                } else {
                                    let id = previous_block.0.unwrap();
                                    world_events_channel
                                        .single_write(Events::EntityToInsertionGameObject(id));
                                    world_events_channel.single_write(Events::DeleteGameObject(id));
                                }
                            }
                        }
                        EditorState::InsertMode => {
                            for (position, _cursor) in (&mut positions, &cursors).join() {
                                let mut pos: Vec2 = position.0.to_vec2();
                                snap_cursor_position_to_grid_corner(&mut pos);
                                let mut size = Vec2::new(TILE_WIDTH, TILE_HEIGHT);
                                size = insertion_serialized_object.0.size.unwrap_or(size);
                                pos.x += size.x / 2.0;
                                pos.y += size.y / 2.0;
                                position.0.x = pos.x;
                                position.0.y = pos.y;
                            }
                        }
                        _ => {}
                    }
                    if change {
                        *editor_state = new_state.clone();
                    }
                }
                EditorEvents::UiClick(button_info) => {
                    let start_id = 4;
                    match button_info.id {
                        0..=1 => {
                            let is_x_axis = button_info.id == 0;
                            for (pos, _, _) in (&mut positions, &entities, &cursors).join() {
                                let mut position = pos.0.to_vec2();
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
                                    EditorButtonType::RightArrow => sprite.number + 1,
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
                    warn!("{:?}", button_info);
                    match insertion_serialized_object.0.object_type {
                        SerializedObjectType::StaticTile => {}
                        SerializedObjectType::Player { ref mut is_player } => {
                            if button_info.id == start_id {
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
            };
        }
    }
}
