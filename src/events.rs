use crate::level::Tile;
use crate::utils::Vec2;

#[derive(Debug, Clone)]
pub enum Events {
    Warp(Vec2),
    Reset,
    AddGameObject(Vec2),
    DeleteGameObject(u32),
    SaveLevel,
    ChangeInsertionGameObject(u8),
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Revive(u8),
}
