use crate::level::Tile;
use crate::utils::Vec2;

#[derive(Debug, Clone)]
pub enum Events {
    Warp(i32, i32),
    Reset,
    AddGameObject(Vec2),
    DeleteGameObject(u32),
    SaveLevel,
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Revive(u8),
}
