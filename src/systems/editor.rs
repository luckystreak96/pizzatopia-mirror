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
    CursorWasInThisEntity, EditorCursor, RealCursorPosition, RealEntityId, SizeForEditorGrid,
};
use crate::components::game::Health;
use crate::components::graphics::Scale;
use crate::components::physics::{GravityDirection, Grounded, PlatformCuboid, Position, Velocity};
use crate::components::player::Player;
use crate::level::Level;
use crate::states::editor::EDITOR_GRID_SIZE;
use crate::states::pizzatopia::{CAM_HEIGHT, DEPTH_UI, TILE_HEIGHT, TILE_WIDTH};
use crate::systems::physics::{
    gravitationally_adapted_velocity, gravitationally_de_adapted_velocity,
};
use crate::utils::Vec2;
use log::{info, warn};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum EditorEvents {
    A,
    B,
}

#[derive(SystemDesc)]
pub struct CursorPositionSystem {
    repeat_delay_h: Instant,
    repeat_delay_v: Instant,
    ready_v: bool,
    ready_h: bool,
    counter: u32,
}

impl Default for CursorPositionSystem {
    fn default() -> Self {
        CursorPositionSystem {
            repeat_delay_h: Instant::now(),
            repeat_delay_v: Instant::now(),
            ready_h: true,
            ready_v: true,
            counter: 0,
        }
    }
}

impl<'s> System<'s> for CursorPositionSystem {
    type SystemData = (
        Write<'s, DebugLines>,
        Read<'s, InputHandler<StringBindings>>,
        WriteStorage<'s, Position>,
        ReadStorage<'s, EditorCursor>,
        WriteStorage<'s, RealCursorPosition>,
        ReadStorage<'s, SizeForEditorGrid>,
        WriteStorage<'s, Scale>,
        WriteStorage<'s, CursorWasInThisEntity>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (
            mut debug_lines_resource,
            input,
            mut positions,
            cursors,
            mut real_positions,
            size_for_editor,
            mut scales,
            mut previous_block,
            entities,
        ): Self::SystemData,
    ) {
        let mut no_movement = true;
        let mut pos = Vec2::new(0.0, 0.0);
        let mut prev_block: Option<u32> = None;

        // Controller input
        let v_move = input.axis_value("vertical_move");
        let h_move = input.axis_value("horizontal_move");

        let mut vertical = v_move.unwrap_or(0.0).round();
        let mut horizontal = h_move.unwrap_or(0.0).round();

        for (position, cursor, mut real_pos, previous_block) in (
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

            // need to set again after possible changes
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

            // Snap to nearest grid size
            real_pos.0.x -= (real_pos.0.x % EDITOR_GRID_SIZE) - EDITOR_GRID_SIZE / 2.0;
            real_pos.0.y -= (real_pos.0.y % EDITOR_GRID_SIZE) - EDITOR_GRID_SIZE / 2.0;

            pos = real_pos.0.clone();
        }

        let mut new_scale = None;
        let mut new_position = None;
        let mut new_prev = prev_block.clone();
        // look for a block on us
        if !no_movement {
            let mut was_same = true;
            // Loop until you exit the block
            while was_same {
                was_same = false;
                new_prev = None;
                new_position = None;
                new_scale = None;
                // if the block exists, snap to it
                for (position, size, entity, _) in
                    (&positions, &size_for_editor, &entities, !&cursors).join()
                {
                    let half_w = size.0.x / 2.0;
                    let half_h = size.0.y / 2.0;
                    if (position.0.x - pos.x).abs() <= half_w
                        && (position.0.y - pos.y).abs() <= half_h
                    {
                        // We are in contact with a block
                        new_position = Some(position.0.clone());
                        new_scale = Some(Vec2::new(size.0.x / TILE_WIDTH, size.0.y / TILE_HEIGHT));

                        let prev = prev_block.unwrap_or(9999999);
                        if prev == entity.id() {
                            // Move the cursor one more grid size
                            pos.x += horizontal * EDITOR_GRID_SIZE;
                            pos.y += vertical * EDITOR_GRID_SIZE;
                            was_same = true;
                        } else {
                            new_prev = Some(entity.id());
                            was_same = false;
                        }
                    }
                }
            }
        }

        for (mut position, cursor, real_pos, mut scale, mut previous) in (
            &mut positions,
            &cursors,
            &mut real_positions,
            &mut scales,
            &mut previous_block,
        )
            .join()
        {
            real_pos.0.x = pos.x;
            real_pos.0.y = pos.y;

            if let Some(new_pos) = &new_position {
                position.0.x = new_pos.x;
                position.0.y = new_pos.y;
            } else if !no_movement {
                position.0.x = real_pos.0.x;
                position.0.y = real_pos.0.y;
            }

            if let Some(new_scl) = &new_scale {
                scale.0.x = new_scl.x;
                scale.0.y = new_scl.y;
            } else if !no_movement {
                scale.0.x = EDITOR_GRID_SIZE / TILE_WIDTH;
                scale.0.y = EDITOR_GRID_SIZE / TILE_WIDTH;
            }

            // Reset tile size if size is not default and cursor touch nothing
            if scale.0.x != EDITOR_GRID_SIZE / TILE_WIDTH && new_prev.is_none() {
                scale.0.x = EDITOR_GRID_SIZE / TILE_WIDTH;
                scale.0.y = EDITOR_GRID_SIZE / TILE_WIDTH;
                position.0.x = real_pos.0.x;
                position.0.y = real_pos.0.y;
            }

            previous.0 = new_prev.clone();

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
        }
    }
}

#[derive(SystemDesc)]
pub struct EditorButtonEventSystem;

impl<'s> System<'s> for EditorButtonEventSystem {
    type SystemData = (
        Read<'s, InputHandler<StringBindings>>,
        Write<'s, EventChannel<EditorEvents>>,
        ReadStorage<'s, EditorCursor>,
        ReadStorage<'s, RealEntityId>,
        WriteStorage<'s, CursorWasInThisEntity>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (input, mut editor_event_writer, cursors, real_entity_ids, mut previous_block, entities): Self::SystemData,
    ) {
        // Controller input
        if input.action_is_down("cancel").unwrap_or(false) {
            info!("One run of system");
            for (cursor, previous_block, cursor_entity) in
                (&cursors, &mut previous_block, &entities).join()
            {
                let mut target: Option<u32> = previous_block.0;
                let mut deleted = false;
                if target.is_some() {
                    deleted = true;
                    warn!("Deleting tile!");
                    // Get the editor entity
                    let editor_entity = entities.entity(target.unwrap());

                    // Delete the real entity using editor entity
                    let real_ent_id = real_entity_ids
                        .get(editor_entity)
                        .expect("Tried to delete editor entity with no real entity.");
                    if let Some(real_entity_id) = real_ent_id.0 {
                        let real_entity = entities.entity(real_entity_id);
                        entities.delete(real_entity);
                    }
                    entities.delete(editor_entity);
                }
                if deleted {
                    previous_block.0 = None;
                }
            }
        } else if input.action_is_down("accept").unwrap_or(false) {
            // Send event - you have no choice
            editor_event_writer.single_write(EditorEvents::A);
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
    type SystemData = (Read<'s, EventChannel<EditorEvents>>, Entities<'s>);

    fn run(&mut self, (editor_event_channel, entities): Self::SystemData) {
        for event in editor_event_channel.read(&mut self.reader) {
            println!("Received an event: {:?}", event);
        }
    }
}
