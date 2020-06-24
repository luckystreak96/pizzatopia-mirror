use amethyst::{
    assets::ProgressCounter,
    assets::{AssetStorage, Handle, Loader, PrefabData},
    core::transform::Transform,
    derive::PrefabData,
    ecs::prelude::{Component, DenseVecStorage, NullStorage},
    ecs::Entity,
    ecs::WriteStorage,
    prelude::*,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
    Error,
};
use serde::{Deserialize, Serialize};

use crate::animations::AnimationId;
use crate::states::pizzatopia::{CAM_WIDTH, TILE_HEIGHT, TILE_WIDTH};
use crate::utils::{Vec2, Vec3};
use derivative::Derivative;
use log::{info, warn};
use num_traits::identities::Zero;
use std::ops::Add;
use std::sync::Arc;
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{EnumCount, EnumIter};

#[derive(Derivative)]
#[derivative(Default)]
pub struct Lerper {
    pub target: Vec2,
    prev_velocity: Vec2,
    #[derivative(Default(value = "0.1"))]
    pub amount: f32,
    #[derivative(Default(value = "0.0"))]
    min_velocity: f32,
    #[derivative(Default(value = "60.0"))]
    max_velocity: f32,
    #[derivative(Default(value = "0.4"))]
    acceleration: f32,
    #[derivative(Default(value = "1.0"))]
    pub epsilon: f32,
}

impl Component for Lerper {
    type Storage = DenseVecStorage<Self>;
}

impl Lerper {
    fn epsilon(&self, first: f32, second: f32) -> bool {
        return (first - second).abs() < self.epsilon;
    }

    pub fn linear_lerp(&mut self, current: Vec2, time_scale: f32) -> Vec2 {
        let mut movement_vector = Vec2::subtract(&self.target, &current);
        let mut result = Vec2::new(self.amount * time_scale, self.amount * time_scale);
        if movement_vector.x.abs() <= self.epsilon {
            self.prev_velocity.x = 0.0;
            movement_vector.x = 0.0;
            result.x = 0.;
        }
        if movement_vector.y.abs() <= self.epsilon {
            self.prev_velocity.y = 0.0;
            movement_vector.y = 0.0;
            result.y = 0.;
        }

        // if it's zero just return
        if movement_vector.is_zero() {
            return self.target;
        }
        if movement_vector.x.abs() < result.x {
            result.x = movement_vector.x.abs();
        }
        if movement_vector.y.abs() < result.y {
            result.y = movement_vector.y.abs();
        }

        if movement_vector.x.is_sign_negative() {
            result.x *= -1.0;
        }
        if movement_vector.y.is_sign_negative() {
            result.y *= -1.0;
        }

        return result.add(&current);
    }

    pub fn lerp(&mut self, pos: Vec2, time_scale: f32) -> Vec2 {
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

        new_velocity.x = movement_vector.x / (20.0 / time_scale);
        new_velocity.y = movement_vector.y / (20.0 / time_scale);

        // Remember the previous velocity
        self.prev_velocity = new_velocity;

        // Adjust the pos based on the new velocity
        return pos.add(&new_velocity);
    }
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
pub struct AbsolutePositioning;

impl Component for AbsolutePositioning {
    type Storage = NullStorage<Self>;
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Derivative, EnumIter, EnumCount)]
#[derivative(Default)]
pub enum SpriteSheetType {
    #[derivative(Default)]
    Tiles,
    Didi,
    Snap,
    Ui,
    Animation,
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
            x if x == SpriteSheetType::Animation as u8 => SpriteSheetType::Animation,
            _ => SpriteSheetType::Animation,
        }
    }
}

impl Component for SpriteSheetType {
    type Storage = DenseVecStorage<Self>;
}
