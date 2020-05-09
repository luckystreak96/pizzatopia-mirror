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
    CursorWasInThisEntity, EditorCursor, EditorCursorState, EditorState, InsertionGameObject,
    InstanceEntityId, RealCursorPosition, SizeForEditorGrid,
};
use crate::components::game::{Health, SerializedObjectType};
use crate::components::game::{Player, SerializedObject};
use crate::components::graphics::Scale;
use crate::components::physics::{GravityDirection, Grounded, PlatformCuboid, Position, Velocity};
use crate::events::Events;
use crate::level::Level;
use crate::states::editor::EDITOR_GRID_SIZE;
use crate::states::pizzatopia::{CAM_HEIGHT, DEPTH_UI, TILE_HEIGHT, TILE_WIDTH};
use crate::systems::physics::{
    gravitationally_adapted_velocity, gravitationally_de_adapted_velocity, ActorCollisionSystem,
};
use crate::utils::{Vec2, Vec3};
use amethyst::assets::Handle;
use amethyst::ecs::prelude::ReadExpect;
use amethyst::prelude::WorldExt;
use amethyst::renderer::SpriteSheet;
use log::{error, info, warn};
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
}

fn snap_cursor_position_to_grid_center(position: &mut Vec2) {
    position.x -= (position.x % EDITOR_GRID_SIZE) - EDITOR_GRID_SIZE / 2.0;
    position.y -= (position.y % EDITOR_GRID_SIZE) - EDITOR_GRID_SIZE / 2.0;
}

fn snap_cursor_position_to_grid_corner(position: &mut Vec2) {
    position.x -= position.x % EDITOR_GRID_SIZE;
    position.y -= position.y % EDITOR_GRID_SIZE;
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
        if entities.is_alive(entity) {
            let half_w = size.0.x / 2.0;
            let half_h = size.0.y / 2.0;
            if (position.0.x - pos.x).abs() <= half_w && (position.0.y - pos.y).abs() <= half_h {
                // We are in contact with a block
                return Some((position.0.clone(), size.0.clone(), entity.id()));
            }
        }
    }
    return None;
}

#[derive(SystemDesc)]
pub struct CursorPositionSystem {
    repeat_delay_h: Instant,
    repeat_delay_v: Instant,
    ready_v: bool,
    ready_h: bool,
    counter: u32,
    reader: ReaderId<EditorEvents>,
}

impl CursorPositionSystem {
    pub fn new(world: &mut World) -> Self {
        <Self as System<'_>>::SystemData::setup(world);
        let reader = world
            .fetch_mut::<EventChannel<EditorEvents>>()
            .register_reader();
        Self {
            repeat_delay_h: Instant::now(),
            repeat_delay_v: Instant::now(),
            ready_h: true,
            ready_v: true,
            counter: 0,
            reader,
        }
    }
}

