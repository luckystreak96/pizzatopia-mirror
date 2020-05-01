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

#[derive(Derivative)]
#[derivative(Default)]
pub struct Lerper {
    pub target: Vec2,
    prev_velocity: Vec2,
    #[derivative(Default(value = "0.2"))]
    amount: f32,
    #[derivative(Default(value = "0.0"))]
    min_velocity: f32,
    #[derivative(Default(value = "60.0"))]
    max_velocity: f32,
    #[derivative(Default(value = "0.3"))]
    acceleration: f32,
    #[derivative(Default(value = "0.5"))]
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
        let mut result = pos;
        let mut movement_vector = Vec2::subtract(&self.target, &pos);
        if movement_vector.x.abs() <= self.epsilon {
            info!("Target.X has been reached!");
            self.prev_velocity.x = 0.0;
            result.x = self.target.x;
            movement_vector.x = 0.0;
        }
        if movement_vector.y.abs() <= self.epsilon {
            info!("Target.Y has been reached!");
            self.prev_velocity.y = 0.0;
            result.y = self.target.y;
            movement_vector.y = 0.0;
        }

        // if it's zero just return
        if movement_vector.is_zero() {
            return result;
        }

        // store this value
        let mut new_velocity = movement_vector.clone();

        let weighted_acceleration = Vec2::new(
            movement_vector.x * self.amount,
            movement_vector.y * self.amount,
        );

        let normalized_vector = movement_vector.normalize();

        // Accelerate
        // The normalized_vector will make sure the direction is respected
        new_velocity.x = self.prev_velocity.x + self.acceleration * normalized_vector.x;
        new_velocity.y = self.prev_velocity.y + self.acceleration * normalized_vector.y;

        // Clamp velocity to weighted acceleration
        new_velocity.x = match new_velocity.x.is_sign_positive() {
            true => new_velocity.x.min(weighted_acceleration.x),
            false => new_velocity.x.max(weighted_acceleration.x),
        };
        new_velocity.y = match new_velocity.y.is_sign_positive() {
            true => new_velocity.y.min(weighted_acceleration.y),
            false => new_velocity.y.max(weighted_acceleration.y),
        };

        // Clamp to minimums and maxes
        new_velocity.x = match new_velocity.x.is_sign_positive() {
            true => new_velocity.x.clamp(self.min_velocity, self.max_velocity),
            false => new_velocity.x.clamp(-self.max_velocity, -self.min_velocity),
        };
        new_velocity.y = match new_velocity.y.is_sign_positive() {
            true => new_velocity.y.clamp(self.min_velocity, self.max_velocity),
            false => new_velocity.y.clamp(-self.max_velocity, -self.min_velocity),
        };

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
