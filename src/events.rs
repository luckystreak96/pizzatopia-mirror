use crate::level::Tile;

#[derive(Debug, Clone)]
pub enum Events {
    Warp(i32, i32),
    Reset,
    AddTile(Tile),
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Revive(u8),
}
