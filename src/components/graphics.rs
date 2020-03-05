use amethyst::{
    assets::ProgressCounter,
    assets::{AssetStorage, Handle, Loader, PrefabData},
    core::transform::Transform,
    derive::PrefabData,
    ecs::prelude::{Component, DenseVecStorage},
    ecs::Entity,
    ecs::WriteStorage,
    prelude::*,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
    Error,
};
use serde::{Deserialize, Serialize};

use crate::states::pizzatopia::{CAM_WIDTH, TILE_HEIGHT, TILE_WIDTH};
use crate::utils::Vec3;

pub struct AnimationCounter(pub u32);

impl Component for AnimationCounter {
    type Storage = DenseVecStorage<Self>;
}

pub struct PulseAnimation {
    pub counter: u32,
    pub scale: Vec3,
}
 impl Default for PulseAnimation {
     fn default() -> Self {
         PulseAnimation {
             counter: 0,
             scale: Vec3::new(1.0, 1.0, 1.0),
         }
     }
 }

impl PulseAnimation {
    pub fn new(scale: Vec3) -> PulseAnimation {
        let mut pa = PulseAnimation::default();
        pa.scale = scale;
        pa
    }
}

impl Component for PulseAnimation {
    type Storage = DenseVecStorage<Self>;
}
