use amethyst::{
    assets::{AssetStorage, Handle, Loader, PrefabData, ProgressCounter},
    core::transform::Transform,
    derive::PrefabData,
    ecs::{
        prelude::{Component, DenseVecStorage, NullStorage},
        Entity, WriteStorage,
    },
    prelude::*,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
    Error,
};
use serde::{Deserialize, Serialize};

use crate::{
    animations::AnimationId,
    states::pizzatopia::{CAM_WIDTH, TILE_HEIGHT, TILE_WIDTH},
};
use derivative::Derivative;

use num_traits::identities::Zero;
use std::sync::Arc;

use pizzatopia_utils::*;
use ultraviolet::Vec2;

#[derive(Derivative)]
#[derivative(Default)]
pub struct Pan {
    #[derivative(Default)]
    pub destination: ultraviolet::Vec2,
    #[derivative(Default(value = "1.0"))]
    pub speed_factor: f32,
}
impl Component for Pan {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Clone)]
pub struct AnimationCounter {
    pub count_down: f32,
    pub animation_type: AnimationId,
    pub on_complete: Arc<dyn Fn(&mut World) + Send + Sync>,
}
impl AnimationCounter {
    pub fn new(
        time: f32,
        animation: AnimationId,
        on_complete: Arc<dyn Fn(&mut World) + Send + Sync>,
    ) -> AnimationCounter {
        AnimationCounter {
            count_down: time,
            animation_type: animation,
            on_complete,
        }
    }
}

impl Component for AnimationCounter {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default)]
pub struct CameraLimit {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Component for CameraLimit {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Clone)]
pub struct Scale(pub Vec2);

impl Component for Scale {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default)]
pub struct PulseAnimation(pub u32);

impl Component for PulseAnimation {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default)]
// sprite index / depth, number in the `seamless` chain
pub struct BackgroundParallax(pub(crate) u32, pub i32);

impl Component for BackgroundParallax {
    type Storage = DenseVecStorage<Self>;
}

#[enum_cycle]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub enum SpriteSheetType {
    #[derivative(Default)]
    Tiles,
    Didi,
    Snap,
    Ui,
    Animation,
    RollingHillsBg,
}

impl Component for SpriteSheetType {
    type Storage = DenseVecStorage<Self>;
}
