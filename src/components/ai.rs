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

#[derive(Derivative, Debug, Copy, Clone, Serialize, Deserialize)]
#[derivative(Default)]
pub struct BasicWalkAi {
    pub going_right: bool,
}
impl Component for BasicWalkAi {
    type Storage = DenseVecStorage<Self>;
}
