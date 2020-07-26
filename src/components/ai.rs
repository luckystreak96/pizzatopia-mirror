use amethyst::{
    assets::{AssetStorage, Handle, Loader},
    core::transform::Transform,
    ecs::prelude::{Component, DenseVecStorage, NullStorage},
    prelude::*,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
};
use derivative::Derivative;

use crate::components::physics::Orientation;
use serde::{Deserialize, Serialize};

#[derive(Derivative, Copy, Clone)]
#[derivative(Default)]
pub struct BasicWalkAi {
    pub orientation: Orientation,
}
impl Component for BasicWalkAi {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct BasicShootAi {
    pub counter: f32,
}
impl Component for BasicShootAi {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct BasicAttackAi {
    pub counter: f32,
}
impl Component for BasicAttackAi {
    type Storage = DenseVecStorage<Self>;
}
