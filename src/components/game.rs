use crate::components::physics::{Position, Velocity};
use crate::level::Tile;
use amethyst::{
    assets::{AssetStorage, Handle, Loader},
    core::transform::Transform,
    ecs::prelude::{Component, DenseVecStorage, NullStorage},
    prelude::*,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
};
use derivative::Derivative;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum CollisionEvent {
    // Entity id and damage dealt
    EnemyCollision(u32, u8),
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Player(pub bool);
impl Component for Player {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Resettable;
impl Component for Resettable {
    type Storage = NullStorage<Self>;
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub enum GameObject {
    #[derivative(Default)]
    StaticTile(Tile),
    Player(Position, Player),
}

impl Component for GameObject {
    type Storage = DenseVecStorage<Self>;
}
