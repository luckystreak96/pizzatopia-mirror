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

#[derive(SmartDefault)]
pub struct Collidee {
    horizontal: Option<CollideeDetails>,
    vertical: Option<CollideeDetails>,
}

impl Component for Collidee {
    type Storage = DenseVecStorage<Self>;
}

pub struct Velocity(f32, f32);

impl Component for Velocity {
    type Storage = DenseVecStorage<Self>;
}

pub struct Cuboid {
    pub width: f32,
    pub height: f32,
}

impl Cuboid {
    pub fn new() -> Cuboid {
        Cuboid {
            width: TILE_WIDTH,
            height: TILE_HEIGHT,
        }
    }

    pub fn create(size_x: f32, size_y: f32) -> Cuboid {
        Cuboid {
            width: CAM_WIDTH,
            height: TILE_HEIGHT,
        }
    }
}

impl Component for Cuboid {
    type Storage = DenseVecStorage<Self>;
}

#[cfg(test)]
mod tests {
    use crate::components::physics::Collidee;

    #[test]
    fn smart_default() {
        let collidee = Collidee {
            horizontal: None,
            vertical: None,
        };
        assert_eq!(
            Collidee::default().horizontal.is_none(),
            collidee.horizontal.is_none()
        );
    }
}
