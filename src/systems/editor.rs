use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::{
    Join, NullStorage, Read, ReadStorage, System, SystemData, World, WriteStorage, Write,
};
use amethyst::input::{InputHandler, StringBindings};
use amethyst::renderer::debug_drawing::{DebugLines, DebugLinesComponent, DebugLinesParams};
use amethyst::renderer::palette::Srgba;

use crate::components::game::Health;
use crate::components::physics::{GravityDirection, Grounded, PlatformCuboid, Position, Velocity};
use crate::components::player::Player;
use crate::states::pizzatopia::{CAM_HEIGHT, TILE_HEIGHT, DEPTH_UI};
use crate::states::editor::{EDITOR_GRID_SIZE};
use crate::systems::physics::{
    gravitationally_adapted_velocity, gravitationally_de_adapted_velocity,
};
use crate::components::editor::{EditorCursor, RealCursorPosition};
use std::time::{Duration, Instant};

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
    );

    fn run(
        &mut self,
        (debug_lines_resource, input, mut positions, cursors, mut real_positions): Self::SystemData,
    ) {
        for (mut position, cursor, mut real_pos) in (
            &mut positions,
            &cursors,
            &mut real_positions,
        )
            .join()
        {
            // Controller input
            let v_move = input.axis_value("vertical_move");
            let h_move = input.axis_value("horizontal_move");

            let mut vertical = v_move.unwrap_or(0.0).round();
            let mut horizontal = h_move.unwrap_or(0.0).round();

            // Releasing the button lets us immediately press again to move
            if vertical == 0.0 {
                self.ready_v = true;
                self.repeat_delay_v = Instant::now();
            }
            if horizontal == 0.0 {
                self.ready_h = true;
                self.repeat_delay_h = Instant::now();
            }

            if horizontal == 0.0 && vertical == 0.0 {
                self.counter = 0;
            } else {
                self.counter += 1;
            }

            // Not ready or timer too short => don't move
            if !self.ready_v && (self.repeat_delay_v.elapsed().as_millis() < 250 || self.counter % 2 == 0) {
                vertical = 0.0;
            }
            if !self.ready_h && (self.repeat_delay_h.elapsed().as_millis() < 250 || self.counter % 2 == 0) {
                horizontal = 0.0;
            }

            // Set ready to false after you start moving
            if self.ready_v && vertical != 0.0 {
                self.ready_v = false;
            }
            if self.ready_h && horizontal != 0.0 {
                self.ready_h = false;
            }

            // This needs to move across the size of the block we select in the future
            real_pos.0.x += horizontal * EDITOR_GRID_SIZE;
            real_pos.0.y += vertical * EDITOR_GRID_SIZE;

            position.0.x = real_pos.0.x;
            position.0.y = real_pos.0.y;

            // This needs to snap to nearest block in future
            // Snap to nearest grid size
            position.0.x -= (position.0.x % EDITOR_GRID_SIZE) - EDITOR_GRID_SIZE / 2.0;
            position.0.y -= (position.0.y % EDITOR_GRID_SIZE) - EDITOR_GRID_SIZE / 2.0;

            // debug_lines_resource.draw_line(
            //     [real_pos.0.x - 16.0, real_pos.0.y + 16.0, DEPTH_UI].into(),
            //     [real_pos.0.x + 16.0, real_pos.0.y - 16.0, DEPTH_UI].into(),
            //     Srgba::new(0.1, 0.1, 0.4, 1.0),
            // );
            //
            // debug_lines_resource.draw_line(
            //     [real_pos.0.x + 16.0, real_pos.0.y + 16.0, DEPTH_UI].into(),
            //     [real_pos.0.x - 16.0, real_pos.0.y - 16.0, DEPTH_UI].into(),
            //     Srgba::new(0.1, 0.1, 0.4, 1.0),
            // );
        }
    }
}
