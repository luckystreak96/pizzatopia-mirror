#[derive(Debug, Clone)]
pub enum Events {
    Warp(i32, i32),
    Reset,
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Revive(u8),
}
