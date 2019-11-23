use crate::pizzatopia::{CAM_WIDTH, TILE_HEIGHT, TILE_WIDTH};
use crate::utils::Vec2;
use amethyst::{
    assets::{AssetStorage, Handle, Loader},
    core::transform::Transform,
    ecs::prelude::{Component, DenseVecStorage},
    prelude::*,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
};

pub struct CollideeDetails {
    pub name: String,
    pub position: Vec2,
    pub half_size: Vec2,
    pub correction: f32,
}

pub struct Collidee {
    horizontal: Option<CollideeDetails>,
    vertical: Option<CollideeDetails>,
}

impl Component for Collidee {
    type Storage = DenseVecStorage<Self>;
}

pub struct Velocity(pub Vec2);

impl Component for Velocity {
    type Storage = DenseVecStorage<Self>;
}

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
        PlatformCollisionPoints(vec)
    }
}

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
