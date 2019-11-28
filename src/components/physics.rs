use crate::pizzatopia::{CAM_WIDTH, TILE_HEIGHT, TILE_WIDTH};
use crate::utils::Vec2;
use amethyst::{
    assets::{AssetStorage, Handle, Loader},
    core::transform::Transform,
    ecs::prelude::{Component, DenseVecStorage},
    prelude::*,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
};

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
    pub new_collider_vel: Vec2,
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
            new_collider_vel: Vec2::new(0.0, 0.0),
            correction: 0.0,
            distance: 0.0,
            side: CollisionSideOfBlock::Top,
        }
    }
}

pub struct Collidee {
    pub horizontal: Option<CollideeDetails>,
    pub vertical: Option<CollideeDetails>,
}

impl Collidee {
    pub fn new() -> Collidee {
        Collidee {
            horizontal: None,
            vertical: None,
        }
    }

    pub fn both(&self) -> bool {
        self.horizontal.is_some() && self.vertical.is_some()
    }
}

impl Component for Collidee {
    type Storage = DenseVecStorage<Self>;
}

pub struct Velocity(pub Vec2);

impl Component for Velocity {
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
