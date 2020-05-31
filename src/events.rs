use crate::components::game::{SerializedObject, SerializedObjectType};
use crate::utils::Vec2;

#[derive(Debug, Clone)]
pub enum Events {
    Warp(Vec2),
    Reset,
    AddGameObject,
    DeleteGameObject(u32),
    SaveLevel,
    ChangeInsertionGameObject(u8),
    SetInsertionGameObject(SerializedObject),
    EntityToInsertionGameObject(u32),
    OpenFilePickerUi,
    HoverGameObject,
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Revive(u8),
}
