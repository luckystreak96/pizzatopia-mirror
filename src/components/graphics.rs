use amethyst::{
    assets::{AssetStorage, Handle, Loader, PrefabData},
    assets::ProgressCounter,
    core::transform::Transform,
    derive::PrefabData,
    ecs::Entity,
    ecs::prelude::{Component, DenseVecStorage},
    ecs::WriteStorage,
    Error,
    prelude::*,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
};
use serde::{Deserialize, Serialize};

use crate::pizzatopia::{CAM_WIDTH, TILE_HEIGHT, TILE_WIDTH};
use crate::utils::Vec2;

pub struct AnimationCounter(pub u32);

impl Component for AnimationCounter {
    type Storage = DenseVecStorage<Self>;
}
