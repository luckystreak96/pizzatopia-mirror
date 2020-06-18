use crate::components::graphics::SpriteSheetType;
use crate::components::physics::{Position, Velocity};
use crate::states::editor::EDITOR_GRID_SIZE;
use crate::states::pizzatopia::{TILE_HEIGHT, TILE_WIDTH};
use crate::systems::editor::align_cursor_position_with_grid;
use crate::utils::{Vec2, Vec3};
use amethyst::{
    assets::{AssetStorage, Handle, Loader},
    core::transform::Transform,
    ecs::prelude::{Component, DenseVecStorage, NullStorage},
    prelude::*,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
};
use derivative::Derivative;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum CollisionEvent {
    // Entity id and damage dealt
    EnemyCollision(u32, u8),
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Player(pub bool);
impl Component for Player {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Tile;
impl Component for Tile {
    type Storage = NullStorage<Self>;
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Resettable;
impl Component for Resettable {
    type Storage = NullStorage<Self>;
}

#[derive(Default)]
pub struct Health(pub u8);
impl Component for Health {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default)]
pub struct Invincibility(pub f32);
impl Component for Invincibility {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub enum CameraTarget {
    #[derivative(Default)]
    Player,
    Cursor,
    GameObject(u32),
}

impl Component for CameraTarget {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub struct SpriteRenderData {
    pub(crate) sheet: SpriteSheetType,
    pub(crate) number: usize,
}

impl SpriteRenderData {
    pub fn new(sheet: SpriteSheetType, number: usize) -> Self {
        SpriteRenderData { sheet, number }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub struct SerializedObject {
    pub(crate) object_type: SerializedObjectType,
    #[derivative(Default(value = "Some(Vec2::default())"))]
    pub(crate) pos: Option<Vec2>,
    #[derivative(Default(value = "Some(Vec2::new(128.0, 128.0))"))]
    pub(crate) size: Option<Vec2>,
    #[derivative(Default(value = "Some(SpriteRenderData::default())"))]
    pub(crate) sprite: Option<SpriteRenderData>,
}

impl SerializedObject {
    pub fn next_size(&mut self, position: &mut Vec2, x_axis: bool) {
        let mut size = self.size.unwrap_or(Vec2::new(TILE_WIDTH, TILE_HEIGHT));
        let change = EDITOR_GRID_SIZE;
        match x_axis {
            true => size.x += change,
            false => size.y += change,
        }
        align_cursor_position_with_grid(position, &size);
        self.size = Some(size);
    }

    pub fn prev_size(&mut self, position: &mut Vec2, x_axis: bool) {
        let mut size = self.size.unwrap_or(Vec2::new(TILE_WIDTH, TILE_HEIGHT));
        let change = -EDITOR_GRID_SIZE;
        match x_axis {
            true => size.x += change,
            false => size.y += change,
        }
        if size.x > 0.0 && size.y > 0.0 {
            align_cursor_position_with_grid(position, &size);
            self.size = Some(size);
        }
    }
}

impl Component for SerializedObject {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub enum SerializedObjectType {
    #[derivative(Default)]
    StaticTile,
    Player {
        is_player: Player,
    },
}

impl Component for SerializedObjectType {
    type Storage = DenseVecStorage<Self>;
}
