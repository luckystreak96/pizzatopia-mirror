use crate::{
    components::game::{SerializedObject, Team},
    utils::Vec2,
};
use amethyst::ecs::prelude::Entity;

#[derive(Debug, Clone)]
pub enum Events {
    Warp(Vec2),
    Reset,
    AddGameObject,
    DeleteGameObject(u32),
    SaveLevel,
    LoadLevel,
    ChangeInsertionGameObject(u8),
    SetInsertionGameObject(SerializedObject),
    EntityToInsertionGameObject(u32),
    OpenFilePickerUi,
    HoverGameObject,
    // Pos, vel, team
    FireProjectile(Vec2, Vec2, Team),
    // Pos, size, team
    CreateDamageBox(Option<Entity>, Vec2, Vec2, Team),
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Revive(u32),
}
