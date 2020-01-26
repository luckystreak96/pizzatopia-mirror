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
use crate::systems::physics::CollisionDirection;

#[derive(Clone, Debug, PartialEq)]
pub enum CollisionSideOfBlock {
    Top,
    Bottom,
    Left,
    Right,
}

impl CollisionSideOfBlock {
    pub fn is_horizontal(&self) -> bool {
        match self {
            CollisionSideOfBlock::Left => true,
            CollisionSideOfBlock::Right => true,
            _ => false,
        }
    }

    pub fn is_vertical(&self) -> bool {
        match self {
            CollisionSideOfBlock::Top => true,
            CollisionSideOfBlock::Bottom => true,
            _ => false,
        }
    }
}

pub struct CollideeDetails {
    pub name: String,
    pub position: Vec2,
    pub half_size: Vec2,
    pub new_collider_pos: Vec2,
    pub old_collider_vel: Vec2,
    pub new_collider_vel: Vec2,
    pub num_points_of_collision: i32,
    pub correction: f32,
    pub distance: f32,
    pub side: CollisionSideOfBlock,
}
impl CollideeDetails {
    pub(crate) fn new() -> CollideeDetails {
        CollideeDetails {
            name: String::from(""),
            position: Vec2::new(0.0, 0.0),
            half_size: Vec2::new(0.0, 0.0),
            new_collider_pos: Vec2::new(0.0, 0.0),
            old_collider_vel: Vec2::new(0.0, 0.0),
            new_collider_vel: Vec2::new(0.0, 0.0),
            num_points_of_collision: 0,
            correction: 0.0,
            distance: 0.0,
            side: CollisionSideOfBlock::Top,
        }
    }
}

pub struct Collidee {
    pub horizontal: Option<CollideeDetails>,
    pub vertical: Option<CollideeDetails>,
    pub prev_horizontal: Option<CollideeDetails>,
    pub prev_vertical: Option<CollideeDetails>,
}

impl Collidee {
    pub fn new() -> Collidee {
        Collidee {
            horizontal: None,
            vertical: None,
            prev_horizontal: None,
            prev_vertical: None,
        }
    }

    pub fn both(&self) -> bool {
        self.horizontal.is_some() && self.vertical.is_some()
    }

    pub fn prev_collision_points(&self) -> i32 {
        let mut result = 0;
        if let Some(x) = &self.prev_horizontal {
            result += x.num_points_of_collision;
        }

        if let Some(x) = &self.prev_vertical {
            result += x.num_points_of_collision;
        }
        return result;
    }

    pub fn current_collision_points(&self) -> i32 {
        let mut result = 0;
        if let Some(x) = &self.horizontal {
            result += x.num_points_of_collision;
        }

        if let Some(x) = &self.vertical {
            result += x.num_points_of_collision;
        }
        return result;
    }
}

impl Component for Collidee {
    type Storage = DenseVecStorage<Self>;
}

pub struct Grounded(pub bool);

impl Component for Grounded {
    type Storage = DenseVecStorage<Self>;
}

pub struct Velocity(pub Vec2);

impl Component for Velocity {
    type Storage = DenseVecStorage<Self>;
}

pub struct Sticky(pub bool);

impl Component for Sticky{
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Copy, Clone)]
pub struct GravityDirection(pub CollisionDirection);

impl Component for GravityDirection{
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug)]
pub struct Position(pub Vec2);

impl Component for Position {
    type Storage = DenseVecStorage<Self>;
}

// The points represent offsets from Position
pub struct PlatformCollisionPoints(pub Vec<Vec2>);

impl Component for PlatformCollisionPoints {
    type Storage = DenseVecStorage<Self>;
}
impl PlatformCollisionPoints {
    pub fn vertical_line(half_height: f32) -> PlatformCollisionPoints {
        let mut vec = Vec::new();
        vec.push(Vec2::new(0.0, -half_height));
        vec.push(Vec2::new(0.0, half_height));
        vec.push(Vec2::new(0.0, 0.0));
        PlatformCollisionPoints(vec)
    }

    pub fn triangle(half_height: f32) -> PlatformCollisionPoints {
        let mut vec = Vec::new();
        vec.push(Vec2::new(-half_height, -half_height));
        vec.push(Vec2::new(0.0, half_height));
        vec.push(Vec2::new(half_height, -half_height));
        PlatformCollisionPoints(vec)
    }

    pub fn square(half_height: f32) -> PlatformCollisionPoints {
        let mut vec = Vec::new();
        vec.push(Vec2::new(-half_height, -half_height));
        vec.push(Vec2::new(-half_height, half_height));
        vec.push(Vec2::new(half_height, -half_height));
        vec.push(Vec2::new(half_height, half_height));
        PlatformCollisionPoints(vec)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PrefabData)]
#[prefab(Component)]
#[serde(deny_unknown_fields)]
pub struct PlatformCuboid {
    pub half_width: f32,
    pub half_height: f32,
}

impl PlatformCuboid {
    pub fn new() -> PlatformCuboid {
        PlatformCuboid {
            half_width: TILE_WIDTH / 2.0,
            half_height: TILE_HEIGHT / 2.0,
        }
    }

    pub fn create(size_x: f32, size_y: f32) -> PlatformCuboid {
        PlatformCuboid {
            half_width: CAM_WIDTH,
            half_height: TILE_HEIGHT,
        }
    }
}

impl Component for PlatformCuboid {
    type Storage = DenseVecStorage<Self>;
}