impl<'s> System<'s> for CursorPositionSystem {
    type SystemData = (
        ReadExpect<'s, EditorState>,
        ReadExpect<'s, InsertionGameObject>,
        Write<'s, DebugLines>,
        Read<'s, EventChannel<EditorEvents>>,
        Read<'s, InputHandler<StringBindings>>,
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
            insertion_serialized_object,
            mut debug_lines_resource,
            editor_events,
            input,
            mut positions,
            cursors,
            mut real_positions,
            size_for_editor,
            mut previous_block,
            entities,
        ): Self::SystemData,
    ) {
        // cursor moved
        let mut no_movement = true;
        let mut pos = Vec2::new(0.0, 0.0);
        let mut prev_block: Option<u32> = None;

        // Controller input
        // TODO : Create a resource to manage input repeat etc
        let v_move = input.axis_value("vertical_move");
        let h_move = input.axis_value("horizontal_move");
        let some_action = input.buttons_that_are_down().count() > 0
            || input.controller_buttons_that_are_down().count() > 0;

        let mut vertical = v_move.unwrap_or(0.0).round();
        let mut horizontal = h_move.unwrap_or(0.0).round();

        for (position, _cursor, mut real_pos, previous_block) in (
            &mut positions,
            &cursors,
            &mut real_positions,
            &previous_block,
        )
            .join()
        {
            prev_block = previous_block.0.clone();

            // Releasing the button lets us immediately press again to move
            if vertical == 0.0 {
                self.ready_v = true;
                self.repeat_delay_v = Instant::now();
            }
            if horizontal == 0.0 {
                self.ready_h = true;
                self.repeat_delay_h = Instant::now();
            }

            no_movement = horizontal == 0.0 && vertical == 0.0;
            if no_movement {
                self.counter = 0;
            } else {
                self.counter += 1;
            }

            // Not ready or timer too short => don't move
            if !self.ready_v
                && (self.repeat_delay_v.elapsed().as_millis() < 250 || self.counter % 2 == 0)
            {
                vertical = 0.0;
            }
            if !self.ready_h
                && (self.repeat_delay_h.elapsed().as_millis() < 250 || self.counter % 2 == 0)
            {
                horizontal = 0.0;
            }

            no_movement = horizontal == 0.0 && vertical == 0.0;

            // Set ready to false after you start moving
            if self.ready_v && vertical != 0.0 {
                self.ready_v = false;
            }
            if self.ready_h && horizontal != 0.0 {
                self.ready_h = false;
            }

            // Move the cursor one grid size
            real_pos.0.x += horizontal * EDITOR_GRID_SIZE;
            real_pos.0.y += vertical * EDITOR_GRID_SIZE;

            match *editor_state {
                EditorState::EditMode => {
                    snap_cursor_position_to_grid_center(&mut real_pos.0);
                }
                _ => {
                    position.0.x += horizontal * EDITOR_GRID_SIZE;
                    position.0.y += vertical * EDITOR_GRID_SIZE;
                }
            }

            pos = real_pos.0.clone();
        }

        let mut new_position = None;
        let mut new_prev = prev_block.clone();
        // look for a block on us if we moved
        let mut was_same = true;
        // Loop until you exit the block
        while was_same {
            was_same = false;
            new_prev = None;
            new_position = None;
            // if the block exists, snap to it
            let tile =
                get_tile_at_position(&pos, &positions, &size_for_editor, &cursors, &entities);

            if tile.is_some() {
                let (position, size, entity) = tile.unwrap();

                // Update the cursor characteristics
                new_position = Some(position.clone());

                let prev = prev_block.unwrap_or(9999999);
                if prev == entity && !no_movement {
                    // If we moved, we try to find the next block
                    if !no_movement {
                        // Move the cursor one more grid size
                        pos.x += horizontal * EDITOR_GRID_SIZE;
                        pos.y += vertical * EDITOR_GRID_SIZE;
                        was_same = true;
                    }
                } else {
                    new_prev = Some(entity);
                    was_same = false;
                }
            }
        }

        for (mut position, cursor, real_pos, mut previous) in (
            &mut positions,
            &cursors,
            &mut real_positions,
            &mut previous_block,
        )
            .join()
        {
            if *editor_state == EditorState::EditMode {
                real_pos.0.x = pos.x;
                real_pos.0.y = pos.y;

                if let Some(new_pos) = &new_position {
                    position.0.x = new_pos.x;
                    position.0.y = new_pos.y;
                } else if !no_movement {
                    position.0.x = real_pos.0.x;
                    position.0.y = real_pos.0.y;
                }
            }

            // Reset tile size if size is not default and cursor touch nothing
            if some_action && new_prev.is_none() && *editor_state == EditorState::EditMode {
                position.0.x = real_pos.0.x;
                position.0.y = real_pos.0.y;
            }

            previous.0 = new_prev.clone();

            // TODO : Draw debug lines in separate system
            debug_lines_resource.draw_line(
                [real_pos.0.x - 16.0, real_pos.0.y + 16.0, DEPTH_UI].into(),
                [real_pos.0.x + 16.0, real_pos.0.y - 16.0, DEPTH_UI].into(),
                Srgba::new(0.1, 0.1, 0.4, 1.0),
            );

            debug_lines_resource.draw_line(
                [real_pos.0.x + 16.0, real_pos.0.y + 16.0, DEPTH_UI].into(),
                [real_pos.0.x - 16.0, real_pos.0.y - 16.0, DEPTH_UI].into(),
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
        for (pos, size, cursor) in (&positions, &size_for_editor, &cursors).join() {
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
        if let Some((pressed, time)) = self.input_last_press.get(action) {
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
        Read<'s, InputHandler<StringBindings>>,
        Read<'s, EventChannel<EditorEvents>>,
        Write<'s, EventChannel<Events>>,
        Write<'s, EditorState>,
        Write<'s, InsertionGameObject>,
        ReadStorage<'s, EditorCursor>,
        WriteStorage<'s, Position>,
        ReadStorage<'s, RealCursorPosition>,
        ReadStorage<'s, InstanceEntityId>,
        WriteStorage<'s, CursorWasInThisEntity>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (
            input,
            editor_event_channel,
            mut world_events_channel,
            mut editor_state,
            mut insertion_serialized_object,
            cursors,
            mut positions,
            real_positions,
            real_entity_ids,
            previous_block,
            entities,
        ): Self::SystemData,
    ) {
        for event in editor_event_channel.read(&mut self.reader) {
            match event {
                // Writing an event here is fine - entities are created lazily (only at frame end)
                // May as well use World and save the trouble for the tile creation
                // https://book.amethyst.rs/master/concepts/system.html?highlight=create#creating-new-entities-in-a-system
                EditorEvents::AddGameObject => {
                    for (cursor, position, previous_block) in
                        (&cursors, &positions, &previous_block).join()
                    {
                        // We only add the GameObject if the cursor isn't currently in a tile
                        match cursor.state {
                            EditorCursorState::Normal => {
                                let pos = position.0.to_vec2();
                                insertion_serialized_object.0.pos = Some(pos);
                                world_events_channel.single_write(Events::AddGameObject);
                            }
                            EditorCursorState::Error => {}
                        }
                    }
                }
                EditorEvents::RemoveGameObject => {
                    for (cursor, previous_block) in (&cursors, &previous_block).join() {
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
                            for (cursor, previous_block) in (&cursors, &previous_block).join() {
                                // Change state if selecting block
                                if previous_block.0.is_none() {
                                    change = false;
                                } else {
                                    let id = previous_block.0.unwrap();
                                    let entity = entities.entity(id);
                                    world_events_channel
                                        .single_write(Events::EntityToInsertionGameObject(id));
                                    world_events_channel.single_write(Events::DeleteGameObject(id));
                                }
                            }
                        }
                        EditorState::InsertMode => {
                            for (position, cursor) in (&mut positions, &cursors).join() {
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
            };
        }
    }
}
