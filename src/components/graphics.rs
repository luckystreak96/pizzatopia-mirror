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
use crate::utils::{Vec2, Vec3};
use derivative::Derivative;
use log::info;
use num_traits::identities::Zero;
use std::ops::Add;

#[derive(Derivative)]
#[derivative(Default)]
pub struct Lerper {
    pub target: Vec2,
    prev_velocity: Vec2,
    #[derivative(Default(value = "0.1"))]
    amount: f32,
    #[derivative(Default(value = "0.0"))]
    min_velocity: f32,
    #[derivative(Default(value = "60.0"))]
    max_velocity: f32,
    #[derivative(Default(value = "0.4"))]
    acceleration: f32,
    #[derivative(Default(value = "1.0"))]
    epsilon: f32,
}

impl Component for Lerper {
    type Storage = DenseVecStorage<Self>;
}

impl Lerper {
    fn epsilon(&self, first: f32, second: f32) -> bool {
        return (first - second).abs() < self.epsilon;
    }

    pub fn lerp(&mut self, pos: Vec2) -> Vec2 {
        // get the movement vector
        let mut movement_vector = Vec2::subtract(&self.target, &pos);
        if movement_vector.x.abs() <= self.epsilon {
            self.prev_velocity.x = 0.0;
            // result.x = self.target.x;
            movement_vector.x = 0.0;
        }
        if movement_vector.y.abs() <= self.epsilon {
            self.prev_velocity.y = 0.0;
            // result.y = self.target.y;
            movement_vector.y = 0.0;
        }

        // if it's zero just return
        if movement_vector.is_zero() {
            return self.target;
        }

        // store this value
        let mut new_velocity = movement_vector.clone();

        new_velocity.x = movement_vector.x / 20.0;
        new_velocity.y = movement_vector.y / 20.0;

        // Remember the previous velocity
        self.prev_velocity = new_velocity;

        // Adjust the pos based on the new velocity
        return Vec2::add(&pos, &new_velocity);
    }
}

pub struct AnimationCounter(pub u32);

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

#[repr(u8)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub enum SpriteSheetType {
    #[derivative(Default)]
    Tiles,
    Didi,
    Snap,
    Ui,
}

impl SpriteSheetType {
    pub fn next(&self) -> Self {
        let x = *self as u8;
        Self::from(x + 1)
    }

    pub fn prev(&self) -> Self {
        let x = *self as u8;
        match x.is_zero() {
            false => Self::from(x - 1),
            true => Self::from(x),
        }
    }
}

impl From<u8> for SpriteSheetType {
    fn from(x: u8) -> Self {
        match x {
            x if x == SpriteSheetType::Tiles as u8 => SpriteSheetType::Tiles,
            x if x == SpriteSheetType::Didi as u8 => SpriteSheetType::Didi,
            x if x == SpriteSheetType::Snap as u8 => SpriteSheetType::Snap,
            x if x == SpriteSheetType::Ui as u8 => SpriteSheetType::Ui,
            _ => SpriteSheetType::Ui,
        }
    }
}

impl Component for SpriteSheetType {
    type Storage = DenseVecStorage<Self>;
}
