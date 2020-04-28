use crate::components::physics::{Position, Velocity};
use crate::components::player::Player;
use amethyst::{
    assets::{AssetStorage, Handle, Loader},
    core::transform::Transform,
    ecs::prelude::{Component, DenseVecStorage, NullStorage},
    prelude::*,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
};

#[derive(Debug)]
pub enum CollisionEvent {
    // Entity id and damage dealt
    EnemyCollision(u32, u8),
}

#[derive(Default)]
pub struct Health(pub u8);
impl Component for Health {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default)]
pub struct Invincibility(pub u32);
impl Component for Invincibility {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Clone, Copy)]
pub enum Resettable {
    StaticTile,
    Player(Position, Player),
}

impl Component for Resettable {
    type Storage = DenseVecStorage<Self>;
}
