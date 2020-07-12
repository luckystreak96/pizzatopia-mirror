use amethyst::{
    core::{
        shrev::{EventChannel, ReaderId},
        SystemDesc, Transform,
    },
    derive::SystemDesc,
    ecs::{
        Entities, Entity, Join, NullStorage, Read, ReadStorage, System, SystemData, World, Write,
        WriteStorage,
    },
    input::{InputHandler, StringBindings},
    renderer::{
        debug_drawing::{DebugLines, DebugLinesComponent, DebugLinesParams},
        palette::Srgba,
    },
};

use crate::{
    components::{
        editor::{
            CursorState, CursorWasInThisEntity, EditorCursor, EditorCursorState,
            InsertionGameObject, InstanceEntityId, RealCursorPosition, SizeForEditorGrid,
        },
        game::{Health, Player, SerializedObject, SerializedObjectType},
        graphics::{Scale, SpriteSheetType},
        physics::{GravityDirection, Grounded, PlatformCuboid, Position, Velocity},
    },
    events::Events,
    level::Level,
    states::{
        editor::EDITOR_GRID_SIZE,
        pizzatopia::{CAM_HEIGHT, DEPTH_UI, TILE_HEIGHT, TILE_WIDTH},
    },
    systems::{
        input::{InputManager, REPEAT_DELAY},
        physics::{
            gravitationally_adapted_velocity, gravitationally_de_adapted_velocity,
            ActorCollisionSystem,
        },
    },
    ui::{
        tile_characteristics::{EditorButton, EditorButtonType, UiIndex},
        UiStack,
    },
    utils::{Vec2, Vec3},
};
use amethyst::{assets::Handle, ecs::prelude::ReadExpect, renderer::SpriteSheet};

use crate::components::editor::TileLayer;
use log::info;
use num_traits::Zero;
use pizzatopia_utils::EnumCycle;

pub const EDITOR_MODIFIERS_ALL: &[&str] = &["modifier1", "modifier2"];
pub const EDITOR_MODIFIERS_UI: &[&str] = &["modifier1"];

#[derive(Debug)]
pub enum EditorEvents {
    AddGameObject,
    RemoveGameObject,
    SaveLevelToFile,
    ChangeInsertionGameObject(u8),
    SetInsertionGameObject(SerializedObject),
    ChangeState(CursorState),
    UiClick(EditorButton),
    CycleActiveLayer(bool),
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
        Write<'s, EventChannel<Events>>,
        ReadExpect<'s, CursorState>,
        Write<'s, DebugLines>,
        Read<'s, InputManager>,
        Read<'s, UiStack>,
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
            mut event_channel,
            cursor_state,
            mut debug_lines_resource,
            input,
            ui,
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

        let mut vertical_move = input
            .is_valid_repeat_press("vertical", REPEAT_DELAY, 2)
            .excluding_modifiers(EDITOR_MODIFIERS_ALL)
            .axis;
        let mut horizontal_move = input
            .is_valid_repeat_press("horizontal", REPEAT_DELAY, 2)
            .excluding_modifiers(EDITOR_MODIFIERS_ALL)
            .axis;
        if ui.is_blocking_all_input() {
            vertical_move = 0.;
            horizontal_move = 0.;
        }
        if !vertical_move.is_zero() {
            vertical_move = vertical_move / vertical_move.abs();
        }
        if !horizontal_move.is_zero() {
            horizontal_move = horizontal_move / horizontal_move.abs();
        }

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
            match *cursor_state {
                CursorState::EditMode => {
                    display_position.0.x = new_cursor_display_pos.x;
                    display_position.0.y = new_cursor_display_pos.y;
                    cursor_pos.0.x = cursor_position.x;
                    cursor_pos.0.y = cursor_position.y;

                    if new_tile_id.is_some() {
                        event_channel.single_write(Events::HoverGameObject);
                    }
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
        ReadExpect<'s, CursorState>,
        WriteStorage<'s, EditorCursor>,
        ReadStorage<'s, SizeForEditorGrid>,
        ReadStorage<'s, Position>,
    );

    fn run(&mut self, (cursor_state, mut cursors, size_for_editor, positions): Self::SystemData) {
        let state: CursorState = cursor_state.clone();
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
            CursorState::EditMode => {
                // change the cursor state to NOT_OVERLAPPING
                cursor_state = EditorCursorState::Normal;
            }
            CursorState::EditGameObject | CursorState::InsertMode => {
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
        ReadExpect<'s, CursorState>,
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
            cursor_state,
            insertion_serialized_object,
            cursors,
            mut size_for_editor,
            previous_block,
            mut scales,
            entities,
        ): Self::SystemData,
    ) {
        let state: CursorState = cursor_state.clone();
        match state {
            CursorState::EditMode => {
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
            CursorState::EditGameObject | CursorState::InsertMode => {
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
pub struct EditorButtonEventSystem;

impl<'s> System<'s> for EditorButtonEventSystem {
    type SystemData = (
        ReadExpect<'s, CursorState>,
        Read<'s, InputManager>,
        Read<'s, UiStack>,
        Write<'s, EventChannel<EditorEvents>>,
        Write<'s, EventChannel<Events>>,
    );

    fn run(
        &mut self,
        (state, input, ui, mut editor_event_writer, mut global_event_writer): Self::SystemData,
    ) {
        if ui.is_blocking_all_input() {
            return;
        }

        let vertical_with_modifier2 = input
            .action_single_press("vertical")
            .including_modifiers(&["modifier2"])
            .axis;
        if !vertical_with_modifier2.is_zero() {
            editor_event_writer
                .single_write(EditorEvents::CycleActiveLayer(vertical_with_modifier2 > 0.));
        }

        if input.action_single_press("save").is_down {
            editor_event_writer.single_write(EditorEvents::SaveLevelToFile);
        }

        if input.action_single_press("load").is_down {
            global_event_writer.single_write(Events::LoadLevel);
        }

        match *state {
            CursorState::EditMode => {
                // Controller input
                if input
                    .action_single_press("cancel")
                    .excluding_modifiers(EDITOR_MODIFIERS_ALL)
                    .is_down
                {
                    editor_event_writer.single_write(EditorEvents::RemoveGameObject);
                } else if input
                    .action_single_press("accept")
                    .excluding_modifiers(EDITOR_MODIFIERS_ALL)
                    .is_down
                {
                    editor_event_writer
                        .single_write(EditorEvents::ChangeState(CursorState::EditGameObject));
                } else if input
                    .action_single_press("insert")
                    .excluding_modifiers(EDITOR_MODIFIERS_ALL)
                    .is_down
                {
                    editor_event_writer
                        .single_write(EditorEvents::ChangeState(CursorState::InsertMode));
                } else if input
                    .action_single_press("start")
                    .excluding_modifiers(EDITOR_MODIFIERS_ALL)
                    .is_down
                {
                    global_event_writer.single_write(Events::OpenFilePickerUi);
                }
            }
            CursorState::InsertMode => {
                // Controller input
                if input
                    .action_single_press("cancel")
                    .excluding_modifiers(EDITOR_MODIFIERS_ALL)
                    .is_down
                {
                    editor_event_writer
                        .single_write(EditorEvents::ChangeState(CursorState::EditMode));
                } else if input
                    .action_single_press("accept")
                    .excluding_modifiers(EDITOR_MODIFIERS_ALL)
                    .is_down
                {
                    editor_event_writer.single_write(EditorEvents::AddGameObject);
                } else if input
                    .action_single_press("1")
                    .excluding_modifiers(EDITOR_MODIFIERS_ALL)
                    .is_down
                {
                    editor_event_writer.single_write(EditorEvents::ChangeInsertionGameObject(0));
                } else if input
                    .action_single_press("2")
                    .excluding_modifiers(EDITOR_MODIFIERS_ALL)
                    .is_down
                {
                    editor_event_writer.single_write(EditorEvents::ChangeInsertionGameObject(1));
                }
            }
            CursorState::EditGameObject => {
                if input
                    .action_single_press("cancel")
                    .excluding_modifiers(EDITOR_MODIFIERS_ALL)
                    .is_down
                {
                    editor_event_writer
                        .single_write(EditorEvents::ChangeState(CursorState::EditMode));
                } else if input
                    .action_single_press("accept")
                    .excluding_modifiers(EDITOR_MODIFIERS_ALL)
                    .is_down
                {
                    editor_event_writer.single_write(EditorEvents::AddGameObject);
                    editor_event_writer
                        .single_write(EditorEvents::ChangeState(CursorState::EditMode));
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
        Write<'s, CursorState>,
        Write<'s, TileLayer>,
        Write<'s, InsertionGameObject>,
        ReadStorage<'s, EditorCursor>,
        WriteStorage<'s, Position>,
        WriteStorage<'s, CursorWasInThisEntity>,
    );

    fn run(
        &mut self,
        (
            editor_event_channel,
            mut world_events_channel,
            mut cursor_state,
            mut active_layer,
            mut insertion_serialized_object,
            cursors,
            mut positions,
            previous_block,
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
                        CursorState::EditGameObject => {
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
                        CursorState::InsertMode => {
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
                        *cursor_state = new_state.clone();
                    }
                }
                EditorEvents::UiClick(_) => {}
                EditorEvents::CycleActiveLayer(forward) => {
                    let layer = *active_layer;
                    *active_layer = match forward {
                        true => layer.next(),
                        false => layer.prev(),
                    };
                    info!("{:?}", *active_layer);
                }
            };
        }
    }
}
